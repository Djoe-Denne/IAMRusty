// Test utilities from rustycog::testing
pub use rustycog::testing::TestFixture;
pub use rustycog::testing::*;

// Migration crate import - use the correct crate name
use iammigration::{Migrator, MigratorTrait};

// IAM imports
use async_trait::async_trait;
use iam_configuration::{load_config_fresh, AppConfig, SecretStorage, SecurityMode, ServerConfig};
use iam_domain::entity::signing_key::{
    SigningKey, SigningKeyStatus, SigningProviderType, TrustScope,
};
use iam_domain::error::DomainError;
use iam_http_server::SERVICE_PREFIX;
use iam_infra::event_adapter::IAMErrorMapper;
use iam_setup::app::build_and_run;
use iam_setup::app::{build_app_state_with_event_publisher, IAMRustyApp};
use readiness::ComponentStatus;
use reqwest::Client;
use rustycog::events::adapter::{GenericEventPublisherAdapter, MultiQueueEventPublisher};
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::{Arc, OnceLock};
use tokio::sync::watch;

// Exact owned listener handles are retained for the parent test lease. Never stop
// containers/SDK singleton servers here. E must join cleanup before fixture release.
struct IamTestListenerLease {
    fixture_db: Arc<sea_orm::DatabaseConnection>,
    config: AppConfig,
    app: Arc<IAMRustyApp>,
    shutdown: watch::Sender<bool>,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
    pem_files: Option<Arc<tempfile::TempDir>>,
}

static IAM_TEST_LISTENERS: OnceLock<Mutex<Vec<IamTestListenerLease>>> = OnceLock::new();

struct AbortWorkersOnDrop(Vec<tokio::task::AbortHandle>);

impl Drop for AbortWorkersOnDrop {
    fn drop(&mut self) {
        for worker in &self.0 {
            worker.abort();
        }
    }
}

fn listener_leases() -> &'static Mutex<Vec<IamTestListenerLease>> {
    IAM_TEST_LISTENERS.get_or_init(|| Mutex::new(Vec::new()))
}

fn fixture_writer(fixture: &TestFixture) -> anyhow::Result<Arc<sea_orm::DatabaseConnection>> {
    fixture
        .database
        .as_ref()
        .map(TestDatabase::get_connection)
        .ok_or_else(|| anyhow::anyhow!("IAM fixture requires writer DB"))
}

/// Compatibility seam for config-only access-token helpers. A config cannot
/// authorize an RSA signer: resolve exactly one live, owned composition root.
///
/// # Errors
/// Rejects missing or ambiguous owned roots instead of inventing a signing epoch.
pub fn fixture_jwt_codec_from_config(
    config: &iam_configuration::JwtConfig,
) -> anyhow::Result<Arc<iam_infra::token::JwtTokenService>> {
    let leases = listener_leases()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut matching = leases.iter().filter(|lease| {
        !lease.task.is_finished()
            && lease.config.jwt.platform_issuer() == config.platform_issuer()
            && lease.config.jwt.audience == config.audience
    });
    let lease = matching
        .next()
        .ok_or_else(|| anyhow::anyhow!("RSA token helper requires a live owned IAM app"))?;
    anyhow::ensure!(
        matching.next().is_none(),
        "ambiguous owned IAM signer; pass fixture codec explicitly"
    );
    let codec = lease.app.jwt_codec();
    drop(leases);
    Ok(codec)
}

/// Exact live root codec and effective issuer, not another registry/encoder.
///
/// # Errors
///
/// Returns an error if the fixture has no writer database or no live owned IAM app.
pub fn fixture_jwt_codec(
    fixture: &TestFixture,
) -> Result<(Arc<iam_infra::token::JwtTokenService>, String), Box<dyn std::error::Error>> {
    let db = fixture_writer(fixture)?;
    let found = {
        let leases = listener_leases()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        leases
            .iter()
            .find(|lease| Arc::ptr_eq(&db, &lease.fixture_db) && !lease.task.is_finished())
            .map(|lease| (lease.app.jwt_codec(), lease.config.jwt.platform_issuer()))
    };
    found.ok_or_else(|| anyhow::anyhow!("fixture has no live owned IAM app").into())
}

