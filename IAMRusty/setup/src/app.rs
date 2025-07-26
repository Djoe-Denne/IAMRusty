use anyhow::Result;
use chrono::Duration;
use std::sync::Arc;
use tracing::info;

use rustycog_http::{AppState, UserIdExtractor};
use iam_http_server::{create_app_routes};
use iam_configuration::ServerConfig as HttpServerConfig;
use iam_infra::{
    auth::{GitHubOAuth2Client, GitLabOAuth2Client, PasswordService, PasswordServiceAdapter, PasswordResetServiceAdapter},
    db::DbConnectionPool,
    event_adapter::create_multi_queue_event_publisher_async,
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
        token_read::TokenReadRepositoryImpl,
        token_write::TokenWriteRepositoryImpl,
        user_email_read::UserEmailReadRepositoryImpl,
        user_email_write::UserEmailWriteRepositoryImpl,
        user_read::UserReadRepositoryImpl,
        user_write::UserWriteRepositoryImpl,
    },
    token::JwtTokenService,
};

use iam_configuration::AppConfig;
use iam_domain::error::DomainError;
use rustycog_events::{adapter::MultiQueueEventPublisher, event::EventPublisher};

use iam_application::{
    command::{CommandRegistryFactory, GenericCommandService},
    usecase::{
        link_provider::LinkProviderUseCaseImpl, login::LoginUseCaseImpl, oauth::OAuthUseCaseImpl,
        password_reset::PasswordResetUseCaseImpl, provider::ProviderUseCaseImpl, registration::RegistrationUseCaseImpl,
        token::TokenUseCaseImpl, user::UserUseCaseImpl,
    },
};

use crate::config::ServerConfig;


pub async fn build_and_run(config: AppConfig, server_config: ServerConfig, maybe_event_publisher: Option<Arc<MultiQueueEventPublisher<DomainError>>>) -> Result<()> {
    let app_state = build_app_state(config.clone(), maybe_event_publisher).await?;
    run_server(app_state, server_config).await
}

pub async fn build_app_state(config: AppConfig, maybe_event_publisher: Option<Arc<MultiQueueEventPublisher<DomainError>>>) -> Result<AppState> {

    let event_publisher: Arc<MultiQueueEventPublisher<DomainError>>;
    if maybe_event_publisher.is_some() {
        event_publisher = maybe_event_publisher.unwrap();
    } else {
        event_publisher = create_event_publisher_from_config(&config).await?;
    }

    build_app_state_with_event_publisher(config, event_publisher).await
}

/// Create the default event publisher from configuration
async fn create_event_publisher_from_config(config: &AppConfig) -> Result<Arc<MultiQueueEventPublisher<DomainError>>> {
    // Create event publisher using configuration
    // For now, create a multi-queue publisher that handles all configured queues
    // You can modify this to handle specific queues by passing Some(queue_names_set)
    //
    // Examples:
    //
    // 1. Handle all queues (current behavior):
    // let event_publisher = create_multi_queue_event_publisher_async(&config.queue, None).await?;
    //
    // 2. Handle only specific queues:
    // let mut specific_queues = HashSet::new();
    // specific_queues.insert("test-user-events".to_string());
    // let event_publisher = create_multi_queue_event_publisher_async(&config.queue, Some(specific_queues)).await?;
    //
    // 3. Handle queues based on environment:
    // let queue_names = if config.is_test_environment() {
    //     let mut test_queues = HashSet::new();
    //     test_queues.insert("test-user-events".to_string());
    //     Some(test_queues)
    // } else {
    //     None // Handle all queues in production
    // };
    // let event_publisher = create_multi_queue_event_publisher_async(&config.queue, queue_names).await?;
    let event_publisher = create_multi_queue_event_publisher_async(&config.queue, None).await?;
    Ok(event_publisher)
}

