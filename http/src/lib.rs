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
use application::usecase::{
    login::LoginUseCase,
    user::UserUseCase,
    token::TokenUseCase,
};

mod handlers;
mod middleware_auth;

use handlers::{
    auth::{oauth_callback, oauth_login},
    user::get_user,
    token::refresh_token,
};
use middleware_auth::auth;

/// Application state for HTTP handlers
#[derive(Clone)]
pub struct AppState {
    /// Login use case
    pub login_usecase: Arc<dyn LoginUseCase>,
    /// User use case
    pub user_usecase: Arc<dyn UserUseCase>,
    /// Token use case
    pub token_usecase: Arc<dyn TokenUseCase>,
}

impl AppState {
    /// Create a new AppState
    pub fn new(
        login_usecase: Arc<dyn LoginUseCase>,
        user_usecase: Arc<dyn UserUseCase>,
        token_usecase: Arc<dyn TokenUseCase>,
    ) -> Self {
        Self {
            login_usecase,
            user_usecase,
            token_usecase,
        }
    }
}

/// Start the HTTP server
pub async fn serve(state: AppState, addr: &str) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/api/auth/:provider/login", post(oauth_login))
        .route("/api/auth/:provider/callback", get(oauth_callback))
        .route("/api/token/refresh", post(refresh_token))
        .route(
            "/api/me",
            get(get_user).route_layer(middleware::from_fn_with_state(state.clone(), auth)),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
} 