/// Exact live signing-key registry and current epoch from the owned composition root.
///
/// # Errors
///
/// Returns an error if the fixture has no writer database or no live owned IAM app.
pub fn fixture_signing_registry(
    fixture: &TestFixture,
) -> Result<(Arc<iam_infra::repository::SeaOrmSigningKeyRegistry>, u64), Box<dyn std::error::Error>>
{
    let db = fixture_writer(fixture)?;
    let found = {
        let leases = listener_leases()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        leases
            .iter()
            .find(|lease| Arc::ptr_eq(&db, &lease.fixture_db) && !lease.task.is_finished())
            .map(|lease| lease.app.signing_registry())
    };
    found.ok_or_else(|| anyhow::anyhow!("fixture has no live owned IAM app").into())
}

/// Public registration/completion creates the account and persisted session using
/// the same root codec. Email verification is explicit fixture arrangement only.
///
/// # Errors
///
/// Returns an error if registration, session persist, or token issuance fails.
pub async fn fixture_platform_access_token(
    fixture: &TestFixture,
) -> Result<String, Box<dyn std::error::Error>> {
    fixture_platform_access_token_impl(fixture)
        .await
        .map_err(Into::into)
}

async fn fixture_platform_access_token_impl(fixture: &TestFixture) -> anyhow::Result<String> {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    let db = fixture_writer(fixture)?;
    let config = {
        let leases = listener_leases()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        leases
            .iter()
            .find(|lease| Arc::ptr_eq(&db, &lease.fixture_db) && !lease.task.is_finished())
            .ok_or_else(|| anyhow::anyhow!("fixture has no live owned IAM app"))?
            .config
            .clone()
    };
    anyhow::ensure!(
        config.jwt.uses_rsa(),
        "platform admission control requires the actual RSA root"
    );
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let base = format!("http://127.0.0.1:{}{SERVICE_PREFIX}", config.server.port);
    let id = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("admission-{id}@example.com");
    let signup = client
        .post(format!("{base}/api/auth/signup"))
        .json(&serde_json::json!({"email": email, "password": "AdmissionTest1a!"}))
        .send()
        .await?;
    anyhow::ensure!(
        signup.status() == reqwest::StatusCode::ACCEPTED,
        "platform fixture signup failed"
    );
    let signup: serde_json::Value = signup.json().await?;
    let verified = db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE user_emails SET is_verified=true WHERE email=$1",
            [email.into()],
        ))
        .await?;
    anyhow::ensure!(
        verified.rows_affected() == 1,
        "platform fixture email arrangement failed"
    );
    let complete = client.post(format!("{base}/api/auth/complete-registration"))
        .json(&serde_json::json!({"registration_token": signup["registration_token"], "username": format!("a{}", &id[..15])}))
        .send().await?;
    anyhow::ensure!(
        complete.status().is_success(),
        "platform fixture completion failed"
    );
    let complete: serde_json::Value = complete.json().await?;
    let access = complete["access_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("platform fixture has no access token"))?
        .to_owned();
    let refresh = complete["refresh_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("platform fixture has no session"))?;
    let hash = iam_domain::entity::token::RefreshToken::hash_token(refresh);
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS count FROM refresh_tokens WHERE token=$1 AND is_valid AND expires_at>NOW()", [hash.into()]))
        .await?.ok_or_else(|| anyhow::anyhow!("platform session count missing"))?;
    anyhow::ensure!(
        row.try_get::<i64>("", "count")? == 1,
        "platform session was not persisted"
    );
    let me = client
        .get(format!("{base}/api/me"))
        .bearer_auth(&access)
        .send()
        .await?;
    anyhow::ensure!(
        me.status().is_success(),
        "actual platform account guard rejected fixture"
    );
    Ok(access)
}

async fn prepare_primary_fixture() -> anyhow::Result<()> {
    // Reap only completed handles from a previous Tokio test runtime. Never reset
    // migrations underneath another live app/lease; replicas don't call this.
    let finished = {
        let mut all = listener_leases()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (finished, live): (Vec<_>, Vec<_>) =
            all.drain(..).partition(|lease| lease.task.is_finished());
        *all = live;
        finished
    };
    for lease in finished {
        let _ = lease.task.await;
    }
    if !listener_leases()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_empty()
    {
        return Err(anyhow::anyhow!(
            "join cleanup_test_servers before creating another primary fixture"
        ));
    }
    Ok(())
}

fn fixture_config(fixture: &TestFixture) -> anyhow::Result<AppConfig> {
    fixture_config_with_security(fixture, None)
}

fn fixture_config_with_security(
    fixture: &TestFixture,
    security: Option<iam_configuration::security::SecurityConfig>,
) -> anyhow::Result<AppConfig> {
    let mut config = load_config_fresh::<AppConfig>()?;
    let explicit_security = security.is_some();
    if let Some(security) = security {
        config.security = security;
    }
    let database = fixture
        .database
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("IAM fixture requires writer DB"))?;
    let database_url = url::Url::parse(&database.database_url)?;
    let host = database_url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("missing fixture DB host"))?;
    config.database.host.clear();
    config.database.host.push_str(host);
    config.database.port = database_url
        .port()
        .ok_or_else(|| anyhow::anyhow!("missing fixture DB port"))?;
    let db_name = database_url.path().trim_start_matches('/');
    config.database.db.clear();
    config.database.db.push_str(db_name);
    config.database.creds.username.clear();
    config
        .database
        .creds
        .username
        .push_str(database_url.username());
    let password = database_url
        .password()
        .ok_or_else(|| anyhow::anyhow!("missing fixture DB credential"))?;
    config.database.creds.password.clear();
    config.database.creds.password.push_str(password);
    config.database.read_replicas.clear();
    if !explicit_security && config.security.mode != SecurityMode::IsolatedTest {
        return Err(anyhow::anyhow!(
            "IAM test listeners require explicit isolated_test configuration"
        ));
    }
    // The test HMAC verifier must recognize the same platform issuer as the
    // account guard and core issuer; a legacy label is not a second realm.
    config.jwt.issuer = config.jwt.platform_issuer();
    // No secret fallback: a missing/empty configured state key still fails boot.
    config
        .security
        .validate(&config.jwt.oauth_state_secret, &config.idp)
        .map_err(anyhow::Error::msg)?;
    config.server.host = "127.0.0.1".into();
    config.server.port = 0;
    config.server.tls_enabled = false;
    Ok(config)
}

