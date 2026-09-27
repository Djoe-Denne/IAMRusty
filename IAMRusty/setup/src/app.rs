use anyhow::Result;
use axum::Router;
use chrono::Duration;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

use iam_http_server::{SignerRouteContext, create_app_routes, create_router};
use iam_infra::{
    auth::{
        HttpIdpConnector, PasswordResetServiceAdapter, PasswordService, PasswordServiceAdapter,
    },
    db::DbConnectionPool,
    event_adapter::IAMErrorMapper,
    repository::{
        SeaOrmIdentityRepository, SeaOrmSigningKeyRegistry, bootstrap_platform_signing_key,
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
        RotateContext, StaticCredential, TransitClientConfig,
    },
    token::{JwtAlgorithm, JwtTokenService},
    transaction::IamOutboxUnitOfWorkImpl,
};
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};

use iam_configuration::{AppConfig, IdpConfig, SecretStorage};
use iam_domain::entity::provider::Provider;
use iam_domain::entity::signing_key::JWKS_RETIRE_SKEW_SECONDS;
use iam_domain::error::DomainError;
use iam_domain::port::service::FederatedOAuthClient;
use readiness::{
    ComponentStatus, QueueRole, ReadinessProbe, attach_ready,
    create_signaled_multi_queue_event_publisher, signal_queue_status,
};
use rustycog::events::{adapter::MultiQueueEventPublisher, event::EventPublisher};
use rustycog::outbox::{OutboxConfig, OutboxDispatcher, OutboxRecorder};

use iam_application::{
    command::{CommandRegistryFactory, GenericCommandService, IamRegistryUseCases},
    usecase::{
        link_provider::{LinkProviderUseCase, LinkProviderUseCaseImpl},
        login::{LoginUseCase, LoginUseCaseImpl},
        oauth::{OAuthUseCase, OAuthUseCaseImpl},
        password_reset::{PasswordResetUseCase, PasswordResetUseCaseImpl, SessionRevoker},
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
}

impl IAMRustyApp {
    pub const fn new(
        app_state: AppState,
        outbox_dispatcher: Arc<OutboxDispatcher<DomainError>>,
        readiness: Arc<ReadinessProbe>,
        idp: Arc<IdpConfig>,
        signer: Option<Arc<SignerRouteContext>>,
    ) -> Self {
        Self {
            app_state,
            outbox_dispatcher,
            readiness,
            idp,
            signer,
        }
    }

    pub fn router(&self) -> Router {
        attach_ready(
            create_router(
                self.app_state.clone(),
                self.idp.clone(),
                self.signer.clone(),
            ),
            self.readiness.clone(),
        )
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
        vec![tokio::spawn(async move {
            dispatcher
                .start()
                .await
                .map_err(|e| anyhow::anyhow!("IAMRusty outbox dispatcher failed: {e}"))
        })]
    }

    pub async fn stop_background_tasks(&self) {
        if let Err(e) = self.outbox_dispatcher.stop().await {
            tracing::error!("Failed to stop IAMRusty outbox dispatcher: {e}");
        }
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
    info!("Building IAM service...");

    // Setup database connection pool
    let db_pool = DbConnectionPool::new(&config.database).await?;
    let db_write = db_pool.get_write_connection();
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

    let repos = setup_repositories(&db_pool, config.jwt.platform_issuer());
    let idp = Arc::new(config.idp.clone());
    let oauth_clients = setup_oauth_clients(&config)?;

    // Create password service
    let password_service = Arc::new(PasswordService::new());
    let password_service_adapter = Arc::new(PasswordServiceAdapter::new(password_service.clone()));

    let (http_verifier_auth, inline_jwks, token_service, registration_token_service, signer_ctx) =
        setup_jwt(&config, db_pool.get_write_connection()).await?;

    let outbox_unit_of_work = Arc::new(IamOutboxUnitOfWorkImpl::new(
        db_pool.clone(),
        OutboxRecorder,
    ));
    let usecases = setup_iam_usecases(
        &db_pool,
        IamUsecasesDeps {
            repos,
            event_publisher: event_publisher.clone(),
            clients: oauth_clients,
            password_service,
            password_service_adapter,
            token_service,
            registration_token_service,
            outbox_unit_of_work,
        },
    );
    let registry = CommandRegistryFactory::create_iam_registry(usecases, &config.command);
    let command_service = Arc::new(GenericCommandService::new(Arc::new(registry)));

    // Seed inline JWKS so IAM does not HTTP-call itself before listen.
    let user_id_extractor =
        UserIdExtractor::from_config_with_inline_jwks(http_verifier_auth, &inline_jwks)
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
    ))
}

