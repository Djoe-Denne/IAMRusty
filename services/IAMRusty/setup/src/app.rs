use anyhow::Result;
use axum::Router;
use chrono::Duration;
use futures::future::select_all;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::info;

use iam_http_server::oauth_browser::{OAuthBrowserPolicy, OAuthRouteContext};
use iam_http_server::rate_limit::AuthRateLimiter;
use iam_http_server::{
    IamHttpSecurityContext, PlatformIssuer, SignerRouteContext, create_app_routes, create_router,
    handlers::organization_signer::SignerRouteContextParams,
};
use iam_infra::{
    auth::{
        HttpIdpConnector, PasswordResetServiceAdapter, PasswordService, PasswordServiceAdapter,
    },
    db::DbConnectionPool,
    event_adapter::IAMErrorMapper,
    repository::{
        SeaOrmAuthenticationSessionWriter, SeaOrmIdentityRepository,
        SeaOrmOAuthTransactionWriteRepository, SeaOrmSigningKeyRegistry,
        bootstrap_platform_signing_key,
        combined_email_verification_repository::CombinedEmailVerificationRepository,
        combined_password_reset_token_repository::CombinedPasswordResetTokenRepository,
        combined_repository::{
            CombinedRefreshTokenRepository, CombinedTokenRepository, CombinedUserRepository,
        },
        combined_user_email_repository::CombinedUserEmailRepository,
        email_verification_read::SeaOrmEmailVerificationReadRepository,
        email_verification_write::SeaOrmEmailVerificationWriteRepository,
        password_reset_token_read::PasswordResetTokenReadRepositoryImpl,
        password_reset_token_write::PasswordResetTokenWriteRepositoryImpl,
        refresh_token_read::RefreshTokenReadRepositoryImpl,
        refresh_token_write::RefreshTokenWriteRepositoryImpl,
        signup_transaction::SignupTransactionImpl,
        token_read::TokenReadRepositoryImpl,
        token_write::TokenWriteRepositoryImpl,
        user_email_read::UserEmailReadRepositoryImpl,
        user_email_write::UserEmailWriteRepositoryImpl,
        user_read::UserReadRepositoryImpl,
        user_write::UserWriteRepositoryImpl,
    },
    signing::{
        DefaultOrganizationSignerProbe, DefaultOrganizationSignerRotator, PemSigningProvider,
        ProviderBindingProbe, RemoteSigningProvider, RotateContext, ScopedSigningProvider,
        TransitClientConfig, TransitSigningProvider, compose_workload_identity,
    },
    token::{JwtAlgorithm, JwtTokenService},
    transaction::IamOutboxUnitOfWorkImpl,
};
use rustycog::command::{CommandError, CommandErrorMapper};
use rustycog::http::{AppState, LocalJwksSeed, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};
use uuid::Uuid;

use iam_configuration::{AppConfig, IdpConfig, SecretStorage, SecurityMode};
use iam_domain::entity::provider::Provider;
use iam_domain::entity::signing_key::{JWKS_RETIRE_SKEW_SECONDS, SigningProviderType};
use iam_domain::error::DomainError;
use iam_domain::port::repository::{
    AuthenticationSessionWriter, OAuthTransactionWriteRepository, SigningKeyRegistry,
};
use iam_domain::port::service::FederatedOAuthClient;
use iam_domain::port::{OrganizationSignerProbe, SigningProvider};
use readiness::{
    ComponentStatus, QueueRole, ReadinessProbe, attach_ready,
    create_signaled_multi_queue_event_publisher, signal_queue_status,
};
use rustycog::events::{adapter::MultiQueueEventPublisher, event::EventPublisher};
use rustycog::outbox::{OutboxConfig, OutboxDispatcher, OutboxRecorder};

use iam_application::{
    background::{OAuthTransactionCleanup, OAuthTransactionCleanupPolicy},
    command::{CommandRegistryFactory, GenericCommandService, IamRegistryUseCases},
    usecase::{
        link_provider::{LinkProviderUseCase, LinkProviderUseCaseImpl},
        login::{LoginUseCase, LoginUseCaseImpl},
        oauth::{OAuthUseCase, OAuthUseCaseImpl},
        password_reset::{PasswordResetUseCase, PasswordResetUseCaseImpl},
        provider::{ProviderUseCase, ProviderUseCaseImpl},
        registration::{RegistrationUseCase, RegistrationUseCaseImpl},
        token::{TokenUseCase, TokenUseCaseImpl},
        user::{UserUseCase, UserUseCaseImpl},
    },
};

use crate::config::ServerConfig;

pub struct IAMRustyApp {
    app_state: AppState,
    outbox_dispatcher: Arc<OutboxDispatcher<DomainError>>,
    readiness: Arc<ReadinessProbe>,
    idp: Arc<IdpConfig>,
    signer: Option<Arc<SignerRouteContext>>,
    http_security: Arc<IamHttpSecurityContext>,
    jwt_codec: Arc<JwtTokenService>,
    signing_registry: Arc<SeaOrmSigningKeyRegistry>,
    access_token_ttl: u64,
    oauth_cleanup: Arc<OAuthTransactionCleanup>,
    oauth_cleanup_stop: watch::Sender<bool>,
}

pub struct IAMRustyAppParts {
    pub app_state: AppState,
    pub outbox_dispatcher: Arc<OutboxDispatcher<DomainError>>,
    pub readiness: Arc<ReadinessProbe>,
    pub idp: Arc<IdpConfig>,
    pub signer: Option<Arc<SignerRouteContext>>,
    pub signing_registry: Arc<SeaOrmSigningKeyRegistry>,
    pub access_token_ttl: u64,
    pub jwt_codec: Arc<JwtTokenService>,
    pub security: Arc<IamHttpSecurityContext>,
    pub oauth_cleanup: Arc<OAuthTransactionCleanup>,
}

impl IAMRustyApp {
    #[must_use]
    pub fn new(parts: IAMRustyAppParts) -> Self {
        let IAMRustyAppParts {
            app_state,
            outbox_dispatcher,
            readiness,
            idp,
            signer,
            signing_registry,
            access_token_ttl,
            jwt_codec,
            security,
            oauth_cleanup,
        } = parts;
        let (oauth_cleanup_stop, _) = watch::channel(false);
        Self {
            app_state,
            outbox_dispatcher,
            readiness,
            idp,
            signer,
            http_security: security,
            jwt_codec,
            signing_registry,
            access_token_ttl,
            oauth_cleanup,
            oauth_cleanup_stop,
        }
    }

    pub fn router(&self) -> Router {
        attach_ready(
            create_router(
                self.app_state.clone(),
                self.idp.clone(),
                self.signer.clone(),
                &self.http_security,
            ),
            self.readiness.clone(),
        )
    }

    /// The root-created codec shared by access and completion issuance.
    /// No private key material or HTTP extension is exposed by this accessor.
    #[must_use]
    pub fn jwt_codec(&self) -> Arc<JwtTokenService> {
        self.jwt_codec.clone()
    }

    #[must_use]
    pub fn signing_registry(&self) -> (Arc<SeaOrmSigningKeyRegistry>, u64) {
        (self.signing_registry.clone(), self.access_token_ttl)
    }

    #[must_use]
    pub fn readiness(&self) -> Arc<ReadinessProbe> {
        self.readiness.clone()
    }

    #[must_use]
    pub fn state(&self) -> AppState {
        self.app_state.clone()
    }

    /// Organization-signer application façade (ADR-0306 `InProcess` injection).
    ///
    /// Present when JWT/signing registry was wired at build time. Callers in the
    /// monolith composition root must fail-closed if this returns [`None`].
    #[must_use]
    pub fn organization_signer(
        &self,
    ) -> Option<Arc<dyn iam_application::usecase::OrganizationSignerFacade>> {
        self.signer.as_ref().map(|ctx| ctx.facade.clone())
    }