fn mock_publisher(mock: Arc<MockEventPublisher>) -> Arc<MultiQueueEventPublisher<DomainError>> {
    Arc::new(MultiQueueEventPublisher::new(
        vec![GenericEventPublisherAdapter::new(
            Arc::new(rustycog::events::ConcreteEventPublisher::NoOp(mock)),
            Arc::new(IAMErrorMapper),
        )],
        HashSet::new(),
    ))
}

async fn start_owned_listener(
    fixture: &TestFixture,
    mut config: AppConfig,
    publisher: Arc<MultiQueueEventPublisher<DomainError>>,
    pem_files: Option<Arc<tempfile::TempDir>>,
) -> anyhow::Result<(String, Client)> {
    let pem_files = if let Some(files) = pem_files {
        Some(files)
    } else {
        // Public, deliberately non-secret/nonproduction RSA fixture. The
        // SDK constants and IAM config/keys/test-platform.* are the SAME
        // pair; this is not evidence of distinct keys or rotation. Real
        // provider parsing/probing still runs on each isolated app boot.
        use rustycog::testing::http::jwt::{test_rs256_private_pem, test_rs256_public_pem};
        let files = Arc::new(tempfile::TempDir::new()?);
        let private = files.path().join("private.pem");
        let public = files.path().join("public.pem");
        std::fs::write(&private, test_rs256_private_pem())?;
        std::fs::write(&public, test_rs256_public_pem())?;
        config.jwt.secret = SecretStorage::PemFile {
            private_key_path: private.to_string_lossy().into_owned(),
            public_key_path: public.to_string_lossy().into_owned(),
            key_id: Some(iam_domain::entity::signing_key::opaque_kid()),
        };
        config.jwt.allowed_algorithms = vec!["RS256".into()];
        config.jwt.backend = None;
        config.jwt.provider = None;
        config.jwt.remote = None;
        Some(files)
    };
    Box::pin(start_owned_listener_with_keys(
        fixture,
        config,
        publisher,
        pem_files,
        &[],
    ))
    .await
}

