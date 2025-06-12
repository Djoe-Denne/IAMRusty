//! HTTP layer: Axum web server and endpoints
//!
//! This crate provides the HTTP interface for the application,
//! implementing the OpenAPI specification.

use axum::{
    Router,
    routing::{get, post},
    middleware,
    http::StatusCode,
    response::{Json, IntoResponse},
};
use tower_http::catch_panic::CatchPanicLayer;
use serde_json::json;
use std::sync::Arc;
use std::net::SocketAddr;
use application::{
    usecase::{
        user::UserUseCase,
        token::TokenUseCase,
    },
    command::GenericCommandService,
};
use configuration::OAuthConfig;

pub mod handlers;
pub mod error;
pub mod extractors;
mod middleware_auth;
pub mod oauth_state;
pub mod validation;

pub use handlers::{
    auth::{oauth_callback, oauth_start, signup, login, verify_email, resend_verification_email, internal_provider_token, jwks},
    user::get_user,
    token::refresh_token,
};
pub use middleware_auth::auth;
pub use error::{ApiError, AuthError, UniformErrorResponse, ValidationError};
pub use extractors::ValidatedJson;
use middleware_auth::auth as auth_middleware;

/// Application state for HTTP handlers
#[derive(Clone)]
pub struct AppState {
    /// Command service for handling commands with cross-cutting concerns
    pub command_service: Arc<GenericCommandService>,
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
        command_service: Arc<GenericCommandService>,
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

/// Health check handler
async fn health_check() -> &'static str {
    "OK"
}

/// Handle panic in middleware
fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> axum::response::Response {
    let details = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic".to_string()
    };

    tracing::error!("Service panicked: {}", details);

    let body = Json(json!({
        "error": {
            "message": "Internal server error",
            "status": 500,
        }
    }));

    (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
}

/// Start the server with optional HTTPS support
pub async fn serve_with_config(state: AppState, config: ServerConfig) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/.well-known/jwks.json", get(jwks))
        .route("/api/auth/signup", post(signup))
        .route("/api/auth/login", post(login))
        .route("/api/auth/verify", post(verify_email))
        .route("/api/auth/resend-verification", post(resend_verification_email))
        .route("/api/auth/{provider_name}/start", get(oauth_start))
        .route("/api/auth/{provider_name}/callback", get(oauth_callback))
        .route("/api/token/refresh", post(refresh_token))
        .route(
            "/api/me",
            get(get_user).route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware)),
        )
        .route(
            "/internal/{provider_name}/token",
            post(internal_provider_token).route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware)),
        )
        .layer(CatchPanicLayer::custom(handle_panic))
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