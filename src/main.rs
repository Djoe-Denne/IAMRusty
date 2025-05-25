use infra::{
    auth::{GitHubOAuth2Client, GitLabOAuth2Client},
    token::JwtTokenService,
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
    },
    config::load_config,
    db::DbConnectionPool,
};
use std::sync::Arc;
use application::{
    usecase::{
        login::LoginUseCaseImpl,
        user::UserUseCaseImpl,
        token::TokenUseCaseImpl,
        link_provider::LinkProviderUseCaseImpl,
    }
};

use http_server::{AppState, serve_with_config, ServerConfig};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration first
    let config = load_config()?;
    
    // Initialize tracing with config-based log level
    let log_level = match config.logging.level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.logging.level))
        )
        .init();

    info!("Starting IAM service...");
    info!("Configuration loaded with log level: {}", config.logging.level);

    // Setup database connection pool
    let db_pool = DbConnectionPool::new(&config.database.url, config.database.read_replicas.clone()).await?;
    info!("Database connection pool initialized with {} read replicas", 
          if config.database.read_replicas.is_empty() { 0 } else { config.database.read_replicas.len() });

    // Create repositories
    let user_read_repo = UserReadRepositoryImpl::new(db_pool.get_read_connection());
    let user_write_repo = UserWriteRepositoryImpl::new(db_pool.get_write_connection());
    let user_repo = CombinedUserRepository::new(user_read_repo, user_write_repo);

    let user_email_read_repo = UserEmailReadRepositoryImpl::new(db_pool.get_read_connection());
    let user_email_write_repo = UserEmailWriteRepositoryImpl::new(db_pool.get_write_connection());
    let user_email_repo = CombinedUserEmailRepository::new(user_email_read_repo, user_email_write_repo);

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

    // Create token service
    let token_service = JwtTokenService::new(
        config.jwt.secret.clone(),
        config.jwt.expiration_seconds,
    );

    // Create use cases
    let login_usecase = LoginUseCaseImpl::new(
        Arc::new(github_auth_login),
        Arc::new(gitlab_auth_login),
        Arc::new(user_repo.clone()),
        Arc::new(user_email_repo.clone()),
        Arc::new(token_repo_login),
        Arc::new(refresh_token_repo.clone()),
        Arc::new(token_service.clone()),
    );

    let link_provider_usecase = LinkProviderUseCaseImpl::new(
        Arc::new(github_auth_link),
        Arc::new(gitlab_auth_link),
        Arc::new(user_repo.clone()),
        Arc::new(user_email_repo.clone()),
        Arc::new(token_repo_link),
        Arc::new(refresh_token_repo.clone()),
        Arc::new(token_service.clone()),
    );

    let user_usecase = UserUseCaseImpl::new(
        Arc::new(user_repo),
        Arc::new(user_email_repo),
        Arc::new(token_service.clone()),
    );
    
    let token_usecase = TokenUseCaseImpl::new(
        Arc::new(refresh_token_repo),
        Arc::new(token_service),
    );

    // Create app state
    let app_state = AppState::new(
        Arc::new(login_usecase),
        Arc::new(user_usecase),
        Arc::new(token_usecase),
        Arc::new(link_provider_usecase),
        config.oauth.clone(),
    );

    // Create server configuration
    let server_config = ServerConfig {
        host: config.server.host.clone(),
        port: config.server.port,
        tls_enabled: config.server.tls_enabled,
        tls_cert_path: if config.server.tls_enabled { Some(config.server.tls_cert_path.clone()) } else { None },
        tls_key_path: if config.server.tls_enabled { Some(config.server.tls_key_path.clone()) } else { None },
        tls_port: if config.server.tls_enabled { Some(config.server.tls_port) } else { None },
    };

    // Start server (HTTP or HTTPS based on configuration)
    if config.server.tls_enabled {
        info!("Starting HTTPS server on {}:{}", config.server.host, config.server.tls_port);
    } else {
        info!("Starting HTTP server on {}:{}", config.server.host, config.server.port);
    }
    
    serve_with_config(app_state, server_config).await?;

    Ok(())
} 