async fn start_owned_listener_with_keys(
    fixture: &TestFixture,
    mut config: AppConfig,
    publisher: Arc<MultiQueueEventPublisher<DomainError>>,
    pem_files: Option<Arc<tempfile::TempDir>>,
    signing_keys: &[SigningKey],
) -> anyhow::Result<(String, Client)> {
    let fixture_db = fixture_writer(fixture)?;
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    config.server.port = address.port();
    if config.jwt.uses_rsa() {
        config.jwt.jwks_url = Some(format!(
            "http://{address}{SERVICE_PREFIX}/.well-known/jwks.json"
        ));
        config.jwt.allowed_algorithms = vec!["RS256".into()];
        config.auth.jwt.hs256_secret = None;
        config.auth.jwt.allowed_algorithms = vec!["RS256".into()];
    }
    let app = Arc::new(
        Box::pin(
            iam_setup::app::build_app_state_with_event_publisher_and_signing_keys(
                config.clone(),
                publisher,
                ComponentStatus::Injected,
                None,
                signing_keys,
            ),
        )
        .await?,
    );
    let router = axum::Router::new().nest(SERVICE_PREFIX, app.router());
    let (shutdown, mut receiver) = watch::channel(false);
    let server_app = app.clone();
    let stop_server = shutdown.clone();
    let task = tokio::spawn(async move {
        let mut workers = server_app.start_background_tasks();
        let _abort_workers = AbortWorkersOnDrop(
            workers
                .iter()
                .map(tokio::task::JoinHandle::abort_handle)
                .collect(),
        );
        let server = axum::serve(
            listener,
            router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            let already_shutdown = *receiver.borrow();
            if !already_shutdown {
                let _ = receiver.changed().await;
            }
        });
        let server = std::future::IntoFuture::into_future(server);
        tokio::pin!(server);
        let (mut result, server_finished) = tokio::select! {
            result = &mut server => (result.map_err(anyhow::Error::from), true),
            result = iam_setup::app::wait_for_background_failure(&mut workers) => (result, false),
        };
        let _ = stop_server.send(true);
        let cleanup = server_app.shutdown_background_tasks(&mut workers).await;
        if result.is_ok() {
            result = cleanup;
        }
        if !server_finished {
            let drained =
                tokio::time::timeout(std::time::Duration::from_secs(5), &mut server).await;
            let server_result = match drained {
                Ok(result) => result.map_err(anyhow::Error::from),
                Err(_) => Err(anyhow::anyhow!("IAM fixture HTTP drain timed out")),
            };
            if result.is_ok() {
                result = server_result;
            }
        }
        result
    });
    listener_leases()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(IamTestListenerLease {
            fixture_db,
            config,
            app,
            shutdown,
            task,
            pem_files,
        });
    let base = format!("http://{address}{SERVICE_PREFIX}");
    // Serving is not an auth proof; a bounded readiness wait only establishes the listener.
    if iam_listener_ready(&client, &base).await {
        return Ok((base, client));
    }
    cleanup_test_servers(fixture).await?;
    Err(anyhow::anyhow!(
        "IAM fixture listener failed to become available"
    ))
}

