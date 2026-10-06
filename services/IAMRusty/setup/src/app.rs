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
        RemoteSigningProvider, RotateContext, TransitClientConfig, compose_workload_identity,
    },
    token::{JwtAlgorithm, JwtTokenService},
    transaction::IamOutboxUnitOfWorkImpl,
};
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};

use iam_configuration::{AppConfig, IdpConfig, SecretStorage, SecurityMode};
use iam_domain::entity::provider::Provider;
use iam_domain::entity::signing_key::{JWKS_RETIRE_SKEW_SECONDS, SigningProviderType};
use iam_domain::error::DomainError;
use iam_domain::port::SigningProvider;
use iam_domain::port::repository::{AuthenticationSessionWriter, OAuthTransactionWriteRepository};
use iam_domain::port::service::FederatedOAuthClient;
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

impl IAMRustyApp {
    pub fn new(
        app_state: AppState,
        outbox_dispatcher: Arc<OutboxDispatcher<DomainError>>,
        readiness: Arc<ReadinessProbe>,
        idp: Arc<IdpConfig>,
        signer: Option<Arc<SignerRouteContext>>,
        signing_registry: Arc<SeaOrmSigningKeyRegistry>,
        access_token_ttl: u64,
        jwt_codec: Arc<JwtTokenService>,
        security: Arc<IamHttpSecurityContext>,
        oauth_cleanup: Arc<OAuthTransactionCleanup>,
    ) -> Self {
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
                self.http_security.clone(),
            ),
            self.readiness.clone(),
        )
    }

    /// The root-created codec shared by access and completion issuance.
    /// No private key material or HTTP extension is exposed by this accessor.
    pub fn jwt_codec(&self) -> Arc<JwtTokenService> {
        self.jwt_codec.clone()
    }

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

    /// Organization-signer application façade (ADR-0306 InProcess injection).
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

    pub async fn stop_background_tasks(&self) -> Result<()> {
        self.oauth_cleanup_stop.send_replace(true);
        self.outbox_dispatcher
            .stop()
            .await
            .map_err(|_| anyhow::anyhow!("Failed to stop IAMRusty outbox dispatcher"))
    }

    /// Cooperatively stop, then join every owned IAM handle under one five-second budget.
    /// Timed-out handles are aborted and awaited; no worker handle is discarded.
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
            match joined {
                Ok((result, index, remaining)) => {
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
                }
                Err(_) => {
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
    let app_state = build_app_state(config.clone(), maybe_event_publisher).await?;
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
    iam_http_server::configure_oauth_state_secret(oauth_state_secret)
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

/// Isolated fixtures seed through the SAME root-created admission writer before bootstrap.
/// Normal production construction supplies no fixture keys.
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
    if !signing_keys.is_empty() {
        anyhow::ensure!(
            config.security.mode == iam_configuration::security::SecurityMode::IsolatedTest,
            "initial fixture keys require explicit isolated_test mode"
        );
        // Validate the configured pair before any fixture Active insertion too.
        let iam_configuration::JwtAlgorithm::RS256(pair) = config.jwt.create_jwt_algorithm()?
        else {
            anyhow::bail!("initial signing fixture keys require RSA configuration");
        };
        PemSigningProvider::new(&pair.private_key, pair.public_key)?;
    }
    let oauth_state_secret = config
        .security
        .validate(&config.jwt.oauth_state_secret, &config.idp)
        .map_err(anyhow::Error::msg)?;
    iam_http_server::configure_oauth_state_secret(oauth_state_secret)
        .map_err(anyhow::Error::msg)?;
    info!("Building IAM service...");

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
    // Setup database connection pool
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
    let dispatcher_publisher: Arc<dyn EventPublisher<DomainError>> = event_publisher.clone();
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

    let repos = setup_repositories(
        &db_pool,
        config.jwt.platform_issuer(),
        signing_registry.clone(),
    );
    let idp = Arc::new(config.idp.clone());
    let oauth_clients = setup_oauth_clients(&config)?;

    // Create password service
    let password_service = Arc::new(PasswordService::new());
    let password_service_adapter = Arc::new(PasswordServiceAdapter::new(password_service.clone()));

    let (http_verifier_auth, _inline_jwks, token_service, registration_token_service, signer_ctx) =
        setup_jwt(&config, db_write.clone(), signing_registry.clone()).await?;

    let outbox_unit_of_work = Arc::new(IamOutboxUnitOfWorkImpl::new(
        db_pool.clone(),
        OutboxRecorder,
    ));
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
    let registry = CommandRegistryFactory::create_iam_registry(usecases, &config.command);
    let command_service = Arc::new(GenericCommandService::new(Arc::new(registry)));

    // The configured live publisher is authoritative, including organization
    // epochs and revocations. An inline-only bootstrap snapshot ignores the URL
    // and expires after 60 s without any way to refresh. Construction performs
    // no HTTP request; the first bearer is verified only after listen.
    let user_id_extractor = UserIdExtractor::new(http_verifier_auth)
        .map_err(|e| anyhow::anyhow!("Invalid auth configuration: {e}"))?;

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

    Ok(IAMRustyApp::new(
        app_state,
        outbox_dispatcher,
        readiness,
        idp,
        signer_ctx,
        signing_registry,
        config.jwt.expiration_seconds,
        token_service,
        http_security,
        oauth_cleanup,
    ))
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

fn setup_provider_user_token(
    db_pool: &DbConnectionPool,
    user_repo: UserRepo,
    user_email_repo: UserEmailRepo,
    refresh_token_repo: RefreshRepo,
    token_service: Arc<JwtTokenService>,
    signing_key_registry: Arc<
        dyn iam_domain::port::repository::SigningKeyRegistry<Error = DomainError>,
    >,
    identity_repo: Arc<dyn iam_domain::port::repository::IdentityRepository<Error = DomainError>>,
    platform_issuer: String,
    access_token_expiration_seconds: u64,
) -> (
    Arc<dyn ProviderUseCase>,
    Arc<dyn UserUseCase>,
    Arc<dyn TokenUseCase>,
) {
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
        user_repo,
        user_email_repo,
        refresh_token_repo,
        token_service.clone(),
        signing_key_registry,
        identity_repo,
        platform_issuer,
        token_service.access_token_expiration_seconds(),
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
    let mut http_verifier_auth = config.jwt.http_verifier_auth().map_err(|e| {
        tracing::error!("JWT verifier config invalid: {e}");
        anyhow::anyhow!("JWT verifier config invalid: {e}")
    })?;
    http_verifier_auth.mesh = config.auth.mesh.clone();
    tracing::info!("Setting up JWT token service");
    let jwt_algorithm_config = config.jwt.create_jwt_algorithm().map_err(|e| {
        tracing::error!("Failed to create JWT algorithm from configuration: {e}");
        anyhow::anyhow!("Failed to create JWT algorithm from configuration: {e}")
    })?;

    let platform_issuer = config.jwt.platform_issuer();
    let identity_repo = Arc::new(SeaOrmIdentityRepository::new(db));
    let pem_root = organization_signer_pem_root(&config.jwt.secret);
    let transit = resolve_transit_client_config(&config.jwt)?;
    let mut probe = DefaultOrganizationSignerProbe::new(pem_root.clone());
    if let Some(ref t) = transit {
        probe = probe.with_transit(t.clone());
    }
    let rotator = Arc::new(DefaultOrganizationSignerRotator::new(RotateContext {
        registry: registry.clone()
            as Arc<dyn iam_domain::port::repository::SigningKeyRegistry<Error = DomainError>>,
        pem_root: pem_root.clone(),
        transit: transit.clone(),
    }));
    let signer_ctx = Some(Arc::new(SignerRouteContext::new(
        registry.clone()
            as Arc<dyn iam_domain::port::repository::SigningKeyRegistry<Error = DomainError>>,
        identity_repo
            as Arc<dyn iam_domain::port::repository::IdentityRepository<Error = DomainError>>,
        config.jwt.public_base_url.clone(),
        Arc::new(probe),
        rotator as Arc<dyn iam_domain::port::OrganizationSignerRotator>,
        pem_root,
        config.jwt.expiration_seconds,
        JWKS_RETIRE_SKEW_SECONDS as u64,
        transit.map(|t| t.base_url),
    )));

    let remote_cfg = config
        .jwt
        .remote_http_endpoint()
        .map_err(|e| anyhow::anyhow!("remote signer: {e}"))?;

    let (jwt_algorithm, signing_bits) = match jwt_algorithm_config {
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
            // Remote access signer: skip PEM access-signer bootstrap (no mixed PEM+remote JWKS/kid).
            if remote_cfg.is_some() {
                tracing::info!(
                    "Remote access signer requested — skipping PEM access-signer bootstrap"
                );
                (jwt_algorithm, None)
            } else {
                let provider = Arc::new(
                    PemSigningProvider::new(&key_pair.private_key, key_pair.public_key.clone())
                        .map_err(|e| anyhow::anyhow!("PEM signing provider: {e}"))?,
                ) as Arc<dyn iam_domain::port::SigningProvider>;

                let bootstrapped = bootstrap_platform_signing_key(
                    registry.as_ref(),
                    &key_pair.kid,
                    &platform_issuer,
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
    };

    let signing_bits = if let Some(remote) = remote_cfg {
        Some(remote_signing_bits(config, registry.as_ref(), &platform_issuer, remote).await?)
    } else {
        signing_bits
    };

    let mut token_service = JwtTokenService::with_refresh_expiration(
        jwt_algorithm.clone(),
        config.jwt.expiration_seconds,
        config.jwt.refresh_token_expiration_seconds,
    )
    .with_issuer_audience(platform_issuer.clone(), config.jwt.audience.clone());
    token_service = token_service.with_signing_registry(registry.clone());

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
    let bootstrapped = bootstrap_platform_signing_key(
        registry,
        &remote.key_id,
        platform_issuer,
        &public_key,
        "remote:jwt.remote",
        SigningProviderType::RemoteHttp,
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
    Ok((
        Arc::new(provider) as Arc<dyn iam_domain::port::SigningProvider>,
        bootstrapped.kid,
        bootstrapped.issuer,
        jwks,
    ))
}

fn organization_signer_pem_root(secret: &SecretStorage) -> PathBuf {
    match secret {
        SecretStorage::PemFile {
            private_key_path, ..
        } => Path::new(private_key_path)
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| parent.join("organizations"))
            .unwrap_or_else(|| PathBuf::from("config/keys/organizations")),
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
    if static_secret.is_empty() {
        if let SecretStorage::Vault { token, .. } = &jwt.secret {
            static_secret = token.trim().to_string();
        }
    }

    if !is_wif && static_secret.is_empty() {
        // Preserve prior skip when Transit URL is set without a static token.
        return Ok(None);
    }

    let workload =
        compose_workload_identity(jwt.workload.as_ref(), &static_secret, "openbao-token")
            .map_err(|e| anyhow::anyhow!("jwt.workload / transit_token invalid: {e}"))?;

    Ok(Some(TransitClientConfig {
        base_url: url,
        workload,
        token_ref: "openbao-token".to_string(),
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
    use super::{organization_signer_pem_root, setup_http_idp_clients};
    use iam_configuration::{IdpConfig, IdpConnectorConfig, SecretStorage};
    use iam_domain::entity::provider::Provider;
    use std::path::{Path, PathBuf};

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
            match super::build_app_state(config, None).await {
                Ok(_) => panic!("empty OAuth state secret must fail closed at boot"),
                Err(error) => assert!(
                    error.to_string().contains("OAuth state"),
                    "must fail secret validation, not queue/database/network setup: {error}"
                ),
            }
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