type UserRepo = CombinedUserRepository<UserReadRepositoryImpl, UserWriteRepositoryImpl>;
type UserEmailRepo =
    CombinedUserEmailRepository<UserEmailReadRepositoryImpl, UserEmailWriteRepositoryImpl>;
type TokenRepo = CombinedTokenRepository<TokenReadRepositoryImpl, TokenWriteRepositoryImpl>;
type RefreshRepo =
    CombinedRefreshTokenRepository<RefreshTokenReadRepositoryImpl, RefreshTokenWriteRepositoryImpl>;

struct RefreshSessionRevoker {
    repo: RefreshRepo,
}

#[async_trait::async_trait]
impl SessionRevoker for RefreshSessionRevoker {
    async fn revoke_all(&self, user_id: uuid::Uuid) -> Result<u64, String> {
        use iam_domain::port::repository::RefreshTokenWriteRepository;
        self.repo
            .delete_by_user_id(user_id)
            .await
            .map_err(|e| e.to_string())
    }
}
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
    refresh_token_repo: RefreshRepo,
    identity_repo: Arc<dyn iam_domain::port::repository::IdentityRepository<Error = DomainError>>,
    platform_issuer: String,
}

struct IamUsecasesDeps<EP> {
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
    let oauth = Arc::new(OAuthUseCaseImpl::new(
        Arc::new(oauth_service),
        registration_token_service,
        token_service,
        identity_repo,
        platform_issuer,
    ));
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
        refresh_token_repo,
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
        ),
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
        ),
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
        .with_session_revoker(Arc::new(RefreshSessionRevoker {
            repo: refresh_token_repo,
        })),
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
    let (oauth, link_provider) = setup_oauth_and_link(OauthLinkDeps {
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
            refresh_token_repo: refresh_token_repo.clone(),
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

fn setup_repositories(db_pool: &DbConnectionPool, platform_issuer: String) -> IamRepos {
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
    let signing_key_registry = Arc::new(SeaOrmSigningKeyRegistry::new(
        db_pool.get_write_connection(),
    ))
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
) -> Result<(
    iam_configuration::AuthConfig,
    String,
    Arc<JwtTokenService>,
    Arc<iam_infra::token::RegistrationTokenServiceImpl>,
    Option<Arc<SignerRouteContext>>,
)> {
    let http_verifier_auth = config.jwt.http_verifier_auth().map_err(|e| {
        tracing::error!("JWT verifier config invalid: {e}");
        anyhow::anyhow!("JWT verifier config invalid: {e}")
    })?;
    tracing::info!("Setting up JWT token service");
    let jwt_algorithm_config = config.jwt.create_jwt_algorithm().map_err(|e| {
        tracing::error!("Failed to create JWT algorithm from configuration: {e}");
        anyhow::anyhow!("Failed to create JWT algorithm from configuration: {e}")
    })?;

    let platform_issuer = config.jwt.platform_issuer();
    let registry = Arc::new(SeaOrmSigningKeyRegistry::new(db.clone()));
    let identity_repo = Arc::new(SeaOrmIdentityRepository::new(db));
    let pem_root = organization_signer_pem_root(&config.jwt.secret);
    let transit = match config.jwt.transit_endpoint() {
        Some((url, token)) => {
            let workload = Arc::new(
                StaticCredential::from_pair("openbao-token", token)
                    .map_err(|e| anyhow::anyhow!("jwt.transit_token invalid: {e}"))?,
            );
            Some(TransitClientConfig {
                base_url: url,
                workload: workload as Arc<dyn iam_domain::port::WorkloadIdentity>,
                token_ref: "openbao-token".to_string(),
            })
        }
        None => None,
    };
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
            )
            .await
            .map_err(|e| anyhow::anyhow!("signing key bootstrap: {e}"))?;

            let jwk = JwtTokenService::jwk_from_pem(
                &bootstrapped.public_key,
                &bootstrapped.kid,
                &bootstrapped.issuer,
            )
            .map_err(|e| anyhow::anyhow!("JWKS build: {e}"))?;
            let jwks = iam_domain::entity::token::JwkSet { keys: vec![jwk] };

            (
                JwtAlgorithm::RS256(iam_domain::entity::token::JwtKeyPair {
                    private_key: key_pair.private_key,
                    public_key: key_pair.public_key,
                    kid: key_pair.kid.clone(),
                }),
                Some((provider, bootstrapped.kid, bootstrapped.issuer, jwks)),
            )
        }
    };

    let mut token_service = JwtTokenService::with_refresh_expiration(
        jwt_algorithm.clone(),
        config.jwt.expiration_seconds,
        config.jwt.refresh_token_expiration_seconds,
    )
    .with_issuer_audience(platform_issuer.clone(), config.jwt.audience.clone());

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
    iam_http_server::configure_oauth_state_secret(config.jwt.oauth_state_secret.clone());
    if !config.internal_service_token.is_empty() {
        iam_http_server::configure_internal_service_token(config.internal_service_token.clone());
    }
    let registration_token_service = Arc::new(
        iam_infra::token::RegistrationTokenServiceImpl::new(jwt_algorithm)
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
    let by_slug = setup_http_idp_clients(&config.idp)?;
    Ok(OauthClients { by_slug })
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

fn setup_http_idp_clients(
    idp: &IdpConfig,
) -> Result<HashMap<Provider, Arc<dyn FederatedOAuthClient>>> {
    idp.validate().map_err(DomainError::OAuth2Error)?;
    let mut by_slug = HashMap::new();
    for connector in &idp.connectors {
        let provider = Provider::parse_slug(&connector.id).map_err(|_| {
            DomainError::OAuth2Error(format!("illegal IdP connector id: {}", connector.id))
        })?;
        let client = HttpIdpConnector::new(&connector.base_url, &connector.hmac_secret)?;
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
        tokio::spawn(async move {
            create_app_routes(app_state, server_config, probe, idp, signer).await
        })
    };

    let mut background_tasks = app.start_background_tasks();
    let mut outbox_handle = background_tasks
        .pop()
        .ok_or_else(|| anyhow::anyhow!("IAMRusty outbox dispatcher should always be configured"))?;

    let result: Result<()> = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown signal received; stopping IAMRusty runtime");
            Ok(())
        }
        result = &mut outbox_handle => {
            match result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) => Err(error),
                Err(error) => Err(anyhow::anyhow!("IAMRusty outbox dispatcher task panicked: {error}")),
            }
        }
        result = &mut server_handle => {
            match result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) => Err(error),
                Err(error) => Err(anyhow::anyhow!("IAMRusty HTTP server task panicked: {error}")),
            }
        }
    };

    app.stop_background_tasks().await;
    if !outbox_handle.is_finished() {
        outbox_handle.abort();
    }
    if !server_handle.is_finished() {
        server_handle.abort();
    }
    let _ = outbox_handle.await;
    let _ = server_handle.await;

    result
}