async fn iam_listener_ready(client: &Client, base: &str) -> bool {
    for _ in 0..100 {
        if client
            .get(format!("{base}/ready"))
            .timeout(std::time::Duration::from_millis(200))
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    false
}

/// Stop and join only listeners owned by this fixture's parent lease, not containers.
///
/// # Errors
///
/// Returns the first listener shutdown or join error, if any.
pub async fn cleanup_test_servers(fixture: &TestFixture) -> anyhow::Result<()> {
    let db = fixture_writer(fixture)?;
    let leases = {
        let mut all = listener_leases()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (owned, other): (Vec<_>, Vec<_>) = all
            .drain(..)
            .partition(|lease| Arc::ptr_eq(&db, &lease.fixture_db));
        *all = other;
        owned
    };
    for lease in &leases {
        let _ = lease.shutdown.send(true);
    }
    let mut first_error = None;
    for mut lease in leases {
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(15), &mut lease.task).await;
        let error = match result {
            Ok(Ok(Ok(()))) => None,
            Ok(Ok(Err(error))) => Some(error),
            Ok(Err(error)) => Some(anyhow::Error::from(error)),
            Err(_) => {
                lease.task.abort();
                let _ = lease.task.await;
                Some(anyhow::anyhow!("IAM fixture listener cleanup timeout"))
            }
        };
        if first_error.is_none() {
            first_error = error;
        }
    }
    first_error.map_or(Ok(()), Err)
}

/// A real second app/listener: same writer storage/config, new per-instance budgets.
///
/// # Errors
///
/// Returns an error if no live primary listener exists or the replica fails to start.
pub async fn setup_test_replica(
    fixture: &TestFixture,
) -> Result<(String, Client), Box<dyn std::error::Error>> {
    let db = fixture_writer(fixture)?;
    let (config, pem_files) = {
        let leases = listener_leases()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cloned = {
            let primary = leases
                .iter()
                .find(|lease| Arc::ptr_eq(&db, &lease.fixture_db) && !lease.task.is_finished())
                .ok_or_else(|| {
                    anyhow::anyhow!("replica requires a live fixture-owned primary listener")
                })?;
            (primary.config.clone(), primary.pem_files.clone())
        };
        drop(leases);
        cloned
    };
    Ok(Box::pin(start_owned_listener(
        fixture,
        config,
        mock_publisher(Arc::new(MockEventPublisher::new())),
        pem_files,
    ))
    .await?)
}

/// Start a primary IAM test listener with GitHub PKCE enabled.
///
/// # Errors
///
/// Returns an error if the fixture, GitHub connector, or HTTP listener fails to start.
pub async fn setup_test_server_with_pkce(
) -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>> {
    prepare_primary_fixture().await?;
    let fixture = TestFixture::new(Arc::new(IAMRustyTestDescriptor)).await?;
    let mut config = fixture_config(&fixture)?;
    let github = config
        .idp
        .connectors
        .iter_mut()
        .find(|entry| entry.id == "github")
        .ok_or_else(|| anyhow::anyhow!("PKCE fixture requires configured GitHub connector"))?;
    github.pkce_supported = true;
    let (base, client) = Box::pin(start_owned_listener(
        &fixture,
        config,
        mock_publisher(Arc::new(MockEventPublisher::new())),
        None,
    ))
    .await?;
    Ok((fixture, base, client))
}

/// Build the actual root on existing writer storage; no migrations, listeners or tasks.
///
/// # Errors
///
/// Returns an error if configuration or composition-root construction fails.
pub async fn build_test_iam_app(
    fixture: &TestFixture,
    security: iam_configuration::security::SecurityConfig,
) -> Result<IAMRustyApp, Box<dyn std::error::Error>> {
    let mut config = fixture_config_with_security(fixture, Some(security))?;
    // A second root on the same writer must retain its registered signing
    // binding, not re-bootstrap from the unrelated on-disk default key.
    let db = fixture_writer(fixture)?;
    {
        let leases = listener_leases()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(primary) = leases
            .iter()
            .find(|lease| Arc::ptr_eq(&db, &lease.fixture_db) && !lease.task.is_finished())
        {
            config.jwt = primary.config.jwt.clone();
        }
    }
    Ok(Box::pin(build_app_state_with_event_publisher(
        config,
        mock_publisher(Arc::new(MockEventPublisher::new())),
        ComponentStatus::Injected,
        None,
    ))
    .await?)
}

