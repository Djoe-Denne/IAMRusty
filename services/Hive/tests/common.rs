//! Common test utilities for Hive
//!
//! Mirrors [`rustycog::testing`] patterns and `Telegraph` tests structure, plus
//! the real `OpenFGA` testcontainer every permission-touching Hive test
//! routes through (mirrors `services/Manifesto/tests/common.rs`).

use async_trait::async_trait;
use reqwest::Client;
use rustycog::config::ServerConfig;
use rustycog::testing::*;
use std::sync::Arc;

use anyhow::anyhow;
use hive_configuration::AppConfig;
use hive_http::SERVICE_PREFIX;
use hive_migration::{Migrator, MigratorTrait};
use hive_setup::app::AppBuilder;

// Re-export the real OpenFGA testcontainer fixture so tests can arrange
// `Check` decisions by writing real relationship tuples without pulling
// `rustycog::testing::common::openfga_testcontainer` paths into every file.
// The harness writes **no** permissive default; each test must
// explicitly call `openfga.allow(subject, action, resource)` for every
// tuple the route guard will check (default = deny).
pub use rustycog::testing::common::openfga_testcontainer::TestOpenFga;

// Re-export the permission domain types tests need to express tuples.
#[allow(unused_imports)]
pub use rustycog::permission::{Permission, ResourceRef, Subject};

// Re-export fixtures
#[path = "fixtures/mod.rs"]
pub mod fixtures;

static mut APP: Option<hive_setup::app::Application> = None;

/// `Hive` test descriptor following [`rustycog::testing`] patterns
pub struct HiveTestDescriptor;

#[async_trait]
impl ServiceTestDescriptor<HiveTestFixture> for HiveTestDescriptor {
    type Config = AppConfig;

    async fn build_app(
        &self,
        config: AppConfig,
        _server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        let app = AppBuilder::new(config).build().await?;
        unsafe {
            APP.replace(app);
        }
        Ok(())
    }

    async fn run_app(&self, _config: AppConfig, server_config: ServerConfig) -> anyhow::Result<()> {
        // Move application out of the static store to run it (run consumes self)
        let app = unsafe { APP.take() }.ok_or_else(|| anyhow!("App not built"))?;
        app.run(server_config).await?;
        Ok(())
    }

    async fn run_migrations_up(
        &self,
        connection: &sea_orm::DatabaseConnection,
    ) -> anyhow::Result<()> {
        println!("Running migrations up");
        Migrator::up(connection, None).await?;
        Ok(())
    }

    async fn run_migrations_down(
        &self,
        connection: &sea_orm::DatabaseConnection,
    ) -> anyhow::Result<()> {
        println!("Running migrations down");
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
        true
    }

    fn openfga_authorization_model_json(&self) -> Option<&'static str> {
        Some(include_str!("../../../ops/openfga/model.json"))
    }
}

/// Hive-specific test fixture
pub struct HiveTestFixture {
    pub fixture: rustycog::testing::common::TestFixture,
}

impl HiveTestFixture {
    /// Build the fixture (DB + `OpenFGA` testcontainer + migrations).
    ///
    /// # Errors
    ///
    /// Returns an error if the shared test fixture cannot start.
    pub async fn new(
        descriptor: Arc<HiveTestDescriptor>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let fixture = rustycog::testing::common::TestFixture::new(descriptor).await?;
        Ok(Self { fixture })
    }

    /// Get the database connection
    pub fn db(&self) -> Arc<sea_orm::DatabaseConnection> {
        self.fixture.db()
    }

    /// Get the `OpenFGA` fixture
    pub const fn openfga(&self) -> &TestOpenFga {
        self.fixture.openfga()
    }
}