#[cfg(test)]
mod tests {
    use super::{organization_signer_pem_root, setup_http_idp_clients};
    use iam_configuration::{IdpConfig, IdpConnectorConfig, SecretStorage};
    use iam_domain::entity::provider::Provider;
    use std::path::{Path, PathBuf};

    fn complete_connector(id: &str) -> IdpConnectorConfig {
        IdpConnectorConfig {
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
        let by_slug = setup_http_idp_clients(&idp).expect("complete huggingface line boots");
        let expected = Provider::parse_slug("huggingface").expect("slug");
        assert!(by_slug.contains_key(&expected));
        assert_eq!(by_slug.len(), 1);
    }

    #[test]
    fn setup_wires_github_and_gitlab() {
        let idp = IdpConfig {
            connectors: vec![complete_connector("github"), complete_connector("gitlab")],
        };
        let by_slug = setup_http_idp_clients(&idp).expect("github+gitlab boot");
        assert!(by_slug.contains_key(&Provider::parse_slug("github").expect("github")));
        assert!(by_slug.contains_key(&Provider::parse_slug("gitlab").expect("gitlab")));
        assert_eq!(by_slug.len(), 2);
    }

    #[test]
    fn setup_rejects_illegal_id() {
        let idp = IdpConfig {
            connectors: vec![complete_connector("hugging-face")],
        };
        match setup_http_idp_clients(&idp) {
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
        match setup_http_idp_clients(&idp) {
            Ok(_) => panic!("expected fail-closed boot"),
            Err(err) => assert!(
                err.to_string().contains("at least 16"),
                "unexpected error: {err}"
            ),
        }
    }

    #[test]
    fn setup_rejects_empty_registry() {
        match setup_http_idp_clients(&IdpConfig::default()) {
            Ok(_) => panic!("expected fail-closed boot"),
            Err(err) => assert!(
                err.to_string().contains("must not be empty"),
                "unexpected error: {err}"
            ),
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
