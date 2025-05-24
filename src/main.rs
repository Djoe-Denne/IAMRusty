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
    }
};

use http_server::{AppState, serve_with_config, ServerConfig};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("Starting IAM service...");

    // Load configuration
    let config = load_config()?;
    info!("Configuration loaded");

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
    let token_repo = CombinedTokenRepository::new(token_read_repo, token_write_repo);
    
    let refresh_token_read_repo = RefreshTokenReadRepositoryImpl::new(db_pool.get_read_connection());
    let refresh_token_write_repo = RefreshTokenWriteRepositoryImpl::new(db_pool.get_write_connection());
    let refresh_token_repo = CombinedRefreshTokenRepository::new(refresh_token_read_repo, refresh_token_write_repo);

    // Create auth services
    let github_auth = GitHubOAuth2Client::new(
        config.oauth.github.client_id.clone(),
        config.oauth.github.client_secret.clone(),
        config.oauth.github.redirect_uri.clone(),
    );
    let gitlab_auth = GitLabOAuth2Client::new(
        config.oauth.gitlab.client_id.clone(),
        config.oauth.gitlab.client_secret.clone(),
        config.oauth.gitlab.redirect_uri.clone(),
    );

    // Create token service
    let token_service = JwtTokenService::new(
        config.jwt.secret.clone(),
        config.jwt.expiration_seconds,
    );

    // Create use cases
    let login_usecase = LoginUseCaseImpl::new(
        Arc::new(github_auth),
        Arc::new(gitlab_auth),
        Arc::new(user_repo.clone()),
        Arc::new(user_email_repo.clone()),
        Arc::new(token_repo),
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