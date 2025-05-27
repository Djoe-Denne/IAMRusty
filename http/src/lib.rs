//! HTTP layer: Axum web server and endpoints
//!
//! This crate provides the HTTP interface for the application,
//! implementing the OpenAPI specification.

use axum::{
    Router,
    routing::{get, post},
    middleware,
};
use std::sync::Arc;
use std::net::SocketAddr;
use application::{
    usecase::{
        user::UserUseCase,
        token::TokenUseCase,
    },
    command::service::DynCommandService,
};
use configuration::OAuthConfig;

pub mod handlers;
mod middleware_auth;
pub mod oauth_state;

pub use handlers::{
    auth::{oauth_callback, oauth_start},
    user::get_user,
    token::refresh_token,
};
pub use middleware_auth::auth;
use middleware_auth::auth as auth_middleware;

/// Application state for HTTP handlers
#[derive(Clone)]
pub struct AppState {
    /// Command service for handling commands with cross-cutting concerns
    pub command_service: Arc<DynCommandService>,
    /// User use case
    pub user_usecase: Arc<dyn UserUseCase>,
    /// Token use case
    pub token_usecase: Arc<dyn TokenUseCase>,
    /// OAuth configuration for accessing redirect URIs
    pub oauth_config: OAuthConfig,
}

impl AppState {
    /// Create a new AppState
    pub fn new(
        command_service: Arc<DynCommandService>,
        user_usecase: Arc<dyn UserUseCase>,
        token_usecase: Arc<dyn TokenUseCase>,
        oauth_config: OAuthConfig,
    ) -> Self {
        Self {
            command_service,
            user_usecase,
            token_usecase,
            oauth_config,
        }
    }
}

/// Server configuration for HTTP/HTTPS
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub tls_port: Option<u16>,
}

/// Start the HTTP server
pub async fn serve(state: AppState, addr: &str) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/api/auth/{provider}/start", get(oauth_start))
        .route("/api/auth/{provider}/callback", get(oauth_callback))
        .route("/api/token/refresh", post(refresh_token))
        .route(
            "/api/me",
            get(get_user).route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware)),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check handler
async fn health_check() -> &'static str {
    "OK"
}

/// Start the server with optional HTTPS support
pub async fn serve_with_config(state: AppState, config: ServerConfig) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/auth/{provider}/start", get(oauth_start))
        .route("/api/auth/{provider}/callback", get(oauth_callback))
        .route("/api/token/refresh", post(refresh_token))
        .route(
            "/api/me",
            get(get_user).route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware)),
        )

        .with_state(state);

    if config.tls_enabled {
        if let (Some(cert_path), Some(key_path), Some(tls_port)) = 
            (config.tls_cert_path, config.tls_key_path, config.tls_port) {
            
            tracing::info!("Starting HTTPS server on {}:{}", config.host, tls_port);
            
            let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path).await?;
            let addr: SocketAddr = format!("{}:{}", config.host, tls_port).parse()?;
            
            axum_server::bind_rustls(addr, tls_config)
                .serve(app.into_make_service())
                .await?;
        } else {
            return Err(anyhow::anyhow!("TLS enabled but certificate/key paths or port not provided"));
        }
    } else {
        tracing::info!("Starting HTTP server on {}:{}", config.host, config.port);
        let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
} 