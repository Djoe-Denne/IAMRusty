use anyhow::Result;
use axum::Router;
use chrono::Duration;
use std::sync::Arc;
use tracing::info;

use iam_http_server::{create_app_routes, create_router};
use iam_infra::{
    auth::{
        GitHubOAuth2Client, GitLabOAuth2Client, PasswordResetServiceAdapter, PasswordService,
        PasswordServiceAdapter,
    },
    db::DbConnectionPool,
    event_adapter::IAMErrorMapper,
    repository::{
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
    token::JwtTokenService,
    transaction::IamOutboxUnitOfWorkImpl,
};
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};

use iam_configuration::AppConfig;
use iam_domain::error::DomainError;
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
}

impl IAMRustyApp {
    pub const fn new(
        app_state: AppState,
        outbox_dispatcher: Arc<OutboxDispatcher<DomainError>>,
        readiness: Arc<ReadinessProbe>,
    ) -> Self {
        Self {
            app_state,
            outbox_dispatcher,
            readiness,
        }
    }

    pub fn router(&self) -> Router {
        attach_ready(
            create_router(self.app_state.clone()),
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
                None,
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

    let repos = setup_repositories(&db_pool);
    let (github_auth_login, gitlab_auth_login, github_auth_link, gitlab_auth_link) =
        setup_oauth_clients(&config)?;

    // Create password service
    let password_service = Arc::new(PasswordService::new());
    let password_service_adapter = Arc::new(PasswordServiceAdapter::new(password_service.clone()));

    let (http_verifier_auth, token_service, registration_token_service) = setup_jwt(&config)?;

    let outbox_unit_of_work = Arc::new(IamOutboxUnitOfWorkImpl::new(
        db_pool.clone(),
        OutboxRecorder,
    ));
    let usecases = setup_iam_usecases(
        &db_pool,
        repos,
        event_publisher.clone(),
        github_auth_login,
        gitlab_auth_login,
        github_auth_link,
        gitlab_auth_link,
        password_service,
        password_service_adapter,
        token_service,
        registration_token_service,
        outbox_unit_of_work,
    );
    let registry = CommandRegistryFactory::create_iam_registry(usecases, &config.command);
    let command_service = Arc::new(GenericCommandService::new(Arc::new(registry)));

    // Verifier secret comes from the issuer (`[jwt.secret]`), not a
    // second `[auth.jwt]` copy that can drift.
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

    Ok(IAMRustyApp::new(app_state, outbox_dispatcher, readiness))
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
}

fn setup_oauth_and_link(
    user_repo: UserRepo,
    user_email_repo: UserEmailRepo,
    token_repo_login: TokenRepo,
    token_repo_link: TokenRepo,
    github_auth_login: GitHubOAuth2Client,
    gitlab_auth_login: GitLabOAuth2Client,
    github_auth_link: GitHubOAuth2Client,
    gitlab_auth_link: GitLabOAuth2Client,
    token_service: Arc<JwtTokenService>,
    registration_token_service: Arc<iam_infra::token::RegistrationTokenServiceImpl>,
) -> (Arc<dyn OAuthUseCase>, Arc<dyn LinkProviderUseCase>) {
    let mut oauth_service = iam_domain::service::oauth_service::OAuthService::new(
        user_repo.clone(),
        token_repo_login,
        user_email_repo.clone(),
        iam_domain::service::TokenService::new(token_service.clone(), Duration::hours(1)),
    );
    oauth_service.register_provider_client(
        iam_domain::entity::provider::Provider::GitHub,
        Box::new(github_auth_login),
    );
    oauth_service.register_provider_client(
        iam_domain::entity::provider::Provider::GitLab,
        Box::new(gitlab_auth_login),
    );
    let oauth = Arc::new(OAuthUseCaseImpl::new(
        Arc::new(oauth_service),
        registration_token_service,
        token_service,
    ));
    let provider_link_service = Arc::new(iam_domain::service::ProviderLinkService::new(
        Arc::new(user_repo),
        Arc::new(user_email_repo),
        Arc::new(token_repo_link),
    ));
    let link_provider = Arc::new(LinkProviderUseCaseImpl::new(
        Arc::new(github_auth_link),
        Arc::new(gitlab_auth_link),
        provider_link_service,
    ));
    (oauth, link_provider)
}

fn setup_auth_registration_password<EP>(
    db_pool: &DbConnectionPool,
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
) -> (
    Arc<dyn LoginUseCase>,
    Arc<dyn RegistrationUseCase>,
    Arc<dyn PasswordResetUseCase>,
)
where
    EP: EventPublisher<DomainError> + Send + Sync + 'static,
{
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
    let login_auth = Arc::new(LoginUseCaseImpl::new(auth_service));
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
    let registration = Arc::new(RegistrationUseCaseImpl::new(registration_service));
    let password_reset_service_adapter = Arc::new(PasswordResetServiceAdapter::new(password_service));
    let password_reset = Arc::new(PasswordResetUseCaseImpl::new_with_outbox_unit_of_work(
        Arc::new(user_repo),
        Arc::new(user_email_repo),
        Arc::new(password_reset_repo),
        token_service,
        event_publisher,
        password_reset_service_adapter,
        outbox_unit_of_work,
    ));
    (login_auth, registration, password_reset)
}

fn setup_provider_user_token(
    db_pool: &DbConnectionPool,
    user_repo: UserRepo,
    user_email_repo: UserEmailRepo,
    refresh_token_repo: RefreshRepo,
    token_service: Arc<JwtTokenService>,
) -> (Arc<dyn ProviderUseCase>, Arc<dyn UserUseCase>, Arc<dyn TokenUseCase>) {
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
    );
    let provider = Arc::new(ProviderUseCaseImpl::new(Arc::new(provider_auth_service)));
    let user = Arc::new(UserUseCaseImpl::new(Arc::new(
        iam_domain::service::UserServiceImpl::new(
            Arc::new(user_repo),
            Arc::new(user_email_repo),
            token_service.clone(),
        ),
    )));
    let token = Arc::new(TokenUseCaseImpl::new(Arc::new(
        iam_domain::service::RefreshTokenServiceImpl::new(
            Arc::new(refresh_token_repo),
            token_service,
        ),
    )));
    (provider, user, token)
}

fn setup_iam_usecases<EP>(
    db_pool: &DbConnectionPool,
    repos: IamRepos,
    event_publisher: Arc<EP>,
    github_auth_login: GitHubOAuth2Client,
    gitlab_auth_login: GitLabOAuth2Client,
    github_auth_link: GitHubOAuth2Client,
    gitlab_auth_link: GitLabOAuth2Client,
    password_service: Arc<PasswordService>,
    password_service_adapter: Arc<PasswordServiceAdapter>,
    token_service: Arc<JwtTokenService>,
    registration_token_service: Arc<iam_infra::token::RegistrationTokenServiceImpl>,
    outbox_unit_of_work: Arc<IamOutboxUnitOfWorkImpl>,
) -> IamRegistryUseCases
where
    EP: EventPublisher<DomainError> + Send + Sync + 'static,
{
    let IamRepos {
        user_repo,
        user_email_repo,
        email_verification_repo,
        password_reset_repo,
        token_repo_login,
        token_repo_link,
        refresh_token_repo,
    } = repos;
    let (oauth, link_provider) = setup_oauth_and_link(
        user_repo.clone(),
        user_email_repo.clone(),
        token_repo_login,
        token_repo_link,
        github_auth_login,
        gitlab_auth_login,
        github_auth_link,
        gitlab_auth_link,
        token_service.clone(),
        registration_token_service.clone(),
    );
    let (login_auth, registration, password_reset) = setup_auth_registration_password(
        db_pool,
        user_repo.clone(),
        user_email_repo.clone(),
        email_verification_repo,
        password_reset_repo,
        event_publisher,
        password_service,
        password_service_adapter,
        token_service.clone(),
        registration_token_service,
        outbox_unit_of_work,
    );
    let (provider, user, token) = setup_provider_user_token(
        db_pool,
        user_repo,
        user_email_repo,
        refresh_token_repo,
        token_service,
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

fn setup_repositories(db_pool: &DbConnectionPool) -> IamRepos {
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
    IamRepos {
        user_repo,
        user_email_repo,
        email_verification_repo,
        password_reset_repo,
        token_repo_login,
        token_repo_link,
        refresh_token_repo,
    }
}

fn setup_jwt(
    config: &AppConfig,
) -> Result<(
    iam_configuration::AuthConfig,
    Arc<JwtTokenService>,
    Arc<iam_infra::token::RegistrationTokenServiceImpl>,
)> {
    let http_verifier_auth = config.jwt.http_verifier_auth().map_err(|e| {
        tracing::error!("JWT issuer is incompatible with rustycog-http verifier: {e}");
        anyhow::anyhow!("JWT issuer is incompatible with rustycog-http verifier: {e}")
    })?;
    tracing::info!("Setting up JWT token service");
    let jwt_algorithm_config = config.jwt.create_jwt_algorithm().map_err(|e| {
        tracing::error!("Failed to create JWT algorithm from configuration: {e}");
        anyhow::anyhow!("Failed to create JWT algorithm from configuration: {e}")
    })?;
    let jwt_algorithm = match jwt_algorithm_config {
        iam_configuration::JwtAlgorithm::HS256(secret) => {
            tracing::info!(
                "Using HMAC256 JWT algorithm (secret length: {})",
                secret.len()
            );
            iam_infra::token::JwtAlgorithm::HS256(secret)
        }
        iam_configuration::JwtAlgorithm::RS256(key_pair) => {
            tracing::info!(
                "Using RSA256 JWT algorithm (key_id: {}, private_key: {} bytes, public_key: {} bytes)",
                key_pair.kid,
                key_pair.private_key.len(),
                key_pair.public_key.len()
            );
            iam_infra::token::JwtAlgorithm::RS256(iam_domain::entity::token::JwtKeyPair {
                private_key: key_pair.private_key,
                public_key: key_pair.public_key,
                kid: key_pair.kid,
            })
        }
    };
    let token_service = Arc::new(JwtTokenService::with_refresh_expiration(
        jwt_algorithm.clone(),
        config.jwt.expiration_seconds,
        config.jwt.refresh_token_expiration_seconds,
    ));
    let registration_token_service = Arc::new(
        iam_infra::token::RegistrationTokenServiceImpl::new(jwt_algorithm)
            .map_err(|e| anyhow::anyhow!("Failed to create registration token service: {e}"))?,
    );
    Ok((
        http_verifier_auth,
        token_service,
        registration_token_service,
    ))
}

fn setup_oauth_clients(
    config: &AppConfig,
) -> Result<(
    GitHubOAuth2Client,
    GitLabOAuth2Client,
    GitHubOAuth2Client,
    GitLabOAuth2Client,
)> {
    Ok((
        GitHubOAuth2Client::from_config(&config.oauth.github)?,
        GitLabOAuth2Client::from_config(&config.oauth.gitlab)?,
        GitHubOAuth2Client::from_config(&config.oauth.github)?,
        GitLabOAuth2Client::from_config(&config.oauth.gitlab)?,
    ))
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
        tokio::spawn(async move { create_app_routes(app_state, server_config, probe).await })
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
