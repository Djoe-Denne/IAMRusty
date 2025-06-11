use std::sync::Arc;
use anyhow::Result;
use tracing::info;
use chrono::Duration;

use http_server::{AppState, serve_with_config, ServerConfig as HttpServerConfig};
use infra::{
    auth::{GitHubOAuth2Client, GitLabOAuth2Client, PasswordService, PasswordServiceAdapter},
    token::{JwtTokenService},
    repository::{
        user_read::UserReadRepositoryImpl,
        user_write::UserWriteRepositoryImpl,
        user_email_read::UserEmailReadRepositoryImpl,
        user_email_write::UserEmailWriteRepositoryImpl,
        token_read::TokenReadRepositoryImpl,
        token_write::TokenWriteRepositoryImpl,
        refresh_token_read::RefreshTokenReadRepositoryImpl,
        refresh_token_write::RefreshTokenWriteRepositoryImpl,
        combined_repository::{CombinedUserRepository, CombinedTokenRepository, CombinedRefreshTokenRepository},
        combined_user_email_repository::CombinedUserEmailRepository,
        email_verification_read::SeaOrmEmailVerificationReadRepository,
        email_verification_write::SeaOrmEmailVerificationWriteRepository,
        combined_email_verification_repository::CombinedEmailVerificationRepository,
    },
    db::DbConnectionPool,
    event_adapter::create_event_publisher,
};

use configuration::AppConfig;

use application::{
    usecase::{
        login::LoginUseCaseImpl,
        user::UserUseCaseImpl,
        token::TokenUseCaseImpl,
        link_provider::LinkProviderUseCaseImpl,
        provider::ProviderUseCaseImpl,
        oauth::AuthUseCaseImpl,
    },
    command::{
        CommandRegistryFactory, GenericCommandService,
    },
};

use crate::config::ServerConfig;
pub async fn build_and_run(
    config: AppConfig,
    app_config: ServerConfig,
) -> Result<()> {
    let app_state = build_app_state(config.clone()).await?;
    run_server(app_state, app_config).await
}