/// Start a primary IAM test listener with an explicit enabled rate-limit policy.
///
/// # Errors
///
/// Returns an error if the policy is disabled or the fixture or listener fails to start.
pub async fn setup_test_server_with_rate_limits(
    limits: iam_configuration::security::AuthRateLimitConfig,
) -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>> {
    prepare_primary_fixture().await?;
    if limits.disabled {
        return Err(anyhow::anyhow!("rate-limit fixture requires enabled policy").into());
    }
    let fixture = TestFixture::new(Arc::new(IAMRustyTestDescriptor)).await?;
    let mut config = fixture_config(&fixture)?;
    config.security.rate_limit = limits;
    let (base, client) = Box::pin(start_owned_listener(
        &fixture,
        config,
        mock_publisher(Arc::new(MockEventPublisher::new())),
        None,
    ))
    .await?;
    Ok((fixture, base, client))
}

/// Start a primary IAM test listener bound to the provided active platform signing keys.
///
/// # Errors
///
/// Returns an error if the platform key does not match the PEM bootstrap binding,
/// or if the fixture or listener fails to start.
pub async fn setup_test_server_with_signing_keys(
    keys: &[SigningKey],
) -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>> {
    use rustycog::testing::http::jwt::{test_rs256_private_pem, test_rs256_public_pem};
    prepare_primary_fixture().await?;
    let platform = keys
        .iter()
        .find(|key| {
            key.trust_scope == TrustScope::Platform && key.status == SigningKeyStatus::Active
        })
        .ok_or_else(|| anyhow::anyhow!("strict fixture requires an active platform key"))?;
    if platform.public_key != test_rs256_public_pem()
        || platform.provider_type != SigningProviderType::PemFile
        || platform.provider_key_ref != "pem:config/jwt.secret"
        || platform.credential_ref.is_some()
    {
        return Err(anyhow::anyhow!(
            "platform fixture must match the configured PEM bootstrap binding"
        )
        .into());
    }
    let origin = platform
        .issuer
        .strip_suffix("/iam")
        .ok_or_else(|| anyhow::anyhow!("platform issuer must match configured /iam derivation"))?;
    let fixture = TestFixture::new(Arc::new(IAMRustyTestDescriptor)).await?;
    let mut config = fixture_config(&fixture)?;
    config.jwt.public_base_url = origin.to_owned();
    config.jwt.issuer = platform.issuer.clone();
    let pem_files = Arc::new(tempfile::TempDir::new()?);
    let private = pem_files.path().join("private.pem");
    let public = pem_files.path().join("public.pem");
    std::fs::write(&private, test_rs256_private_pem())?;
    std::fs::write(&public, test_rs256_public_pem())?;
    config.jwt.secret = SecretStorage::PemFile {
        private_key_path: private.to_string_lossy().into_owned(),
        public_key_path: public.to_string_lossy().into_owned(),
        key_id: Some(platform.kid.clone()),
    };
    config.jwt.allowed_algorithms = vec!["RS256".into()];
    config.jwt.backend = None;
    config.jwt.provider = None;
    config.jwt.remote = None;
    let (base, client) = Box::pin(start_owned_listener_with_keys(
        &fixture,
        config,
        mock_publisher(Arc::new(MockEventPublisher::new())),
        Some(pem_files),
        keys,
    ))
    .await?;
    Ok((fixture, base, client))
}

pub struct IAMRustyTestDescriptor;

#[derive(Clone)]
pub struct IAMRustyTestDescriptorWithMockEvents {
    mock_event_publisher: Arc<MockEventPublisher>,
}

#[async_trait]
impl ServiceTestDescriptor<TestFixture> for IAMRustyTestDescriptor {
    type Config = AppConfig;