/// Build app state with a custom event publisher (useful for testing)
pub async fn build_app_state_with_event_publisher<EP>(
    config: AppConfig, 
    event_publisher: Arc<EP>
) -> Result<AppState> 
where
    EP: EventPublisher<DomainError> + Send + Sync + 'static,
{
    info!("Building IAM service...");

    // Setup database connection pool
    let db_pool = DbConnectionPool::new(&config.database).await?;
    info!(
        "Database connection pool initialized with {} read replicas",
        if config.database.read_replicas.is_empty() {
            0
        } else {
            config.database.read_replicas.len()
        }
    );

    // Create repositories
    let user_read_repo = UserReadRepositoryImpl::new(db_pool.get_read_connection());
    let user_write_repo = UserWriteRepositoryImpl::new(db_pool.get_write_connection());
    let user_repo = CombinedUserRepository::new(user_read_repo, user_write_repo);

    let user_email_read_repo = UserEmailReadRepositoryImpl::new(db_pool.get_read_connection());
    let user_email_write_repo = UserEmailWriteRepositoryImpl::new(db_pool.get_write_connection());
    let user_email_repo =
        CombinedUserEmailRepository::new(user_email_read_repo, user_email_write_repo);

    // Create email verification repositories
    let email_verification_read_repo =
        SeaOrmEmailVerificationReadRepository::new(db_pool.get_read_connection());
    let email_verification_write_repo =
        SeaOrmEmailVerificationWriteRepository::new(db_pool.get_write_connection());
    let email_verification_repo = CombinedEmailVerificationRepository::new_with_sea_orm(
        Arc::new(email_verification_read_repo),
        Arc::new(email_verification_write_repo),
    );

    // Create password reset token repositories
    let password_reset_read_repo =
        PasswordResetTokenReadRepositoryImpl::new(db_pool.get_read_connection());
    let password_reset_write_repo =
        PasswordResetTokenWriteRepositoryImpl::new(db_pool.get_write_connection());
    let password_reset_repo = CombinedPasswordResetTokenRepository::new(
        Arc::new(password_reset_read_repo),
        Arc::new(password_reset_write_repo),
    );

    let token_read_repo = TokenReadRepositoryImpl::new(db_pool.get_read_connection());
    let token_write_repo = TokenWriteRepositoryImpl::new(db_pool.get_write_connection());
    let token_repo_login =
        CombinedTokenRepository::new(token_read_repo.clone(), token_write_repo.clone());

    let token_read_repo_link = TokenReadRepositoryImpl::new(db_pool.get_read_connection());
    let token_write_repo_link = TokenWriteRepositoryImpl::new(db_pool.get_write_connection());
    let token_repo_link = CombinedTokenRepository::new(token_read_repo_link, token_write_repo_link);

    let refresh_token_read_repo =
        RefreshTokenReadRepositoryImpl::new(db_pool.get_read_connection());
    let refresh_token_write_repo =
        RefreshTokenWriteRepositoryImpl::new(db_pool.get_write_connection());
    let refresh_token_repo =
        CombinedRefreshTokenRepository::new(refresh_token_read_repo, refresh_token_write_repo);

    // Create auth services
    let github_auth_login = GitHubOAuth2Client::from_config(&config.oauth.github);
    let gitlab_auth_login = GitLabOAuth2Client::from_config(&config.oauth.gitlab);

    let github_auth_link = GitHubOAuth2Client::from_config(&config.oauth.github);
    let gitlab_auth_link = GitLabOAuth2Client::from_config(&config.oauth.gitlab);

    // Create password service
    let password_service = Arc::new(PasswordService::new());
    let password_service_adapter = Arc::new(PasswordServiceAdapter::new(password_service.clone()));

    // Create token service with secret resolved from configuration
    tracing::info!("Setting up JWT token service");
    let jwt_algorithm_config = config.jwt.create_jwt_algorithm().map_err(|e| {
        tracing::error!("Failed to create JWT algorithm from configuration: {}", e);
        anyhow::anyhow!("Failed to create JWT algorithm from configuration: {}", e)
    })?;
    tracing::debug!("Successfully created JWT algorithm config");

    // Convert configuration JwtAlgorithm to infra JwtAlgorithm
    let jwt_algorithm = match jwt_algorithm_config.clone() {
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
    tracing::info!("JWT token service created successfully");

    // Create registration token service
    tracing::info!("Creating registration token service");
    let registration_token_service = Arc::new(
        iam_infra::token::RegistrationTokenServiceImpl::new(jwt_algorithm.clone())
        .unwrap(),
    );

    // Create OAuth service for OAuth flows
    let mut oauth_service = iam_domain::service::oauth_service::OAuthService::new(
        user_repo.clone(),
        token_repo_login,
        user_email_repo.clone(),
        iam_domain::service::TokenService::new(
            Box::new(token_service.as_ref().clone()),
            chrono::Duration::hours(1),
        ),
    );

    // Register OAuth provider clients
    oauth_service.register_provider_client(
        iam_domain::entity::provider::Provider::GitHub,
        Box::new(github_auth_login),
    );
    oauth_service.register_provider_client(
        iam_domain::entity::provider::Provider::GitLab,
        Box::new(gitlab_auth_login),
    );

    let oauth_usecase =
        OAuthUseCaseImpl::new(Arc::new(oauth_service), registration_token_service.clone(), token_service.clone());

    // Create provider link service for domain business logic
    let provider_link_service = Arc::new(iam_domain::service::ProviderLinkService::new(
        Arc::new(user_repo.clone()),
        Arc::new(user_email_repo.clone()),
        Arc::new(token_repo_link),
    ));

    let link_provider_usecase = LinkProviderUseCaseImpl::new(
        Arc::new(github_auth_link),
        Arc::new(gitlab_auth_link),
        provider_link_service,
    );

    // Create domain services
    let user_service = Arc::new(iam_domain::service::UserServiceImpl::new(
        Arc::new(user_repo.clone()),
        Arc::new(user_email_repo.clone()),
        token_service.clone(),
    ));

    let refresh_token_service = Arc::new(iam_domain::service::RefreshTokenServiceImpl::new(
        Arc::new(refresh_token_repo.clone()),
        token_service.clone(),
    ));
    
    tracing::info!("Creating auth service for login use case");
    let auth_service = Arc::new(iam_domain::service::auth_service::AuthService::new(
        Arc::new(user_repo.clone()),
        Arc::new(user_email_repo.clone()),
        Arc::new(email_verification_repo.clone()),
        password_service_adapter.clone(),
        token_service.clone(),
        registration_token_service.clone(),
        event_publisher.clone(),
    ));

    tracing::info!("Creating login use case with verification token support");
    let login_usecase = LoginUseCaseImpl::new(auth_service);

    // Create registration use case
    tracing::info!("Creating registration service");
    let registration_service = Arc::new(iam_domain::service::RegistrationServiceImpl::new(
        Arc::new(user_repo.clone()),
        Arc::new(user_repo.clone()),
        Arc::new(user_email_repo.clone()),
        Arc::new(email_verification_repo.clone()),
        registration_token_service.clone(),
        token_service.clone(),
        event_publisher.clone(),
    ));

    tracing::info!("Creating registration use case");
    let registration_usecase = Arc::new(RegistrationUseCaseImpl::new(registration_service));

    // Create password reset use case
    tracing::info!("Creating password reset service adapter");
    let password_reset_service_adapter = Arc::new(PasswordResetServiceAdapter::new(
        password_service.clone(),
    ));

    tracing::info!("Creating password reset use case");
    let password_reset_usecase = Arc::new(PasswordResetUseCaseImpl::new(
        Arc::new(user_repo.clone()),
        Arc::new(user_email_repo.clone()),
        Arc::new(password_reset_repo),
        token_service.clone(),
        event_publisher.clone(),
        password_reset_service_adapter,
    ));

    // Create provider usecase
    // For provider usecase, we only need the get_provider_token method from AuthService
    // which doesn't use the TokenService, so we can create a minimal one
    #[derive(Debug, Clone)]
    struct MinimalJwtTokenEncoder;

    impl iam_domain::port::service::JwtTokenEncoder for MinimalJwtTokenEncoder {
        fn encode(
            &self,
            _claims: &iam_domain::entity::token::TokenClaims,
        ) -> Result<String, iam_domain::error::DomainError> {
            Ok("dummy_token".to_string())
        }
        fn decode(
            &self,
            _token: &str,
        ) -> Result<iam_domain::entity::token::TokenClaims, iam_domain::error::DomainError> {
            Err(iam_domain::error::DomainError::InvalidToken)
        }
        fn jwks(&self) -> iam_domain::entity::token::JwkSet {
            iam_domain::entity::token::JwkSet { keys: vec![] }
        }
    }

    // Create separate token repository for provider auth service
    let token_read_repo_provider = TokenReadRepositoryImpl::new(db_pool.get_read_connection());
    let token_write_repo_provider = TokenWriteRepositoryImpl::new(db_pool.get_write_connection());
    let token_repo_provider =
        CombinedTokenRepository::new(token_read_repo_provider, token_write_repo_provider);

    let provider_auth_service = iam_domain::service::oauth_service::OAuthService::new(
        user_repo.clone(),
        token_repo_provider,
        user_email_repo.clone(),
        iam_domain::service::token_service::TokenService::new(
            Box::new(MinimalJwtTokenEncoder),
            Duration::hours(1),
        ),
    );
    let provider_usecase = ProviderUseCaseImpl::new(Arc::new(provider_auth_service));

    // Create separate instances for command service (reuse iam_domain services)
    let user_service_for_commands = Arc::new(iam_domain::service::UserServiceImpl::new(
        Arc::new(user_repo),
        Arc::new(user_email_repo),
        token_service.clone(),
    ));

    let refresh_token_service_for_commands = Arc::new(
        iam_domain::service::RefreshTokenServiceImpl::new(Arc::new(refresh_token_repo), token_service),
    );

    let user_usecase_for_commands = UserUseCaseImpl::new(user_service_for_commands);
    let token_usecase_for_commands = TokenUseCaseImpl::new(refresh_token_service_for_commands);

    // Create command registry and service
    let registry = CommandRegistryFactory::create_iam_registry(
        Arc::new(oauth_usecase),
        Arc::new(link_provider_usecase),
        Arc::new(provider_usecase),
        Arc::new(token_usecase_for_commands),
        Arc::new(user_usecase_for_commands),
        Arc::new(login_usecase),
        registration_usecase.clone(),
        password_reset_usecase.clone(),
        config.command.clone(),
    );
    let command_service = Arc::new(GenericCommandService::new(Arc::new(registry)));

    // Create user ID extractor for authentication
    let user_id_extractor = UserIdExtractor::new();

    // Create app state
    let app_state = AppState::new(command_service, user_id_extractor);

    Ok(app_state)
}

pub async fn run_server(app_state: AppState, app_config: ServerConfig) -> Result<()> {
    info!("Starting IAM service...");

    // Convert our ServerConfig to HttpServerConfig
    let server_config = HttpServerConfig {
        host: app_config.host,
        port: app_config.port,
        tls_enabled: app_config.tls_enabled,
        tls_cert_path: app_config.tls_cert_path,
        tls_key_path: app_config.tls_key_path,
        tls_port: app_config.tls_port,
    };

    // Start server (HTTP or HTTPS based on configuration)
    if server_config.tls_enabled {
        info!(
            "Starting HTTPS server on {}:{}",
            server_config.host,
            server_config.tls_port
        );
    } else {
        info!(
            "Starting HTTP server on {}:{}",
            server_config.host, server_config.port
        );
    }

    create_app_routes(app_state, server_config).await?;

    Ok(())
}