pub async fn build_app_state(config: AppConfig) -> Result<AppState> {
    info!("Building IAM service...");
    
    // Setup database connection pool
    let db_pool = DbConnectionPool::new(&config.database).await?;
    info!("Database connection pool initialized with {} read replicas", 
          if config.database.read_replicas.is_empty() { 0 } else { config.database.read_replicas.len() });

    // Create repositories
    let user_read_repo = UserReadRepositoryImpl::new(db_pool.get_read_connection());
    let user_write_repo = UserWriteRepositoryImpl::new(db_pool.get_write_connection());
    let user_repo = CombinedUserRepository::new(user_read_repo, user_write_repo);

    let user_email_read_repo = UserEmailReadRepositoryImpl::new(db_pool.get_read_connection());
    let user_email_write_repo = UserEmailWriteRepositoryImpl::new(db_pool.get_write_connection());
    let user_email_repo = CombinedUserEmailRepository::new(user_email_read_repo, user_email_write_repo);

    // Create email verification repositories
    let email_verification_read_repo = SeaOrmEmailVerificationReadRepository::new(db_pool.get_read_connection());
    let email_verification_write_repo = SeaOrmEmailVerificationWriteRepository::new(db_pool.get_write_connection());
    let email_verification_repo = CombinedEmailVerificationRepository::new_with_sea_orm(
        Arc::new(email_verification_read_repo),
        Arc::new(email_verification_write_repo),
    );

    let token_read_repo = TokenReadRepositoryImpl::new(db_pool.get_read_connection());
    let token_write_repo = TokenWriteRepositoryImpl::new(db_pool.get_write_connection());
    let token_repo_login = CombinedTokenRepository::new(token_read_repo.clone(), token_write_repo.clone());
    
    let token_read_repo_link = TokenReadRepositoryImpl::new(db_pool.get_read_connection());
    let token_write_repo_link = TokenWriteRepositoryImpl::new(db_pool.get_write_connection());
    let token_repo_link = CombinedTokenRepository::new(token_read_repo_link, token_write_repo_link);
    
    let refresh_token_read_repo = RefreshTokenReadRepositoryImpl::new(db_pool.get_read_connection());
    let refresh_token_write_repo = RefreshTokenWriteRepositoryImpl::new(db_pool.get_write_connection());
    let refresh_token_repo = CombinedRefreshTokenRepository::new(refresh_token_read_repo, refresh_token_write_repo);

    // Create auth services
    let github_auth_login = GitHubOAuth2Client::from_config(&config.oauth.github);
    let gitlab_auth_login = GitLabOAuth2Client::from_config(&config.oauth.gitlab);
    
    let github_auth_link = GitHubOAuth2Client::from_config(&config.oauth.github);
    let gitlab_auth_link = GitLabOAuth2Client::from_config(&config.oauth.gitlab);

    // Create password service
    let password_service = Arc::new(PasswordService::new());
    let password_service_adapter = Arc::new(PasswordServiceAdapter::new(password_service));

    // Create event publisher using configuration
    let event_publisher = create_event_publisher(&config.kafka)?;

    // Create token service with secret resolved from configuration
    tracing::info!("Setting up JWT token service");
    let jwt_algorithm_config = config.jwt.create_jwt_algorithm()
        .map_err(|e| {
            tracing::error!("Failed to create JWT algorithm from configuration: {}", e);
            anyhow::anyhow!("Failed to create JWT algorithm from configuration: {}", e)
        })?;
    tracing::debug!("Successfully created JWT algorithm config");
    
    // Convert configuration JwtAlgorithm to infra JwtAlgorithm
    let jwt_algorithm = match jwt_algorithm_config {
        configuration::JwtAlgorithm::HS256(secret) => {
            tracing::info!("Using HMAC256 JWT algorithm (secret length: {})", secret.len());
            infra::token::JwtAlgorithm::HS256(secret)
        }
        configuration::JwtAlgorithm::RS256(key_pair) => {
            tracing::info!("Using RSA256 JWT algorithm (key_id: {}, private_key: {} bytes, public_key: {} bytes)", 
                key_pair.kid, key_pair.private_key.len(), key_pair.public_key.len());
            infra::token::JwtAlgorithm::RS256(domain::entity::token::JwtKeyPair {
                private_key: key_pair.private_key,
                public_key: key_pair.public_key,
                kid: key_pair.kid,
            })
        }
    };
    
    let token_service = Arc::new(JwtTokenService::with_refresh_expiration(
        jwt_algorithm,
        config.jwt.expiration_seconds,
        config.jwt.refresh_token_expiration_seconds,
    ));
    tracing::info!("JWT token service created successfully");

    // Create use cases
    let login_usecase = LoginUseCaseImpl::new(
        Arc::new(github_auth_login),
        Arc::new(gitlab_auth_login),
        Arc::new(user_repo.clone()),
        Arc::new(user_email_repo.clone()),
        Arc::new(token_repo_login),
        Arc::new(refresh_token_repo.clone()),
        token_service.clone(),
    );

    // Create provider link service for domain business logic
    let provider_link_service = Arc::new(domain::service::ProviderLinkService::new(
        Arc::new(user_repo.clone()),
        Arc::new(user_email_repo.clone()),
        Arc::new(token_repo_link),
    ));

    let link_provider_usecase = LinkProviderUseCaseImpl::new(
        Arc::new(github_auth_link),
        Arc::new(gitlab_auth_link),
        provider_link_service,
    );

    let user_usecase = UserUseCaseImpl::new(
        Arc::new(user_repo.clone()),
        Arc::new(user_email_repo.clone()),
        token_service.clone(),
    );
    
    let token_usecase = TokenUseCaseImpl::new(
        Arc::new(refresh_token_repo.clone()),
        token_service.clone(),
    );

    // Create auth use case with resolved secret for verification token generation
    tracing::info!("Resolving JWT secret for email verification tokens");
    let jwt_secret_for_verification = match config.jwt.resolve_secret() {
        Ok(configuration::JwtSecret::Hmac(secret)) => {
            tracing::debug!("Using HMAC secret for verification tokens (length: {} bytes)", secret.len());
            secret
        }
        Ok(configuration::JwtSecret::Rsa { private_key, key_id, .. }) => {
            tracing::info!("RSA JWT configuration detected, deriving HMAC secret for verification tokens");
            
            // For RSA configurations, we derive a consistent HMAC secret from the private key
            // This ensures verification tokens work even with RSA JWT configurations
            // We use SHA256 to create a fixed-length secret from the private key
            use sha2::{Sha256, Digest};
            use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
            
            let mut hasher = Sha256::new();
            hasher.update(b"iam_verification_token_secret:");
            hasher.update(key_id.as_bytes());
            hasher.update(b":");
            hasher.update(private_key.as_bytes());
            let hash = hasher.finalize();
            let derived_secret = URL_SAFE_NO_PAD.encode(&hash);
            
            tracing::debug!("Derived HMAC secret for verification tokens from RSA private key (key_id: {}, derived length: {} bytes)", 
                key_id, derived_secret.len());
            
            derived_secret
        }
        Err(e) => {
            tracing::error!("Failed to resolve JWT secret for verification tokens: {}", e);
            return Err(anyhow::anyhow!("Failed to resolve JWT secret for verification tokens: {}", e));
        }
    };
    
    tracing::info!("Creating auth use case with verification token support");
    let auth_usecase = AuthUseCaseImpl::new(
        Arc::new(user_repo.clone()),
        Arc::new(user_email_repo.clone()),
        Arc::new(email_verification_repo),
        password_service_adapter.clone(),
        token_service.clone(),
        event_publisher,
        jwt_secret_for_verification,
    );

    // Create provider usecase
    // For provider usecase, we only need the get_provider_token method from AuthService
    // which doesn't use the TokenService, so we can create a minimal one
    #[derive(Debug, Clone)]
    struct MinimalJwtTokenEncoder;
    
    impl domain::port::service::JwtTokenEncoder for MinimalJwtTokenEncoder {
        fn encode(&self, _claims: &domain::entity::token::TokenClaims) -> Result<String, domain::error::DomainError> {
            Ok("dummy_token".to_string())
        }
        fn decode(&self, _token: &str) -> Result<domain::entity::token::TokenClaims, domain::error::DomainError> {
            Err(domain::error::DomainError::InvalidToken)
        }
        fn jwks(&self) -> domain::entity::token::JwkSet {
            domain::entity::token::JwkSet { keys: vec![] }
        }
    }
    
    // Create separate token repository for provider auth service
    let token_read_repo_provider = TokenReadRepositoryImpl::new(db_pool.get_read_connection());
    let token_write_repo_provider = TokenWriteRepositoryImpl::new(db_pool.get_write_connection());
    let token_repo_provider = CombinedTokenRepository::new(token_read_repo_provider, token_write_repo_provider);
    
    let provider_auth_service = domain::service::oauth_service::OAuthService::new(
        user_repo.clone(),
        token_repo_provider,
        domain::service::token_service::TokenService::new(
            Box::new(MinimalJwtTokenEncoder),
            Duration::hours(1)
        )
    );
    let provider_usecase = ProviderUseCaseImpl::new(Arc::new(provider_auth_service));

    // Create separate instances for command service
    let user_usecase_for_commands = UserUseCaseImpl::new(
        Arc::new(user_repo),
        Arc::new(user_email_repo),
        token_service.clone(),
    );
    
    let token_usecase_for_commands = TokenUseCaseImpl::new(
        Arc::new(refresh_token_repo),
        token_service,
    );

    // Create command registry and service
    let registry = CommandRegistryFactory::create_iam_registry(
        Arc::new(login_usecase),
        Arc::new(link_provider_usecase),
        Arc::new(provider_usecase),
        Arc::new(token_usecase_for_commands),
        Arc::new(user_usecase_for_commands),
        Arc::new(auth_usecase),
    );
    let command_service = Arc::new(GenericCommandService::new(Arc::new(registry)));

    // Create app state
    let app_state = AppState::new(
        command_service,
        Arc::new(user_usecase),
        Arc::new(token_usecase),
        config.oauth.clone(),
    );

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
        info!("Starting HTTPS server on {}:{}", server_config.host, server_config.tls_port.unwrap());
    } else {
        info!("Starting HTTP server on {}:{}", server_config.host, server_config.port);
    }
    
    serve_with_config(app_state, server_config).await?;

    Ok(())
}