    #[must_use]
    pub fn start_background_tasks(&self) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        let dispatcher = self.outbox_dispatcher.clone();
        let cleanup = self.oauth_cleanup.clone();
        let stop = self.oauth_cleanup_stop.subscribe();
        vec![
            tokio::spawn(async move {
                dispatcher
                    .start()
                    .await
                    .map_err(|_| anyhow::anyhow!("IAMRusty outbox dispatcher failed"))
            }),
            tokio::spawn(async move {
                cleanup
                    .run(stop)
                    .await
                    .map_err(|_| anyhow::anyhow!("IAM OAuth transaction cleanup failed"))
            }),
        ]
    }

    /// # Errors
    ///
    /// Returns an error if the outbox dispatcher cannot stop.
    pub async fn stop_background_tasks(&self) -> Result<()> {
        self.oauth_cleanup_stop.send_replace(true);
        self.outbox_dispatcher
            .stop()
            .await
            .map_err(|_| anyhow::anyhow!("Failed to stop IAMRusty outbox dispatcher"))
    }

    /// Cooperatively stop, then join every owned IAM handle under one five-second budget.
    /// Timed-out handles are aborted and awaited; no worker handle is discarded.
    ///
    /// # Errors
    ///
    /// Returns an error if cooperative stop or a background worker fails, or if the drain times out.
    pub async fn shutdown_background_tasks(
        &self,
        handles: &mut Vec<JoinHandle<Result<()>>>,
    ) -> Result<()> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut first_error =
            match tokio::time::timeout_at(deadline, self.stop_background_tasks()).await {
                Ok(result) => result.err(),
                Err(_) => Some(anyhow::anyhow!("IAM cooperative background stop timed out")),
            };
        while !handles.is_empty() {
            let joined = tokio::time::timeout_at(deadline, select_all(handles.iter_mut())).await;
            if let Ok((result, index, remaining)) = joined {
                drop(remaining);
                drop(handles.swap_remove(index));
                let error = match result {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(error),
                    Err(_) => Some(anyhow::anyhow!("IAM background worker join failed")),
                };
                if first_error.is_none() {
                    first_error = error;
                }
            } else {
                for handle in handles.iter() {
                    handle.abort();
                }
                for handle in handles.drain(..) {
                    let _ = handle.await;
                }
                if first_error.is_none() {
                    first_error = Some(anyhow::anyhow!("IAM background drain timed out"));
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

/// Build IAM app state and serve HTTP until shutdown.
///
/// # Errors
///
/// Returns an error if app state cannot be built or the server fails.
pub async fn build_and_run(
    config: AppConfig,
    server_config: ServerConfig,
    maybe_event_publisher: Option<Arc<MultiQueueEventPublisher<DomainError>>>,
) -> Result<()> {
    let app_state = Box::pin(build_app_state(config.clone(), maybe_event_publisher)).await?;
    run_server(app_state, server_config).await
}

/// Build IAM app state, creating a queue publisher when none is injected.
///
/// # Errors
///
/// Returns an error if the queue publisher or downstream app state cannot be created.
pub async fn build_app_state(
    config: AppConfig,
    maybe_event_publisher: Option<Arc<MultiQueueEventPublisher<DomainError>>>,
) -> Result<IAMRustyApp> {
    let oauth_state_secret = config
        .security
        .validate(&config.jwt.oauth_state_secret, &config.idp)
        .map_err(anyhow::Error::msg)?;
    iam_http_server::configure_oauth_state_secret(&oauth_state_secret)
        .map_err(anyhow::Error::msg)?;
    let (event_publisher, queue_status, queue_transport) =
        if let Some(publisher) = maybe_event_publisher {
            signal_queue_status("iam", QueueRole::Publisher, &ComponentStatus::Injected);
            (publisher, ComponentStatus::Injected, None)
        } else {
            let signaled = create_signaled_multi_queue_event_publisher(
                "iam",
                &config.queue,
                None::<Vec<String>>,
                Arc::new(IAMErrorMapper),
            )
            .await?;
            (
                signaled.publisher,
                signaled.status,
                Some(signaled.transport),
            )
        };

    build_app_state_with_event_publisher(config, event_publisher, queue_status, queue_transport)
        .await
}

/// Build app state with a custom event publisher (useful for testing).
///
/// # Errors
///
/// Returns an error if the database, JWT, command registry, or readiness wiring fails.
pub async fn build_app_state_with_event_publisher<EP>(
    config: AppConfig,
    event_publisher: Arc<EP>,
    queue_status: ComponentStatus,
    queue_transport: Option<Arc<rustycog::events::ConcreteEventPublisher>>,
) -> Result<IAMRustyApp>
where
    EP: EventPublisher<DomainError> + Send + Sync + 'static,
{
    build_app_state_with_event_publisher_and_signing_keys(
        config,
        event_publisher,
        queue_status,
        queue_transport,
        &[],
    )
    .await
}

struct IamSigningPool {
    platform_issuer: PlatformIssuer,
    rate_limiter: Arc<AuthRateLimiter>,
    browser_policy: OAuthBrowserPolicy,
    db_pool: DbConnectionPool,
    db_write: Arc<sea_orm::DatabaseConnection>,
    signing_registry: Arc<SeaOrmSigningKeyRegistry>,
    outbox_dispatcher: Arc<OutboxDispatcher<DomainError>>,
}

fn setup_iam_identity_stack(
    config: &AppConfig,
    db_pool: &DbConnectionPool,
    signing_registry: Arc<SeaOrmSigningKeyRegistry>,
) -> Result<IamIdentityStack> {
    let repos = setup_repositories(db_pool, config.jwt.platform_issuer(), signing_registry);
    let idp = Arc::new(config.idp.clone());
    let oauth_clients = setup_oauth_clients(config)?;
    let password_service = Arc::new(PasswordService::new());
    let password_service_adapter = Arc::new(PasswordServiceAdapter::new(password_service.clone()));
    Ok((
        repos,
        idp,
        oauth_clients,
        password_service,
        password_service_adapter,
    ))
}

fn setup_oauth_cleanup(
    db_pool: &DbConnectionPool,
) -> Result<(
    Arc<dyn OAuthTransactionWriteRepository>,
    Arc<OAuthTransactionCleanup>,
)> {
    let oauth_transaction_writer: Arc<dyn OAuthTransactionWriteRepository> = Arc::new(
        SeaOrmOAuthTransactionWriteRepository::new(db_pool.get_write_connection()),
    );
    let oauth_cleanup = Arc::new(
        OAuthTransactionCleanup::new(
            oauth_transaction_writer.clone(),
            OAuthTransactionCleanupPolicy::default(),
        )
        .map_err(|_| anyhow::anyhow!("Invalid OAuth transaction cleanup policy"))?,
    );
    Ok((oauth_transaction_writer, oauth_cleanup))
}

fn install_oauth_state_secret(config: &AppConfig) -> Result<()> {
    let oauth_state_secret = config
        .security
        .validate(&config.jwt.oauth_state_secret, &config.idp)
        .map_err(anyhow::Error::msg)?;
    iam_http_server::configure_oauth_state_secret(&oauth_state_secret)
        .map_err(anyhow::Error::msg)?;
    Ok(())
}

async fn open_iam_signing_pool<EP>(
    config: &AppConfig,
    event_publisher: Arc<EP>,
    signing_keys: &[iam_domain::entity::signing_key::SigningKey],
) -> Result<IamSigningPool>
where
    EP: EventPublisher<DomainError> + Send + Sync + 'static,
{
    let platform_issuer =
        PlatformIssuer::new(config.jwt.platform_issuer()).map_err(anyhow::Error::msg)?;
    let rate_limiter = Arc::new(
        AuthRateLimiter::new(config.security.rate_limit.clone(), config.security.mode)
            .map_err(anyhow::Error::msg)?,
    );
    let browser_policy = OAuthBrowserPolicy::new(config.security.mode, &config.jwt.public_base_url)
        .map_err(anyhow::Error::msg)?;
    let signing_policy =
        Arc::new(iam_domain::entity::signing_key::SigningKeyLifecyclePolicy::new()?);
    let db_pool = DbConnectionPool::new(&config.database).await?;
    let db_write = db_pool.get_write_connection();
    let signing_registry = Arc::new(SeaOrmSigningKeyRegistry::new(
        db_write.clone(),
        signing_policy,
        config.jwt.expiration_seconds,
    )?);
    for key in signing_keys {
        iam_domain::port::repository::SigningKeyRegistry::insert(signing_registry.as_ref(), key)
            .await?;
    }
    let dispatcher_publisher: Arc<dyn EventPublisher<DomainError>> = event_publisher;
    let outbox_dispatcher = Arc::new(OutboxDispatcher::new(
        db_pool.clone(),
        dispatcher_publisher,
        OutboxConfig::default(),
    ));
    info!(
        "Database connection pool initialized with {} read replicas",
        if config.database.read_replicas.is_empty() {
            0
        } else {
            config.database.read_replicas.len()
        }
    );
    Ok(IamSigningPool {
        platform_issuer,
        rate_limiter,
        browser_policy,
        db_pool,
        db_write,
        signing_registry,
        outbox_dispatcher,
    })
}

fn seed_isolated_signing_keys(
    config: &AppConfig,
    signing_keys: &[iam_domain::entity::signing_key::SigningKey],
) -> Result<()> {
    if signing_keys.is_empty() {
        return Ok(());
    }
    anyhow::ensure!(
        config.security.mode == iam_configuration::security::SecurityMode::IsolatedTest,
        "initial fixture keys require explicit isolated_test mode"
    );
    let iam_configuration::JwtAlgorithm::RS256(pair) = config.jwt.create_jwt_algorithm()? else {
        anyhow::bail!("initial signing fixture keys require RSA configuration");
    };
    PemSigningProvider::new(&pair.private_key, pair.public_key)?;
    Ok(())
}

async fn capture_local_jwks_if_rs256(
    http_verifier_auth: &iam_configuration::AuthConfig,
    usecases: &IamRegistryUseCases,
) -> Result<Option<LocalJwksSeed>> {
    if !http_verifier_auth
        .jwt
        .allowed_algorithms
        .iter()
        .any(|a| a == "RS256")
    {
        // Preserve the explicit HS256-only legacy mode, without inventing a
        // JWKS URL, platform issuer, seed, or enabling another algorithm.
        return Ok(None);
    }
    let authority_url = http_verifier_auth
        .jwt
        .jwks_url
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Invalid auth configuration: missing JWKS URL"))?;
    let jwks_seed = LocalJwksSeed::capture(authority_url, || async {
        let jwks = usecases.token.get_jwks().await.map_err(|e| {
            iam_application::command::token::TokenErrorMapper.map_error(Box::new(e))
        })?;
        serde_json::to_string(&jwks).map_err(|_| {
            CommandError::infrastructure("invalid_jwks", "Invalid signing registry snapshot")
        })
    })
    .await
    .map_err(|e| anyhow::anyhow!("Local JWKS capture failed: {e}"))?;
    Ok(Some(jwks_seed))
}

/// Isolated fixtures seed through the SAME root-created admission writer before bootstrap.
/// Normal production construction supplies no fixture keys.
///
/// # Errors
///
/// Returns an error if the database, JWT, command registry, signing, or readiness wiring fails.
pub async fn build_app_state_with_event_publisher_and_signing_keys<EP>(
    config: AppConfig,
    event_publisher: Arc<EP>,
    queue_status: ComponentStatus,
    queue_transport: Option<Arc<rustycog::events::ConcreteEventPublisher>>,
    signing_keys: &[iam_domain::entity::signing_key::SigningKey],
) -> Result<IAMRustyApp>
where
    EP: EventPublisher<DomainError> + Send + Sync + 'static,
{
    seed_isolated_signing_keys(&config, signing_keys)?;
    install_oauth_state_secret(&config)?;
    info!("Building IAM service...");
    let IamSigningPool {
        platform_issuer,
        rate_limiter,
        browser_policy,
        db_pool,
        db_write,
        signing_registry,
        outbox_dispatcher,
    } = open_iam_signing_pool(&config, event_publisher.clone(), signing_keys).await?;

    let (repos, idp, oauth_clients, password_service, password_service_adapter) =
        setup_iam_identity_stack(&config, &db_pool, signing_registry.clone())?;

    let (http_verifier_auth, _inline_jwks, token_service, registration_token_service, signer_ctx) =
        setup_jwt(&config, db_write.clone(), signing_registry.clone()).await?;

    let outbox_unit_of_work = Arc::new(IamOutboxUnitOfWorkImpl::new(
        db_pool.clone(),
        OutboxRecorder,
    ));
    let (oauth_transaction_writer, oauth_cleanup) = setup_oauth_cleanup(&db_pool)?;
    let usecases = setup_iam_usecases(
        &db_pool,
        IamUsecasesDeps {
            oauth_transaction_writer,
            repos,
            event_publisher: event_publisher.clone(),
            clients: oauth_clients,
            password_service,
            password_service_adapter,
            token_service: token_service.clone(),
            registration_token_service,
            outbox_unit_of_work,
        },
    );
    let oauth = Arc::new(OAuthRouteContext {
        use_case: usecases.oauth.clone(),
        browser: browser_policy,
    });
    let http_security = Arc::new(IamHttpSecurityContext::new(
        platform_issuer,
        rate_limiter,
        oauth,
    ));
    // Capture the complete primary publisher snapshot before moving its use cases
    // into the registry. The SDK dates acquisition before this local read; neither
    // capture nor extractor construction performs HTTP or renews the seed's TTL.
    let jwks_seed = capture_local_jwks_if_rs256(&http_verifier_auth, &usecases).await?;

    let registry = CommandRegistryFactory::create_iam_registry(usecases, &config.command);
    let command_service = Arc::new(GenericCommandService::new(Arc::new(registry)));

    // The configured live publisher is authoritative, including organization
    // epochs and revocations. Its fresh local seed covers the first bearer burst;
    // subsequent refresh replaces it, with no inline/bootstrap fallback.
    let user_id_extractor = configured_http_verifier(http_verifier_auth, jwks_seed)?;

    // IAM routes are never guarded by `with_permission_on` — IAM is the
    // identity provider, not a resource service — so we plug in an empty
    // in-memory checker purely to satisfy `AppState::new`.
    let permission_checker: Arc<dyn PermissionChecker> = Arc::new(InMemoryPermissionChecker::new());

    // Create app state
    let app_state = AppState::new(command_service, user_id_extractor, permission_checker);

    let readiness = Arc::new(
        ReadinessProbe::new("iam")
            .with_database(db_write)
            .with_publisher(queue_status, queue_transport),
    );

    Ok(IAMRustyApp::new(IAMRustyAppParts {
        app_state,
        outbox_dispatcher,
        readiness,
        idp,
        signer: signer_ctx,
        signing_registry,
        access_token_ttl: config.jwt.expiration_seconds,
        jwt_codec: token_service,
        security: http_security,
        oauth_cleanup,
    }))
}

type UserRepo = CombinedUserRepository<UserReadRepositoryImpl, UserWriteRepositoryImpl>;
type UserEmailRepo =
    CombinedUserEmailRepository<UserEmailReadRepositoryImpl, UserEmailWriteRepositoryImpl>;
type TokenRepo = CombinedTokenRepository<TokenReadRepositoryImpl, TokenWriteRepositoryImpl>;
type RefreshRepo =
    CombinedRefreshTokenRepository<RefreshTokenReadRepositoryImpl, RefreshTokenWriteRepositoryImpl>;

type PasswordResetRepo = CombinedPasswordResetTokenRepository<
    PasswordResetTokenReadRepositoryImpl,
    PasswordResetTokenWriteRepositoryImpl,
>;

struct IamRepos {
    user_repo: UserRepo,
    user_email_repo: UserEmailRepo,
    email_verification_repo: CombinedEmailVerificationRepository,
    password_reset_repo: PasswordResetRepo,
    token_repo_login: TokenRepo,
    token_repo_link: TokenRepo,
    refresh_token_repo: RefreshRepo,
    identity_repo: Arc<dyn iam_domain::port::repository::IdentityRepository<Error = DomainError>>,
    signing_key_registry:
        Arc<dyn iam_domain::port::repository::SigningKeyRegistry<Error = DomainError>>,
    platform_issuer: String,
}

struct OauthClients {
    by_slug: HashMap<Provider, Arc<dyn FederatedOAuthClient>>,
}

type IamIdentityStack = (
    IamRepos,
    Arc<IdpConfig>,
    OauthClients,
    Arc<PasswordService>,
    Arc<PasswordServiceAdapter>,
);

struct OauthLinkDeps {
    session_writer: Arc<dyn AuthenticationSessionWriter>,
    oauth_transaction_writer: Arc<dyn OAuthTransactionWriteRepository>,
    user_repo: UserRepo,
    user_email_repo: UserEmailRepo,
    token_repo_login: TokenRepo,
    token_repo_link: TokenRepo,
    clients: OauthClients,
    token_service: Arc<JwtTokenService>,
    registration_token_service: Arc<iam_infra::token::RegistrationTokenServiceImpl>,
    identity_repo: Arc<dyn iam_domain::port::repository::IdentityRepository<Error = DomainError>>,
    platform_issuer: String,
}

struct AuthRegistrationDeps<EP> {
    session_writer: Arc<dyn AuthenticationSessionWriter>,
    user_repo: UserRepo,
    user_email_repo: UserEmailRepo,
    email_verification_repo: CombinedEmailVerificationRepository,
    password_reset_repo: PasswordResetRepo,
    event_publisher: Arc<EP>,
    password_service: Arc<PasswordService>,
    password_service_adapter: Arc<PasswordServiceAdapter>,
    token_service: Arc<JwtTokenService>,
    registration_token_service: Arc<iam_infra::token::RegistrationTokenServiceImpl>,
    outbox_unit_of_work: Arc<IamOutboxUnitOfWorkImpl>,
    identity_repo: Arc<dyn iam_domain::port::repository::IdentityRepository<Error = DomainError>>,
    platform_issuer: String,
}

struct IamUsecasesDeps<EP> {
    oauth_transaction_writer: Arc<dyn OAuthTransactionWriteRepository>,
    repos: IamRepos,
    event_publisher: Arc<EP>,
    clients: OauthClients,
    password_service: Arc<PasswordService>,
    password_service_adapter: Arc<PasswordServiceAdapter>,
    token_service: Arc<JwtTokenService>,
    registration_token_service: Arc<iam_infra::token::RegistrationTokenServiceImpl>,
    outbox_unit_of_work: Arc<IamOutboxUnitOfWorkImpl>,
}

fn setup_oauth_and_link(
    deps: OauthLinkDeps,
) -> (Arc<dyn OAuthUseCase>, Arc<dyn LinkProviderUseCase>) {
    let OauthLinkDeps {
        session_writer,
        oauth_transaction_writer,
        user_repo,
        user_email_repo,
        token_repo_login,
        token_repo_link,
        clients,
        token_service,
        registration_token_service,
        identity_repo,
        platform_issuer,
    } = deps;
    let clients = Arc::new(clients.by_slug);
    let oauth_service = iam_domain::service::oauth_service::OAuthService::new(
        user_repo.clone(),
        token_repo_login,
        user_email_repo.clone(),
        iam_domain::service::TokenService::new(token_service.clone(), Duration::hours(1)),
        clients.clone(),
    );
    let oauth = Arc::new(
        OAuthUseCaseImpl::new(
            Arc::new(oauth_service),
            registration_token_service,
            token_service,
            identity_repo,
            platform_issuer,
        )
        .with_session_writer(session_writer)
        .with_oauth_transaction_writer(oauth_transaction_writer),
    );
    let provider_link_service = Arc::new(iam_domain::service::ProviderLinkService::new(
        Arc::new(user_repo),
        Arc::new(user_email_repo),
        Arc::new(token_repo_link),
    ));
    let link_provider = Arc::new(LinkProviderUseCaseImpl::new(clients, provider_link_service));
    (oauth, link_provider)
}

fn setup_auth_registration_password<EP>(
    db_pool: &DbConnectionPool,
    deps: AuthRegistrationDeps<EP>,
) -> (
    Arc<dyn LoginUseCase>,
    Arc<dyn RegistrationUseCase>,
    Arc<dyn PasswordResetUseCase>,
)
where
    EP: EventPublisher<DomainError> + Send + Sync + 'static,
{
    let AuthRegistrationDeps {
        session_writer,
        user_repo,
        user_email_repo,
        email_verification_repo,
        password_reset_repo,
        event_publisher,
        password_service,
        password_service_adapter,
        token_service,
        registration_token_service,
        outbox_unit_of_work,
        identity_repo,
        platform_issuer,
    } = deps;
    let signup_transaction = Arc::new(SignupTransactionImpl::new(db_pool.get_write_connection()));
    let auth_service = Arc::new(
        iam_domain::service::auth_service::AuthService::new_with_signup_transaction_and_outbox(
            iam_domain::service::auth_service::AuthServiceDependencies {
                user_repository: Arc::new(user_repo.clone()),
                user_email_repository: Arc::new(user_email_repo.clone()),
                email_verification_repository: Arc::new(email_verification_repo.clone()),
                password_service: password_service_adapter,
                token_service: token_service.clone(),
                registration_token_service: registration_token_service.clone(),
                event_publisher: event_publisher.clone(),
            },
            signup_transaction,
            outbox_unit_of_work.clone(),
        )
        .with_session_writer(session_writer.clone()),
    );
    let login_auth = Arc::new(LoginUseCaseImpl::new(
        auth_service,
        identity_repo.clone(),
        platform_issuer.clone(),
    ));
    let registration_service = Arc::new(
        iam_domain::service::RegistrationServiceImpl::new_with_outbox_unit_of_work(
            iam_domain::service::registration_service::RegistrationServiceDependencies {
                user_read_repo: Arc::new(user_repo.clone()),
                user_write_repo: Arc::new(user_repo.clone()),
                user_email_repo: Arc::new(user_email_repo.clone()),
                email_verification_repo: Arc::new(email_verification_repo),
                registration_token_service,
                token_service: token_service.clone(),
                event_publisher: event_publisher.clone(),
            },
            outbox_unit_of_work.clone(),
        )
        .with_session_writer(session_writer.clone()),
    );
    let registration = Arc::new(RegistrationUseCaseImpl::new(
        registration_service,
        identity_repo,
        platform_issuer,
    ));
    let password_reset_service_adapter =
        Arc::new(PasswordResetServiceAdapter::new(password_service));
    let password_reset = Arc::new(
        PasswordResetUseCaseImpl::new_with_outbox_unit_of_work(
            Arc::new(user_repo),
            Arc::new(user_email_repo),
            Arc::new(password_reset_repo),
            token_service,
            event_publisher,
            password_reset_service_adapter,
            outbox_unit_of_work,
        )
        .with_session_writer(session_writer),
    );
    (login_auth, registration, password_reset)
}

struct ProviderUserTokenDeps {
    user_repo: UserRepo,
    user_email_repo: UserEmailRepo,
    refresh_token_repo: RefreshRepo,
    token_service: Arc<JwtTokenService>,
    signing_key_registry:
        Arc<dyn iam_domain::port::repository::SigningKeyRegistry<Error = DomainError>>,
    identity_repo: Arc<dyn iam_domain::port::repository::IdentityRepository<Error = DomainError>>,
    platform_issuer: String,
    access_token_expiration_seconds: u64,
}

fn setup_provider_user_token(
    db_pool: &DbConnectionPool,
    deps: ProviderUserTokenDeps,
) -> (
    Arc<dyn ProviderUseCase>,
    Arc<dyn UserUseCase>,
    Arc<dyn TokenUseCase>,
) {
    let ProviderUserTokenDeps {
        user_repo,
        user_email_repo,
        refresh_token_repo,
        token_service,
        signing_key_registry,
        identity_repo,
        platform_issuer,
        access_token_expiration_seconds,
    } = deps;
    let token_repo_provider = CombinedTokenRepository::new(
        TokenReadRepositoryImpl::new(db_pool.get_read_connection()),
        TokenWriteRepositoryImpl::new(db_pool.get_write_connection()),
    );
    let provider_auth_service = iam_domain::service::oauth_service::OAuthService::new(
        user_repo.clone(),
        token_repo_provider,
        user_email_repo.clone(),
        iam_domain::service::token_service::TokenService::new(
            token_service.clone(),
            Duration::hours(1),
        ),
        Arc::new(HashMap::new()),
    );
    let provider = Arc::new(ProviderUseCaseImpl::new(Arc::new(provider_auth_service)));
    let user = Arc::new(UserUseCaseImpl::new(Arc::new(
        iam_domain::service::UserServiceImpl::new(
            Arc::new(user_repo),
            Arc::new(user_email_repo),
            token_service.clone(),
        ),
    )));
    let token = Arc::new(TokenUseCaseImpl::with_expiration(
        Arc::new(iam_domain::service::RefreshTokenServiceImpl::new(
            Arc::new(refresh_token_repo),
            token_service,
        )),
        signing_key_registry,
        identity_repo,
        platform_issuer,
        access_token_expiration_seconds,
    ));
    (provider, user, token)
}

fn setup_iam_usecases<EP>(
    db_pool: &DbConnectionPool,
    deps: IamUsecasesDeps<EP>,
) -> IamRegistryUseCases
where
    EP: EventPublisher<DomainError> + Send + Sync + 'static,
{
    let IamUsecasesDeps {
        oauth_transaction_writer,
        repos,
        event_publisher,
        clients,
        password_service,
        password_service_adapter,
        token_service,
        registration_token_service,
        outbox_unit_of_work,
    } = deps;
    let IamRepos {
        user_repo,
        user_email_repo,
        email_verification_repo,
        password_reset_repo,
        token_repo_login,
        token_repo_link,
        refresh_token_repo,
        identity_repo,
        signing_key_registry,
        platform_issuer,
    } = repos;
    let session_writer: Arc<dyn AuthenticationSessionWriter> = Arc::new(
        SeaOrmAuthenticationSessionWriter::new(db_pool.get_write_connection()),
    );
    let (oauth, link_provider) = setup_oauth_and_link(OauthLinkDeps {
        session_writer: session_writer.clone(),
        oauth_transaction_writer,
        user_repo: user_repo.clone(),
        user_email_repo: user_email_repo.clone(),
        token_repo_login,
        token_repo_link,
        clients,
        token_service: token_service.clone(),
        registration_token_service: registration_token_service.clone(),
        identity_repo: identity_repo.clone(),
        platform_issuer: platform_issuer.clone(),
    });
    let (login_auth, registration, password_reset) = setup_auth_registration_password(
        db_pool,
        AuthRegistrationDeps {
            session_writer,
            user_repo: user_repo.clone(),
            user_email_repo: user_email_repo.clone(),
            email_verification_repo,
            password_reset_repo,
            event_publisher,
            password_service,
            password_service_adapter,
            token_service: token_service.clone(),
            registration_token_service,
            outbox_unit_of_work,
            identity_repo: identity_repo.clone(),
            platform_issuer: platform_issuer.clone(),
        },
    );
    let (provider, user, token) = setup_provider_user_token(
        db_pool,
        ProviderUserTokenDeps {
            user_repo,
            user_email_repo,
            refresh_token_repo,
            token_service: token_service.clone(),
            signing_key_registry,
            identity_repo,
            platform_issuer,
            access_token_expiration_seconds: token_service.access_token_expiration_seconds(),
        },
    );
    IamRegistryUseCases {
        oauth,
        link_provider,
        provider,
        token,
        user,
        login_auth,
        registration,
        password_reset,
    }
}

fn setup_repositories(
    db_pool: &DbConnectionPool,
    platform_issuer: String,
    signing_registry: Arc<SeaOrmSigningKeyRegistry>,
) -> IamRepos {
    let user_repo = CombinedUserRepository::new(
        UserReadRepositoryImpl::new(db_pool.get_read_connection()),
        UserWriteRepositoryImpl::new(db_pool.get_write_connection()),
    );
    let user_email_repo = CombinedUserEmailRepository::new(
        UserEmailReadRepositoryImpl::new(db_pool.get_read_connection()),
        UserEmailWriteRepositoryImpl::new(db_pool.get_write_connection()),
    );
    let email_verification_repo = CombinedEmailVerificationRepository::new_with_sea_orm(
        Arc::new(SeaOrmEmailVerificationReadRepository::new(
            db_pool.get_read_connection(),
        )),
        Arc::new(SeaOrmEmailVerificationWriteRepository::new(
            db_pool.get_write_connection(),
        )),
    );
    let password_reset_repo = CombinedPasswordResetTokenRepository::new(
        Arc::new(PasswordResetTokenReadRepositoryImpl::new(
            db_pool.get_read_connection(),
        )),
        Arc::new(PasswordResetTokenWriteRepositoryImpl::new(
            db_pool.get_write_connection(),
        )),
    );
    let token_repo_login = CombinedTokenRepository::new(
        TokenReadRepositoryImpl::new(db_pool.get_read_connection()),
        TokenWriteRepositoryImpl::new(db_pool.get_write_connection()),
    );
    let token_repo_link = CombinedTokenRepository::new(
        TokenReadRepositoryImpl::new(db_pool.get_read_connection()),
        TokenWriteRepositoryImpl::new(db_pool.get_write_connection()),
    );
    let refresh_token_repo = CombinedRefreshTokenRepository::new(
        RefreshTokenReadRepositoryImpl::new(db_pool.get_read_connection()),
        RefreshTokenWriteRepositoryImpl::new(db_pool.get_write_connection()),
    );
    let identity_repo = Arc::new(SeaOrmIdentityRepository::new(
        db_pool.get_write_connection(),
    ))
        as Arc<dyn iam_domain::port::repository::IdentityRepository<Error = DomainError>>;
    let signing_key_registry = signing_registry
        as Arc<dyn iam_domain::port::repository::SigningKeyRegistry<Error = DomainError>>;
    IamRepos {
        user_repo,
        user_email_repo,
        email_verification_repo,
        password_reset_repo,
        token_repo_login,
        token_repo_link,
        refresh_token_repo,
        identity_repo,
        signing_key_registry,
        platform_issuer,
    }
}

fn configured_http_verifier(
    auth: iam_configuration::AuthConfig,
    seed: Option<LocalJwksSeed>,
) -> Result<UserIdExtractor> {
    let extractor = if auth.jwt.allowed_algorithms.iter().any(|a| a == "RS256") {
        let seed =
            seed.ok_or_else(|| anyhow::anyhow!("RS256 verifier requires a canonical local seed"))?;
        UserIdExtractor::from_config_with_seeded_jwks(auth, seed)
    } else {
        // This is algorithm selection, never a fallback from a failed seed/URL.
        UserIdExtractor::new(auth)
    };
    extractor.map_err(|e| anyhow::anyhow!("Invalid auth configuration: {e}"))
}

/// The callback may resolve private storage. Policy must precede it even when
/// malformed/mixed verifier algorithms will subsequently be rejected.
fn signing_auth_after_policy<F>(
    config: &AppConfig,
    build: F,
) -> Result<iam_configuration::AuthConfig>
where
    F: FnOnce() -> Result<iam_configuration::AuthConfig>,
{
    config
        .jwt
        .validate_signing_security(config.security.mode)
        .map_err(|e| anyhow::anyhow!("Invalid signing provider configuration: {e}"))?;
    build()
}

/// Delegation is selected from validated Transit/Remote configuration, never
/// from a failed local read. Keep private resolution out of that path entirely.
fn configured_local_algorithm(
    config: &AppConfig,
    delegated: bool,
) -> Result<Option<iam_configuration::JwtAlgorithm>> {
    if delegated {
        return Ok(None);
    }
    Ok(Some(config.jwt.create_jwt_algorithm().map_err(|e| {
        tracing::error!("Failed to create JWT algorithm from configuration: {e}");
        anyhow::anyhow!("Failed to create JWT algorithm from configuration: {e}")
    })?))
}

fn setup_signer_route_context(
    config: &AppConfig,
    db: Arc<sea_orm::DatabaseConnection>,
    registry: Arc<SeaOrmSigningKeyRegistry>,
    pem_root: PathBuf,
    transit: Option<&TransitClientConfig>,
) -> Arc<SignerRouteContext> {
    let identity_repo = Arc::new(SeaOrmIdentityRepository::new(db));
    let mut probe = DefaultOrganizationSignerProbe::new(pem_root.clone())
        .with_local_pem_allowed(config.security.mode.allows_local_pem());
    if let Some(t) = transit {
        probe = probe.with_transit(t.clone());
    }
    let rotator = Arc::new(DefaultOrganizationSignerRotator::new(RotateContext {
        registry: registry.clone()
            as Arc<dyn iam_domain::port::repository::SigningKeyRegistry<Error = DomainError>>,
        pem_root: pem_root.clone(),
        transit: transit.cloned(),
        allow_local_pem: config.security.mode.allows_local_pem(),
    }));
    Arc::new(SignerRouteContext::new(SignerRouteContextParams {
        registry: registry
            as Arc<dyn iam_domain::port::repository::SigningKeyRegistry<Error = DomainError>>,
        identity_repo: identity_repo
            as Arc<dyn iam_domain::port::repository::IdentityRepository<Error = DomainError>>,
        public_base_url: config.jwt.public_base_url.clone(),
        probe: Arc::new(probe),
        rotator: rotator as Arc<dyn iam_domain::port::OrganizationSignerRotator>,
        pem_root,
        expiration_seconds: config.jwt.expiration_seconds,
        skew_seconds: JWKS_RETIRE_SKEW_SECONDS as u64,
        transit_base_url: transit.map(|t| t.base_url.clone()),
    }))
}

async fn resolve_access_signing(
    config: &AppConfig,
    registry: &SeaOrmSigningKeyRegistry,
    platform_issuer: &str,
    pem_root: &Path,
    transit: Option<&TransitClientConfig>,
    jwt_algorithm_config: Option<iam_configuration::JwtAlgorithm>,
) -> Result<(
    JwtAlgorithm,
    Option<(
        Arc<dyn iam_domain::port::SigningProvider>,
        String,
        String,
        iam_domain::entity::token::JwkSet,
    )>,
)> {
    let platform_transit = config
        .jwt
        .platform_transit_binding()
        .map_err(|e| anyhow::anyhow!("Invalid platform Transit binding: {e}"))?;
    let remote_cfg = config
        .jwt
        .remote_http_endpoint()
        .map_err(|e| anyhow::anyhow!("remote signer: {e}"))?;
    Ok(if let Some(binding) = platform_transit {
        let transit =
            transit.ok_or_else(|| anyhow::anyhow!("platform Transit unavailable; no fallback"))?;
        let bits =
            platform_transit_signing_bits(registry, platform_issuer, binding, transit, pem_root)
                .await?;
        let key = registry
            .find_active_platform_key()
            .await?
            .ok_or_else(|| anyhow::anyhow!("platform Active unavailable"))?;
        (
            JwtAlgorithm::RS256(iam_domain::entity::token::JwtKeyPair {
                private_key: String::new(),
                public_key: key.public_key,
                kid: key.kid,
            }),
            Some(bits),
        )
    } else if let Some(remote) = remote_cfg {
        let bits = remote_signing_bits(config, registry, platform_issuer, remote).await?;
        let key = registry
            .find_active_platform_key()
            .await?
            .ok_or_else(|| anyhow::anyhow!("remote Active unavailable"))?;
        (
            JwtAlgorithm::RS256(iam_domain::entity::token::JwtKeyPair {
                private_key: String::new(),
                public_key: key.public_key,
                kid: key.kid,
            }),
            Some(bits),
        )
    } else {
        resolve_local_access_signing(
            config,
            registry,
            platform_issuer,
            jwt_algorithm_config,
            remote_cfg.is_some(),
        )
        .await?
    })
}

async fn resolve_local_access_signing(
    config: &AppConfig,
    registry: &SeaOrmSigningKeyRegistry,
    platform_issuer: &str,
    jwt_algorithm_config: Option<iam_configuration::JwtAlgorithm>,
    remote_already_selected: bool,
) -> Result<(
    JwtAlgorithm,
    Option<(
        Arc<dyn iam_domain::port::SigningProvider>,
        String,
        String,
        iam_domain::entity::token::JwkSet,
    )>,
)> {
    Ok(
        match jwt_algorithm_config
            .ok_or_else(|| anyhow::anyhow!("signing algorithm unavailable"))?
        {
            iam_configuration::JwtAlgorithm::HS256(secret) => {
                tracing::warn!("HS256 JWT algorithm configured — access tokens should be RS256");
                (
                    JwtAlgorithm::HS256(secret),
                    None::<(
                        Arc<dyn iam_domain::port::SigningProvider>,
                        String,
                        String,
                        iam_domain::entity::token::JwkSet,
                    )>,
                )
            }
            iam_configuration::JwtAlgorithm::RS256(key_pair) => {
                anyhow::ensure!(
                    config.security.mode.allows_local_pem(),
                    "local PEM signer requires explicit nonprod mode; no provider fallback"
                );
                tracing::info!(
                    "Using RSA256 JWT algorithm (key_id: {}, private_key: {} bytes, public_key: {} bytes)",
                    key_pair.kid,
                    key_pair.private_key.len(),
                    key_pair.public_key.len()
                );
                let jwt_algorithm = JwtAlgorithm::RS256(iam_domain::entity::token::JwtKeyPair {
                    private_key: key_pair.private_key.clone(),
                    public_key: key_pair.public_key.clone(),
                    kid: key_pair.kid.clone(),
                });
                if remote_already_selected {
                    tracing::info!(
                        "Remote access signer requested — skipping PEM access-signer bootstrap"
                    );
                    (jwt_algorithm, None)
                } else {
                    let provider = Arc::new(
                        PemSigningProvider::new(&key_pair.private_key, key_pair.public_key.clone())
                            .map_err(|e| anyhow::anyhow!("PEM signing provider: {e}"))?,
                    )
                        as Arc<dyn iam_domain::port::SigningProvider>;
                    let provider: Arc<dyn SigningProvider> =
                        Arc::new(ScopedSigningProvider::unversioned(
                            SigningProviderType::PemFile,
                            "pem:config/jwt.secret".into(),
                            None,
                            provider,
                        )?);

                    let bootstrapped = bootstrap_platform_signing_key(
                        registry,
                        &key_pair.kid,
                        platform_issuer,
                        &key_pair.public_key,
                        "pem:config/jwt.secret",
                        SigningProviderType::PemFile,
                    )
                    .await
                    .map_err(|e| anyhow::anyhow!("signing key bootstrap: {e}"))?;

                    let mut jwk = JwtTokenService::jwk_from_pem(
                        &bootstrapped.public_key,
                        &bootstrapped.kid,
                        &bootstrapped.issuer,
                    )
                    .map_err(|e| anyhow::anyhow!("JWKS build: {e}"))?;
                    jwk.status = Some(bootstrapped.status.clone());
                    jwk.trust_scope = Some(bootstrapped.trust_scope.clone());
                    jwk.organization_id = bootstrapped.organization_id;
                    let jwks = iam_domain::entity::token::JwkSet { keys: vec![jwk] };

                    (
                        jwt_algorithm,
                        Some((provider, bootstrapped.kid, bootstrapped.issuer, jwks)),
                    )
                }
            }
        },
    )
}

async fn setup_jwt(
    config: &AppConfig,
    db: Arc<sea_orm::DatabaseConnection>,
    registry: Arc<SeaOrmSigningKeyRegistry>,
) -> Result<(
    iam_configuration::AuthConfig,
    String,
    Arc<JwtTokenService>,
    Arc<iam_infra::token::RegistrationTokenServiceImpl>,
    Option<Arc<SignerRouteContext>>,
)> {
    let mut http_verifier_auth = signing_auth_after_policy(config, || {
        config.jwt.http_verifier_auth().map_err(|e| {
            tracing::error!("JWT verifier config invalid: {e}");
            anyhow::anyhow!("JWT verifier config invalid: {e}")
        })
    })?;
    http_verifier_auth.mesh = config.auth.mesh.clone();
    let platform_transit = config
        .jwt
        .platform_transit_binding()
        .map_err(|e| anyhow::anyhow!("Invalid platform Transit binding: {e}"))?;
    let remote_cfg = config
        .jwt
        .remote_http_endpoint()
        .map_err(|e| anyhow::anyhow!("remote signer: {e}"))?;
    tracing::info!("Setting up JWT token service");
    // Never resolve/read legacy PEM storage for a delegated provider.
    let jwt_algorithm_config =
        configured_local_algorithm(config, platform_transit.is_some() || remote_cfg.is_some())?;

    let platform_issuer = config.jwt.platform_issuer();
    let pem_root = organization_signer_pem_root(&config.jwt.secret);
    let transit = resolve_transit_client_config(&config.jwt)?;
    if config.security.mode == SecurityMode::Verified
        && transit
            .as_ref()
            .is_some_and(|t| !t.base_url.starts_with("https://"))
    {
        return Err(anyhow::anyhow!("verified Transit requires HTTPS"));
    }
    let signer_ctx = Some(setup_signer_route_context(
        config,
        db,
        registry.clone(),
        pem_root.clone(),
        transit.as_ref(),
    ));
    let (jwt_algorithm, signing_bits) = resolve_access_signing(
        config,
        registry.as_ref(),
        &platform_issuer,
        &pem_root,
        transit.as_ref(),
        jwt_algorithm_config,
    )
    .await?;
    let _ = (platform_transit, remote_cfg);

    let mut token_service = JwtTokenService::with_refresh_expiration(
        jwt_algorithm,
        config.jwt.expiration_seconds,
        config.jwt.refresh_token_expiration_seconds,
    )
    .with_issuer_audience(platform_issuer.clone(), config.jwt.audience.clone());
    token_service = token_service.with_signing_registry(registry.clone());
    token_service = token_service.with_local_pem_allowed(config.security.mode.allows_local_pem());

    let inline_jwks = if let Some((provider, kid, issuer, jwks)) = signing_bits {
        let json =
            serde_json::to_string(&jwks).map_err(|e| anyhow::anyhow!("JWKS serialize: {e}"))?;
        token_service = token_service.with_signing_provider(provider, kid, issuer, jwks);
        json
    } else {
        use iam_domain::port::service::JwtTokenEncoder;
        serde_json::to_string(&JwtTokenEncoder::jwks(&token_service))
            .unwrap_or_else(|_| "{\"keys\":[]}".to_string())
    };

    let token_service = Arc::new(token_service);
    if !config.internal_service_token.is_empty() {
        iam_http_server::configure_internal_service_token(config.internal_service_token.clone());
    }
    let registration_token_service = Arc::new(
        iam_infra::token::RegistrationTokenServiceImpl::new(token_service.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create registration token service: {e}"))?,
    );
    Ok((
        http_verifier_auth,
        inline_jwks,
        token_service,
        registration_token_service,
        signer_ctx,
    ))
}

fn setup_oauth_clients(config: &AppConfig) -> Result<OauthClients> {
    let by_slug = setup_http_idp_clients(&config.idp, config.security.mode)?;
    Ok(OauthClients { by_slug })
}

fn require_platform_transit_binding(
    key: &iam_domain::entity::signing_key::SigningKey,
    issuer: &str,
    binding: &iam_configuration::PlatformTransitConfig,
) -> Result<()> {
    if key.issuer != issuer
        || key.provider_type != SigningProviderType::OpenBaoTransit
        || key.provider_key_ref != binding.provider_key_ref
        || key.credential_ref.as_deref() != Some(binding.credential_ref.as_str())
        || key.provider_key_version.is_none_or(|v| v == 0)
    {
        return Err(anyhow::anyhow!(
            "platform Transit binding incompatible; no bootstrap replacement"
        ));
    }
    Ok(())
}

fn require_platform_remote_binding(
    key: &iam_domain::entity::signing_key::SigningKey,
    candidate: &iam_domain::entity::signing_key::SigningKey,
) -> Result<()> {
    if key.kid != candidate.kid
        || !iam_domain::entity::signing_key::same_effective_signing_binding(key, candidate)
    {
        return Err(anyhow::anyhow!(
            "remote binding incompatible; no bootstrap replacement"
        ));
    }
    Ok(())
}

fn require_fresh_platform_scope(revision: u64) -> Result<()> {
    if revision != 0 {
        return Err(anyhow::anyhow!(
            "disabled platform scope requires explicit recovery; no automatic resurrection"
        ));
    }
    Ok(())
}

async fn platform_transit_signing_bits(
    registry: &SeaOrmSigningKeyRegistry,
    issuer: &str,
    binding: &iam_configuration::PlatformTransitConfig,
    transit: &TransitClientConfig,
    _pem_root: &Path,
) -> Result<(
    Arc<dyn SigningProvider>,
    String,
    String,
    iam_domain::entity::token::JwkSet,
)> {
    use iam_domain::entity::signing_key::{
        SigningKey, SigningKeyPreparation, SigningKeyStatus, SigningScope, opaque_kid,
    };
    let before = registry
        .signing_scope_snapshot(&SigningScope::platform())
        .await?;
    let delegate: Arc<dyn SigningProvider> = Arc::new(ScopedSigningProvider::transit(
        transit.clone(),
        binding.provider_key_ref.clone(),
        binding.credential_ref.clone(),
    )?);
    let probe = ProviderBindingProbe(delegate.clone());
    let key = if let Some(active) = &before.active {
        require_platform_transit_binding(active, issuer, binding)?;
        probe.challenge(active).await?;
        if !registry.confirm_active_for_emission(active).await? {
            return Err(anyhow::anyhow!("platform binding changed during probe"));
        }
        active.clone()
    } else if let Some(pending) = &before.pending {
        require_platform_transit_binding(&pending.key, issuer, binding)?;
        registry.promote_signing_key(pending).await?
    } else {
        require_fresh_platform_scope(before.revision)?;
        let client = TransitSigningProvider::new(
            transit.base_url.clone(),
            &binding.provider_key_ref,
            binding.provider_key_version,
            &binding.credential_ref,
            transit.workload.clone(),
            None,
        )?;
        let public_key = client.enrollment_public_key().await?;
        let now = chrono::Utc::now();
        let candidate = SigningKey {
            id: Uuid::new_v4(),
            kid: opaque_kid(),
            algorithm: "RS256".into(),
            trust_scope: iam_domain::entity::signing_key::TrustScope::Platform,
            issuer: issuer.into(),
            provider_type: SigningProviderType::OpenBaoTransit,
            provider_key_ref: binding.provider_key_ref.clone(),
            provider_key_version: Some(binding.provider_key_version),
            credential_ref: Some(binding.credential_ref.clone()),
            public_key,
            status: SigningKeyStatus::Pending,
            organization_id: None,
            created_at: now,
            updated_at: now,
        };
        let proof = probe.prove(candidate, before).await?;
        match registry.prepare_signing_key(proof).await? {
            SigningKeyPreparation::Unchanged(key) => key,
            SigningKeyPreparation::Pending(pending) => {
                registry.promote_signing_key(&pending).await?
            }
        }
    };
    let jwks =
        iam_domain::entity::token::JwkSet::from_registry_keys_checked(std::slice::from_ref(&key))?;
    Ok((delegate, key.kid, key.issuer, jwks))
}

async fn remote_signing_bits(
    config: &AppConfig,
    registry: &SeaOrmSigningKeyRegistry,
    platform_issuer: &str,
    remote: &iam_configuration::RemoteSignerConfig,
) -> Result<(
    Arc<dyn iam_domain::port::SigningProvider>,
    String,
    String,
    iam_domain::entity::token::JwkSet,
)> {
    use iam_domain::entity::signing_key::{
        SigningKey, SigningKeyPreparation, SigningKeyStatus, SigningScope,
    };
    let before = registry
        .signing_scope_snapshot(&SigningScope::platform())
        .await?;
    let static_secret = remote.token.as_deref().unwrap_or("");
    let workload =
        compose_workload_identity(config.jwt.workload.as_ref(), static_secret, "remote-signer")
            .map_err(|e| anyhow::anyhow!("jwt.workload / jwt.remote.token invalid: {e}"))?;
    let provider = RemoteSigningProvider::new(
        &remote.url,
        &remote.key_id,
        "RS256",
        "remote-signer",
        workload,
    )
    .map_err(|e| anyhow::anyhow!("remote signing provider: {e}"))?;
    let public_key = provider
        .public_key()
        .await
        .map_err(|e| anyhow::anyhow!("remote public key: {e}"))?;
    let provider: Arc<dyn SigningProvider> = Arc::new(ScopedSigningProvider::unversioned(
        SigningProviderType::RemoteHttp,
        "remote:jwt.remote".into(),
        None,
        Arc::new(provider),
    )?);
    let probe = ProviderBindingProbe(provider.clone());
    let now = chrono::Utc::now();
    let candidate = SigningKey {
        id: Uuid::new_v4(),
        kid: remote.key_id.clone(),
        algorithm: "RS256".into(),
        trust_scope: iam_domain::entity::signing_key::TrustScope::Platform,
        issuer: platform_issuer.into(),
        provider_type: SigningProviderType::RemoteHttp,
        provider_key_ref: "remote:jwt.remote".into(),
        provider_key_version: None,
        credential_ref: None,
        public_key,
        status: SigningKeyStatus::Pending,
        organization_id: None,
        created_at: now,
        updated_at: now,
    };
    let bootstrapped = if let Some(active) = &before.active {
        require_platform_remote_binding(active, &candidate)?;
        probe.challenge(active).await?;
        if !registry.confirm_active_for_emission(active).await? {
            return Err(anyhow::anyhow!("remote binding changed during probe"));
        }
        active.clone()
    } else if let Some(pending) = &before.pending {
        require_platform_remote_binding(&pending.key, &candidate)?;
        registry.promote_signing_key(pending).await?
    } else {
        require_fresh_platform_scope(before.revision)?;
        match registry
            .prepare_signing_key(probe.prove(candidate, before).await?)
            .await?
        {
            SigningKeyPreparation::Unchanged(key) => key,
            SigningKeyPreparation::Pending(pending) => {
                registry.promote_signing_key(&pending).await?
            }
        }
    };
    let mut jwk = JwtTokenService::jwk_from_pem(
        &bootstrapped.public_key,
        &bootstrapped.kid,
        &bootstrapped.issuer,
    )
    .map_err(|e| anyhow::anyhow!("JWKS build: {e}"))?;
    jwk.status = Some(bootstrapped.status.clone());
    jwk.trust_scope = Some(bootstrapped.trust_scope.clone());
    jwk.organization_id = bootstrapped.organization_id;
    let jwks = iam_domain::entity::token::JwkSet { keys: vec![jwk] };
    Ok((provider, bootstrapped.kid, bootstrapped.issuer, jwks))
}

fn organization_signer_pem_root(secret: &SecretStorage) -> PathBuf {
    match secret {
        SecretStorage::PemFile {
            private_key_path, ..
        } => Path::new(private_key_path)
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map_or_else(
                || PathBuf::from("config/keys/organizations"),
                |parent| parent.join("organizations"),
            ),
        _ => PathBuf::from("config/keys/organizations"),
    }
}

/// Resolve Transit HTTP client config: WIF when configured, else static token.
///
/// Static token remains optional skip (no Transit) when WIF is absent — matching
/// prior `transit_endpoint()` behaviour. WIF incomplete → fail-closed at boot.
fn resolve_transit_client_config(
    jwt: &iam_configuration::JwtConfig,
) -> Result<Option<TransitClientConfig>> {
    let url = jwt
        .transit_url
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .map(str::to_string)
        .or_else(|| {
            if let SecretStorage::Vault { url, .. } = &jwt.secret {
                let u = url.trim();
                if u.is_empty() {
                    None
                } else {
                    Some(u.to_string())
                }
            } else {
                None
            }
        });
    let Some(url) = url else {
        return Ok(None);
    };

    let provider = jwt
        .workload
        .as_ref()
        .and_then(|w| w.provider.as_deref())
        .unwrap_or("static")
        .trim()
        .to_ascii_lowercase();
    let is_wif = matches!(provider.as_str(), "aws" | "gcp" | "azure");

    let mut static_secret = jwt
        .transit_token
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    if static_secret.is_empty()
        && let SecretStorage::Vault { token, .. } = &jwt.secret
    {
        static_secret = token.trim().to_string();
    }

    if !is_wif && static_secret.is_empty() {
        // Preserve prior skip when Transit URL is set without a static token.
        return Ok(None);
    }

    let token_ref = jwt
        .platform_transit_binding()
        .map_err(|e| anyhow::anyhow!("platform Transit binding: {e}"))?
        .map_or("openbao-token", |binding| binding.credential_ref.as_str());
    let workload = compose_workload_identity(jwt.workload.as_ref(), &static_secret, token_ref)
        .map_err(|e| anyhow::anyhow!("jwt.workload / transit_token invalid: {e}"))?;

    Ok(Some(TransitClientConfig {
        base_url: url,
        workload,
        token_ref: token_ref.to_string(),
    }))
}

fn setup_http_idp_clients(
    idp: &IdpConfig,
    security_mode: SecurityMode,
) -> Result<HashMap<Provider, Arc<dyn FederatedOAuthClient>>> {
    idp.validate().map_err(DomainError::OAuth2Error)?;
    let mut by_slug = HashMap::new();
    for connector in &idp.connectors {
        let provider = Provider::parse_slug(&connector.id).map_err(|_| {
            DomainError::OAuth2Error(format!("illegal IdP connector id: {}", connector.id))
        })?;
        let client = HttpIdpConnector::with_security_mode(
            &connector.base_url,
            &connector.hmac_secret,
            security_mode,
        )?;
        by_slug.insert(provider, Arc::new(client) as Arc<dyn FederatedOAuthClient>);
    }
    Ok(by_slug)
}

/// Serve IAM HTTP/HTTPS until a shutdown signal or a background task fails.
///
/// # Errors
///
/// Returns an error if the HTTP server or outbox dispatcher fails.
pub async fn run_server(app: IAMRustyApp, app_config: ServerConfig) -> Result<()> {
    info!("Starting IAM service...");

    // Convert our ServerConfig to HttpServerConfig
    let server_config = app_config;

    // Start server (HTTP or HTTPS based on configuration)
    if server_config.tls_enabled {
        info!(
            "Starting HTTPS server on {}:{}",
            server_config.host, server_config.tls_port
        );
    } else {
        info!(
            "Starting HTTP server on {}:{}",
            server_config.host, server_config.port
        );
    }

    let mut server_handle = {
        let app_state = app.app_state.clone();
        let probe = app.readiness.clone();
        let idp = app.idp.clone();
        let signer = app.signer.clone();
        let security = app.http_security.clone();
        tokio::spawn(async move {
            create_app_routes(app_state, server_config, probe, idp, signer, security).await
        })
    };

    let mut background_tasks = app.start_background_tasks();
    let _abort_on_cancel = OwnedTaskAbortGuard(
        background_tasks
            .iter()
            .map(JoinHandle::abort_handle)
            .chain(std::iter::once(server_handle.abort_handle()))
            .collect(),
    );
    let mut server_finished = false;

    let result: Result<()> = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown signal received; stopping IAMRusty runtime");
            Ok(())
        }
        result = wait_for_background_failure(&mut background_tasks) => result,
        result = &mut server_handle => {
            server_finished = true;
            match result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) => Err(error),
                Err(_) => Err(anyhow::anyhow!("IAMRusty HTTP server task join failed")),
            }
        }
    };

    let cleanup_result = app.shutdown_background_tasks(&mut background_tasks).await;
    if !server_finished {
        server_handle.abort();
        let _ = server_handle.await;
    }
    result.and(cleanup_result)
}