    async fn build_app(
        &self,
        _config: AppConfig,
        _server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn run_app(&self, config: AppConfig, server_config: ServerConfig) -> anyhow::Result<()> {
        Box::pin(build_and_run(config, server_config, None)).await
    }

    async fn run_migrations_up(
        &self,
        connection: &sea_orm::DatabaseConnection,
    ) -> anyhow::Result<()> {
        Migrator::up(connection, None).await?;
        Ok(())
    }

    async fn run_migrations_down(
        &self,
        connection: &sea_orm::DatabaseConnection,
    ) -> anyhow::Result<()> {
        Migrator::down(connection, None).await?;
        Ok(())
    }

    fn has_db(&self) -> bool {
        true
    }

    fn has_sqs(&self) -> bool {
        false
    }

    fn has_openfga(&self) -> bool {
        false
    }
}

/// Start the IAM test server with the default descriptor.
///
/// # Errors
///
/// Returns an error if the test fixture or HTTP server fails to start.
pub async fn setup_test_server() -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>>
{
    prepare_primary_fixture().await?;
    let fixture = TestFixture::new(Arc::new(IAMRustyTestDescriptor)).await?;
    let config = fixture_config(&fixture)?;
    let (base, client) = Box::pin(start_owned_listener(
        &fixture,
        config,
        mock_publisher(Arc::new(MockEventPublisher::new())),
        None,
    ))
    .await?;
    Ok((fixture, base, client))
}

impl Default for IAMRustyTestDescriptorWithMockEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl IAMRustyTestDescriptorWithMockEvents {
    #[must_use]
    pub fn new() -> Self {
        Self {
            mock_event_publisher: Arc::new(MockEventPublisher::new()),
        }
    }
}

#[async_trait]
impl ServiceTestDescriptor<TestFixture> for IAMRustyTestDescriptorWithMockEvents {
    type Config = AppConfig;

    async fn build_app(
        &self,
        _config: AppConfig,
        _server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn run_app(&self, _config: AppConfig, server_config: ServerConfig) -> anyhow::Result<()> {
        let no_op_event_publisher = Arc::new(rustycog::events::ConcreteEventPublisher::NoOp(
            self.mock_event_publisher.clone(),
        ));
        let error_mapper = Arc::new(IAMErrorMapper);
        let multi_queue_event_publisher = MultiQueueEventPublisher::new(
            vec![GenericEventPublisherAdapter::<DomainError>::new(
                no_op_event_publisher,
                error_mapper,
            )],
            HashSet::new(),
        );
        Box::pin(build_and_run(
            _config,
            server_config,
            Some(Arc::new(multi_queue_event_publisher)),
        ))
        .await?;
        Ok(())
    }

    async fn run_migrations_up(
        &self,
        connection: &sea_orm::DatabaseConnection,
    ) -> anyhow::Result<()> {
        Migrator::up(connection, None).await?;
        Ok(())
    }

    async fn run_migrations_down(
        &self,
        connection: &sea_orm::DatabaseConnection,
    ) -> anyhow::Result<()> {
        Migrator::down(connection, None).await?;
        Ok(())
    }

    fn has_db(&self) -> bool {
        true
    }

    fn has_sqs(&self) -> bool {
        false // but maybe yes ?
    }

    fn has_openfga(&self) -> bool {
        false
    }
}

/// Start the IAM test server with a shared mock event publisher.
///
/// # Errors
///
/// Returns an error if the test fixture or HTTP server fails to start.
pub async fn setup_test_server_with_mock_events(
) -> Result<(TestFixture, String, Client, Arc<MockEventPublisher>), Box<dyn std::error::Error>> {
    static MOCK_EVENTS_DESCRIPTOR: OnceLock<Arc<IAMRustyTestDescriptorWithMockEvents>> =
        OnceLock::new();
    prepare_primary_fixture().await?;

    let descriptor = MOCK_EVENTS_DESCRIPTOR
        .get_or_init(|| Arc::new(IAMRustyTestDescriptorWithMockEvents::new()))
        .clone();
    descriptor.mock_event_publisher.clear_events();
    let fixture = TestFixture::new(descriptor.clone()).await?;
    let mock_event_publisher = descriptor.mock_event_publisher.clone();
    let config = fixture_config(&fixture)?;
    let (base_url, client) = Box::pin(start_owned_listener(
        &fixture,
        config,
        mock_publisher(mock_event_publisher.clone()),
        None,
    ))
    .await?;
    Ok((fixture, base_url, client, mock_event_publisher))
}
