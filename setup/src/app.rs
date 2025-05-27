use std::sync::Arc;
use anyhow::Result;
use tracing::info;

use http_server::{AppState, serve_with_config, ServerConfig as HttpServerConfig};
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
    db::DbConnectionPool,
};

use configuration::AppConfig;

use application::{
    usecase::{
        login::LoginUseCaseImpl,
        user::UserUseCaseImpl,
        token::TokenUseCaseImpl,
        link_provider::LinkProviderUseCaseImpl,
    }
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