struct OwnedTaskAbortGuard(Vec<tokio::task::AbortHandle>);

impl Drop for OwnedTaskAbortGuard {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}

/// Observe every IAM task without transferring ownership to a disposable wait future.
///
/// # Errors
///
/// Returns an error if the task set is empty, a worker fails, or a worker joins with a panic.
pub async fn wait_for_background_failure(handles: &mut Vec<JoinHandle<Result<()>>>) -> Result<()> {
    if handles.is_empty() {
        return Err(anyhow::anyhow!("IAM background task set is empty"));
    }
    let (result, index, remaining) = select_all(handles.iter_mut()).await;
    drop(remaining);
    drop(handles.swap_remove(index));
    match result {
        Ok(Ok(())) => Err(anyhow::anyhow!("IAM background worker exited unexpectedly")),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(anyhow::anyhow!("IAM background worker join failed")),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        configured_http_verifier, configured_local_algorithm, organization_signer_pem_root,
        require_fresh_platform_scope, require_platform_remote_binding,
        require_platform_transit_binding, setup_http_idp_clients, signing_auth_after_policy,
    };
    use iam_configuration::{
        AppConfig, IdpConfig, IdpConnectorConfig, SecretStorage, SecurityMode,
    };
    use iam_domain::entity::provider::Provider;
    use iam_domain::entity::signing_key::SigningProviderType;
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    fn complete_connector(id: &str) -> IdpConnectorConfig {
        IdpConnectorConfig {
            pkce_supported: false,
            id: id.to_string(),
            base_url: format!("http://127.0.0.1:9/{id}-connect"),
            hmac_secret: "sixteen-bytes-ok".to_string(),
            redirect_uris: vec![
                format!("http://127.0.0.1:8080/iam/api/auth/{id}/callback"),
                format!("http://127.0.0.1:8080/iam/api/auth/{id}/relink-callback"),
            ],
        }
    }

    #[test]
    fn setup_wires_huggingface_complete_line() {
        let idp = IdpConfig {
            connectors: vec![complete_connector("huggingface")],
        };
        let by_slug = setup_http_idp_clients(&idp, iam_configuration::SecurityMode::IsolatedTest)
            .expect("complete huggingface line boots");
        let expected = Provider::parse_slug("huggingface").expect("slug");
        assert!(by_slug.contains_key(&expected));
        assert_eq!(by_slug.len(), 1);
    }

    #[test]
    fn setup_wires_github_and_gitlab() {
        let idp = IdpConfig {
            connectors: vec![complete_connector("github"), complete_connector("gitlab")],
        };
        let by_slug = setup_http_idp_clients(&idp, iam_configuration::SecurityMode::IsolatedTest)
            .expect("github+gitlab boot");
        assert!(by_slug.contains_key(&Provider::parse_slug("github").expect("github")));
        assert!(by_slug.contains_key(&Provider::parse_slug("gitlab").expect("gitlab")));
        assert_eq!(by_slug.len(), 2);
    }

    #[test]
    fn setup_rejects_illegal_id() {
        let idp = IdpConfig {
            connectors: vec![complete_connector("hugging-face")],
        };
        match setup_http_idp_clients(&idp, iam_configuration::SecurityMode::IsolatedTest) {
            Ok(_) => panic!("expected fail-closed boot"),
            Err(err) => assert!(
                err.to_string().contains("illegal IdP connector id"),
                "unexpected error: {err}"
            ),
        }
    }

    #[test]
    fn setup_rejects_short_hmac() {
        let mut connector = complete_connector("github");
        connector.hmac_secret = "fifteen-bytes!!".to_string();
        let idp = IdpConfig {
            connectors: vec![connector],
        };
        match setup_http_idp_clients(&idp, iam_configuration::SecurityMode::IsolatedTest) {
            Ok(_) => panic!("expected fail-closed boot"),
            Err(err) => assert!(
                err.to_string().contains("at least 16"),
                "unexpected error: {err}"
            ),
        }
    }

    #[test]
    fn setup_rejects_empty_registry() {
        match setup_http_idp_clients(
            &IdpConfig::default(),
            iam_configuration::SecurityMode::IsolatedTest,
        ) {
            Ok(_) => panic!("expected fail-closed boot"),
            Err(err) => assert!(
                err.to_string().contains("must not be empty"),
                "unexpected error: {err}"
            ),
        }
    }

    #[tokio::test]
    async fn boot_rejects_empty_oauth_state_secret_before_any_runtime_dependency() {
        for mode in [
            iam_configuration::SecurityMode::Verified,
            iam_configuration::SecurityMode::LocalInsecure,
            iam_configuration::SecurityMode::IsolatedTest,
        ] {
            let mut config = iam_configuration::AppConfig::default();
            config.security.mode = mode;
            config.jwt.oauth_state_secret.clear();
            config.idp = IdpConfig {
                connectors: vec![complete_connector("github")],
            };
            match Box::pin(super::build_app_state(config, None)).await {
                Ok(_) => panic!("empty OAuth state secret must fail closed at boot"),
                Err(error) => assert!(
                    error.to_string().contains("OAuth state"),
                    "must fail secret validation, not queue/database/network setup: {error}"
                ),
            }
        }
    }

    #[test]
    fn delegated_signing_configuration_skips_even_unreadable_private_pem_material() {
        let mut config = iam_configuration::AppConfig::default();
        config.security.mode = iam_configuration::SecurityMode::Verified;
        config.jwt.backend = Some("transit".into());
        config.jwt.transit_url = Some("https://bao.example".into());
        config.jwt.platform_transit = Some(iam_configuration::PlatformTransitConfig {
            provider_key_ref: "platform".into(),
            provider_key_version: 7,
            credential_ref: "platform-credential".into(),
        });
        let absent = std::env::temp_dir().join(format!(
            "iam-no-private-resolution-{}",
            uuid::Uuid::new_v4()
        ));
        config.jwt.secret = SecretStorage::PemFile {
            private_key_path: absent.join("private.pem").to_string_lossy().into_owned(),
            public_key_path: absent.join("public.pem").to_string_lossy().into_owned(),
            key_id: None,
        };
        config.jwt.allowed_algorithms = vec!["RS256".into()];
        config
            .jwt
            .validate_signing_security(config.security.mode)
            .unwrap();
        assert!(configured_local_algorithm(&config, true).unwrap().is_none());
        assert!(
            config.jwt.create_jwt_algorithm().is_err(),
            "the unused local pair is actually unreadable"
        );
        assert!(!absent.exists());
    }

    #[test]
    fn verified_mixed_pem_is_rejected_before_the_verifier_resolution_callback() {
        let mut config = AppConfig::default();
        config.security.mode = SecurityMode::Verified;
        config.jwt.secret = SecretStorage::PemFile {
            private_key_path: "must-not-read/private.pem".into(),
            public_key_path: "must-not-read/public.pem".into(),
            key_id: None,
        };
        config.jwt.allowed_algorithms = vec!["RS256".into(), "HS256".into()];
        let invoked = std::cell::Cell::new(false);
        let result = signing_auth_after_policy(&config, || {
            invoked.set(true);
            Ok(iam_configuration::AuthConfig::default())
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("private PEM provider forbidden")
        );
        assert!(
            !invoked.get(),
            "policy denial must precede even an attempted private resolution"
        );
        config.jwt.backend = Some("transit".into());
        config.jwt.transit_url = Some("https://bao.example".into());
        config.jwt.platform_transit = Some(iam_configuration::PlatformTransitConfig {
            provider_key_ref: "platform".into(),
            provider_key_version: 7,
            credential_ref: "platform-credential".into(),
        });
        config.jwt.allowed_algorithms = vec!["RS256".into()];
        signing_auth_after_policy(&config, || {
            invoked.set(true);
            config.jwt.http_verifier_auth().map_err(Into::into)
        })
        .unwrap();
        assert!(
            invoked.get(),
            "valid delegated RS256 retains the real verifier helper without PEM reads"
        );
    }

    #[test]
    fn delegated_pem_hmac_is_rejected_before_resolution_but_independent_hmac_is_preserved() {
        for backend in ["transit", "remote"] {
            let mut config = AppConfig::default();
            config.security.mode = SecurityMode::Verified;
            config.jwt.backend = Some(backend.into());
            if backend == "transit" {
                config.jwt.transit_url = Some("https://bao.example".into());
                config.jwt.platform_transit = Some(iam_configuration::PlatformTransitConfig {
                    provider_key_ref: "platform".into(),
                    provider_key_version: 7,
                    credential_ref: "platform-credential".into(),
                });
            } else {
                config.jwt.remote = Some(iam_configuration::RemoteSignerConfig {
                    url: "https://remote.example".into(),
                    key_id: "platform".into(),
                    token: None,
                });
            }
            config.jwt.secret = SecretStorage::PemFile {
                private_key_path: "must-not-read/private.pem".into(),
                public_key_path: "must-not-read/public.pem".into(),
                key_id: None,
            };
            config.jwt.allowed_algorithms = vec!["RS256".into(), "HS256".into()];
            let calls = std::cell::Cell::new(0);
            let result = signing_auth_after_policy(&config, || {
                calls.set(calls.get() + 1);
                config.jwt.http_verifier_auth().map_err(Into::into)
            });
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("invalid delegated signing secret configuration")
            );
            assert_eq!(
                calls.get(),
                0,
                "neither delegated backend may enter the PEM/HMAC resolver"
            );
            // D-1R-N: the delegated guard normalizes algorithm names (trim +
            // ASCII case), so padded or lowercase HS256 aliases are rejected
            // before any callback with the unreadable PEM still configured.
            for alias in [" HS256 ", "hs256", "\tHS256"] {
                config.jwt.allowed_algorithms = vec!["RS256".into(), alias.into()];
                let alias_calls = std::cell::Cell::new(0);
                let alias_result = signing_auth_after_policy(&config, || {
                    alias_calls.set(alias_calls.get() + 1);
                    config.jwt.http_verifier_auth().map_err(Into::into)
                });
                assert!(
                    alias_result
                        .unwrap_err()
                        .to_string()
                        .contains("invalid delegated signing secret configuration"),
                    "padded/lowercase HS256 alias must hit the delegated guard: {alias:?}"
                );
                assert_eq!(
                    alias_calls.get(),
                    0,
                    "normalized HS256 aliases must not enter the PEM/HMAC resolver: {alias:?}"
                );
            }
            config.jwt.allowed_algorithms = vec!["RS256".into()];
            let auth = signing_auth_after_policy(&config, || {
                calls.set(calls.get() + 1);
                config.jwt.http_verifier_auth().map_err(Into::into)
            })
            .unwrap();
            assert_eq!(calls.get(), 1);
            assert!(auth.jwt.hs256_secret.is_none());
            assert_eq!(auth.jwt.issuer, Some(config.jwt.platform_issuer()));
            config.jwt.allowed_algorithms = vec!["RS256".into(), "HS256".into()];
            config.jwt.issuer = "independent-legacy-HMAC-issuer".into();
            config.jwt.secret = SecretStorage::PlainText {
                value: "explicit-nonproduction-HMAC-fixture".into(),
            };
            let auth = signing_auth_after_policy(&config, || {
                config.jwt.http_verifier_auth().map_err(Into::into)
            })
            .unwrap();
            assert_eq!(
                auth.jwt.hs256_secret.as_deref(),
                Some("explicit-nonproduction-HMAC-fixture")
            );
            assert_eq!(
                auth.jwt.issuer.as_deref(),
                Some("independent-legacy-HMAC-issuer")
            );
            assert_eq!(auth.jwt.allowed_algorithms, vec!["RS256", "HS256"]);
            // This helper preserves config semantics, NOT delivery of distinct
            // mixed issuers through the still-pinned SDK constructor.
        }
    }

    #[test]
    fn platform_transit_and_remote_boot_guards_refuse_replacement_and_disabled_resurrection() {
        use iam_domain::entity::signing_key::{SigningKey, SigningKeyStatus, TrustScope};
        let now = chrono::Utc::now();
        let mut key = SigningKey {
            id: Uuid::new_v4(),
            kid: iam_domain::entity::signing_key::opaque_kid(),
            algorithm: "RS256".into(),
            trust_scope: TrustScope::Platform,
            issuer: "https://platform.example/iam".into(),
            provider_type: SigningProviderType::OpenBaoTransit,
            provider_key_ref: "platform".into(),
            provider_key_version: Some(7),
            credential_ref: Some("platform-credential".into()),
            public_key: include_str!("../../config/keys/test-platform.pub").into(),
            status: SigningKeyStatus::Active,
            organization_id: None,
            created_at: now,
            updated_at: now,
        };
        let binding = iam_configuration::PlatformTransitConfig {
            provider_key_ref: key.provider_key_ref.clone(),
            provider_key_version: 7,
            credential_ref: "platform-credential".into(),
        };
        require_platform_transit_binding(&key, &key.issuer, &binding).unwrap();
        // A positive persisted successor is authoritative after explicit rotate;
        // a stale boot version is NOT allowed to replace it with its old pin.
        let mut successor = key.clone();
        successor.provider_key_version = Some(8);
        require_platform_transit_binding(&successor, &key.issuer, &binding).unwrap();
        for case in 0..6 {
            let mut incompatible = key.clone();
            match case {
                0 => incompatible.issuer.push_str("/other"),
                1 => incompatible.provider_key_ref.push_str("-other"),
                2 => incompatible.credential_ref = Some("other".into()),
                3 => incompatible.provider_key_version = None,
                4 => incompatible.provider_key_version = Some(0),
                _ => incompatible.provider_type = SigningProviderType::PemFile,
            }
            assert!(
                require_platform_transit_binding(&incompatible, &key.issuer, &binding)
                    .unwrap_err()
                    .to_string()
                    .contains("no bootstrap replacement")
            );
            assert_eq!(key.provider_key_version, Some(7));
        }
        key.provider_type = SigningProviderType::RemoteHttp;
        key.provider_key_ref = "remote:jwt.remote".into();
        key.provider_key_version = None;
        key.credential_ref = None;
        let mut candidate = key.clone();
        candidate.status = SigningKeyStatus::Pending;
        require_platform_remote_binding(&key, &candidate).unwrap();
        for case in 0..5 {
            let mut incompatible = candidate.clone();
            match case {
                0 => incompatible.kid = iam_domain::entity::signing_key::opaque_kid(),
                1 => incompatible.issuer.push_str("/other"),
                2 => incompatible.provider_key_ref.push_str("-other"),
                3 => incompatible.public_key = "invalid".into(),
                _ => incompatible.credential_ref = Some("other".into()),
            }
            assert!(
                require_platform_remote_binding(&key, &incompatible)
                    .unwrap_err()
                    .to_string()
                    .contains("no bootstrap replacement")
            );
        }
        require_fresh_platform_scope(0).unwrap();
        for revision in [1, 2, u64::MAX] {
            assert!(
                require_fresh_platform_scope(revision)
                    .unwrap_err()
                    .to_string()
                    .contains("disabled platform scope")
            );
        }
    }

    #[test]
    fn hs256_only_verifier_preserves_legacy_without_a_jwks_url_or_seed() {
        let jwt = iam_configuration::JwtConfig {
            secret: SecretStorage::PlainText {
                value: "explicit-legacy-hmac-secret".into(),
            },
            issuer: "independent-legacy-hmac-issuer".into(),
            public_base_url: String::new(),
            jwks_url: None,
            allowed_algorithms: vec!["HS256".into()],
            ..iam_configuration::JwtConfig::default()
        };
        let mut auth = jwt.http_verifier_auth().expect("legacy IAM config");
        assert_eq!(auth.jwt.allowed_algorithms, vec!["HS256"]);
        assert!(auth.jwt.jwks_url.is_none());
        assert_eq!(
            auth.jwt.issuer.as_deref(),
            Some("independent-legacy-hmac-issuer")
        );
        assert_eq!(
            auth.jwt.hs256_secret.as_deref(),
            Some("explicit-legacy-hmac-secret")
        );
        auth.mesh.trusted_gateway_san = "spiffe://fixture/mesh".into();
        let extractor = configured_http_verifier(auth, None).expect("HS256-only remains supported");
        assert_eq!(extractor.gateway_san(), Some("spiffe://fixture/mesh"));
    }

    #[test]
    fn rs256_verifier_never_falls_back_when_its_seed_is_absent() {
        let jwt = iam_configuration::JwtConfig {
            allowed_algorithms: vec!["RS256".into()],
            ..iam_configuration::JwtConfig::default()
        };
        let auth = jwt.http_verifier_auth().expect("trusted RS256 config");
        match configured_http_verifier(auth, None) {
            Ok(_) => panic!("RS256 seed absence must not select another constructor"),
            Err(error) => assert!(error.to_string().contains("canonical local seed")),
        }
    }

    #[test]
    fn organization_signer_pem_root_excludes_platform_pem() {
        let secret = SecretStorage::PemFile {
            private_key_path: "config/keys/test-platform.pem".to_string(),
            public_key_path: "config/keys/test-platform.pub".to_string(),
            key_id: Some("test-platform".to_string()),
        };
        let root = organization_signer_pem_root(&secret);
        let platform = Path::new("config/keys/test-platform.pem");
        assert!(!platform.starts_with(&root));
        assert_eq!(root, PathBuf::from("config/keys/organizations"));
    }
}