/// Bootstrap the Hive test server **and** the real `OpenFGA` testcontainer.
///
/// Returns a 4-tuple:
/// 1. [`HiveTestFixture`] — owns the test DB, the singleton `OpenFGA`
///    testcontainer, and the migration lifecycle.
/// 2. `String` — base URL of the live HTTP server.
/// 3. `Client` — `reqwest` client preconfigured for the test server.
/// 4. `TestOpenFga` (clone) — typed handle exposing `allow` / `deny`
///    against the real `OpenFGA` Check pipeline. The harness writes
///    **no** permissive default; each test must explicitly call
///    `openfga.allow(...)` for every tuple the route guard will check
///    (default = deny).
///
/// The `OpenFGA` fixture is process-global, so tests must remain
/// `#[serial]` to avoid tuple-state collisions.
///
/// # Errors
///
/// Returns an error if the fixture, `OpenFGA` container, or HTTP server cannot start.
pub async fn setup_test_server()
-> Result<(HiveTestFixture, String, Client, TestOpenFga), Box<dyn std::error::Error>> {
    // Bring up the OpenFGA testcontainer + database first so the env
    // vars are populated before the app boots.
    let descriptor = Arc::new(HiveTestDescriptor);
    let fixture = HiveTestFixture::new(descriptor.clone()).await?;
    let openfga = fixture.openfga().clone();

    let (server_url, client) =
        rustycog::testing::setup_test_server::<HiveTestDescriptor, HiveTestFixture>(descriptor)
            .await?;

    Ok((fixture, prefixed_url(&server_url), client, openfga))
}

fn prefixed_url(server_url: &str) -> String {
    format!("{server_url}{SERVICE_PREFIX}")
}

struct HiveListenerLease {
    db: Arc<sea_orm::DatabaseConnection>,
    _app: Arc<hive_setup::app::Application>,
    shutdown: tokio::sync::watch::Sender<bool>,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

static OWNED_LISTENERS: std::sync::OnceLock<std::sync::Mutex<Vec<HiveListenerLease>>> =
    std::sync::OnceLock::new();

fn owned_listeners() -> &'static std::sync::Mutex<Vec<HiveListenerLease>> {
    OWNED_LISTENERS.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

struct AbortHiveWorkers(Vec<tokio::task::AbortHandle>);

impl Drop for AbortHiveWorkers {
    fn drop(&mut self) {
        for worker in &self.0 {
            worker.abort();
        }
    }
}

/// Real fixture storage/OpenFGA and root; only the supplied IAM outbound config is overridden.
pub async fn setup_test_server_with_iam_service(
    iam: hive_configuration::IamServiceConfig,
) -> Result<
    (
        HiveTestFixture,
        String,
        Client,
        TestOpenFga,
        rustycog::config::AuthConfig,
    ),
    Box<dyn std::error::Error>,
> {
    // Never reset migrations underneath another live owned listener.
    let finished = {
        let mut leases = owned_listeners()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (finished, live): (Vec<_>, Vec<_>) =
            leases.drain(..).partition(|lease| lease.task.is_finished());
        *leases = live;
        finished
    };
    for lease in finished {
        let _ = lease.task.await;
    }
    if !owned_listeners()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_empty()
    {
        return Err(anyhow!(
            "join Hive cleanup_test_servers before creating another owned primary fixture"
        )
        .into());
    }
    let fixture = HiveTestFixture::new(Arc::new(HiveTestDescriptor)).await?;
    let openfga = fixture.openfga().clone();
    let database = fixture
        .fixture
        .database
        .as_ref()
        .ok_or_else(|| anyhow!("Hive fixture requires writer DB"))?;
    let db = database.get_connection();
    let db_url = reqwest::Url::parse(&database.database_url)?;
    let mut config = rustycog::config::load_config_fresh::<AppConfig>()?;
    config.database.host = db_url
        .host_str()
        .ok_or_else(|| anyhow!("missing fixture DB host"))?
        .to_owned();
    config.database.port = db_url
        .port()
        .ok_or_else(|| anyhow!("missing fixture DB port"))?;
    config.database.db = db_url.path().trim_start_matches('/').to_owned();
    config.database.creds.username = db_url.username().to_owned();
    config.database.creds.password = db_url
        .password()
        .ok_or_else(|| anyhow!("missing fixture DB credential"))?
        .to_owned();
    config.database.read_replicas.clear();
    config.iam_service = iam;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    config.server.host = "127.0.0.1".into();
    config.server.port = address.port();
    config.server.tls_enabled = false;
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let app = Arc::new(AppBuilder::new(config).build().await?);
    let effective_auth = app.config.auth.clone();
    let router = axum::Router::new().nest(SERVICE_PREFIX, app.router());
    let (shutdown, mut stop) = tokio::sync::watch::channel(false);
    let server_app = app.clone();
    let signal = shutdown.clone();
    let task = tokio::spawn(async move {
        let mut workers = server_app.start_background_tasks();
        let _abort = AbortHiveWorkers(
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
            let stopped = *stop.borrow();
            if !stopped {
                let _ = stop.changed().await;
            }
        });
        let server = std::future::IntoFuture::into_future(server);
        tokio::pin!(server);
        let (mut result, server_finished) = tokio::select! {
            result = &mut server => (result.map_err(anyhow::Error::from), true),
            result = observe_hive_workers(&mut workers) => (result, false),
        };
        signal.send_replace(true);
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        let stopped = tokio::time::timeout_at(deadline, server_app.outbox_dispatcher.stop()).await;
        let stop_result = match stopped {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(anyhow!("Hive owned dispatcher stop failed")),
            Err(_) => Err(anyhow!("Hive owned dispatcher stop timed out")),
        };
        if result.is_ok() {
            result = stop_result;
        }
        while !workers.is_empty() {
            match tokio::time::timeout_at(deadline, futures::future::select_all(workers.iter_mut()))
                .await
            {
                Ok((joined, index, rest)) => {
                    drop(rest);
                    drop(workers.swap_remove(index));
                    let joined = joined
                        .map_err(|_| anyhow!("Hive owned worker join failed"))
                        .and_then(|result| result);
                    if result.is_ok() {
                        result = joined;
                    }
                }
                Err(_) => {
                    for worker in &workers {
                        worker.abort();
                    }
                    for worker in workers.drain(..) {
                        let _ = worker.await;
                    }
                    if result.is_ok() {
                        result = Err(anyhow!("Hive owned worker drain timed out"));
                    }
                }
            }
        }
        if !server_finished {
            let drained =
                tokio::time::timeout(std::time::Duration::from_secs(5), &mut server).await;
            let drained = match drained {
                Ok(result) => result.map_err(anyhow::Error::from),
                Err(_) => Err(anyhow!("Hive owned HTTP drain timed out")),
            };
            if result.is_ok() {
                result = drained;
            }
        }
        result
    });
    owned_listeners()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(HiveListenerLease {
            db,
            _app: app,
            shutdown,
            task,
        });
    let base = format!("http://{address}{SERVICE_PREFIX}");
    for _ in 0..100 {
        if client
            .get(format!("{base}/ready"))
            .timeout(std::time::Duration::from_millis(200))
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return Ok((fixture, base, client, openfga, effective_auth));
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    cleanup_test_servers(&fixture).await?;
    Err(anyhow!("Hive owned listener failed readiness").into())
}

async fn observe_hive_workers(
    workers: &mut Vec<tokio::task::JoinHandle<anyhow::Result<()>>>,
) -> anyhow::Result<()> {
    if workers.is_empty() {
        return Err(anyhow!("Hive owned worker set is empty"));
    }
    let (result, index, rest) = futures::future::select_all(workers.iter_mut()).await;
    drop(rest);
    drop(workers.swap_remove(index));
    match result {
        Ok(Ok(())) => Err(anyhow!("Hive owned worker exited unexpectedly")),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(anyhow!("Hive owned worker join failed")),
    }
}

/// Stop/join this fixture's exact app/listener handles only; never shared containers.
pub async fn cleanup_test_servers(fixture: &HiveTestFixture) -> anyhow::Result<()> {
    let db = fixture
        .fixture
        .database
        .as_ref()
        .ok_or_else(|| anyhow!("Hive fixture requires writer DB"))?
        .get_connection();
    let leases = {
        let mut all = owned_listeners()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (owned, other): (Vec<_>, Vec<_>) =
            all.drain(..).partition(|lease| Arc::ptr_eq(&db, &lease.db));
        *all = other;
        owned
    };
    for lease in &leases {
        lease.shutdown.send_replace(true);
    }
    let mut first_error = None;
    for mut lease in leases {
        let error =
            match tokio::time::timeout(std::time::Duration::from_secs(15), &mut lease.task).await {
                Ok(Ok(result)) => result.err(),
                Ok(Err(_)) => Some(anyhow!("Hive owned listener join failed")),
                Err(_) => {
                    lease.task.abort();
                    let _ = lease.task.await;
                    Some(anyhow!("Hive owned listener cleanup timed out"))
                }
            };
        if first_error.is_none() {
            first_error = error;
        }
    }
    first_error.map_or(Ok(()), Err)
}
