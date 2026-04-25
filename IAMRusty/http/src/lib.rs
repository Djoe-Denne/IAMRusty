//! HTTP layer: Axum web server and endpoints
//!
//! This crate provides the HTTP interface for the application,
//! implementing the OpenAPI specification.

use axum::Router;
use iam_configuration::ServerConfig;
use rustycog_http::{AppState, RouteBuilder};

pub mod error;
pub mod handlers;
pub mod oauth_state;
pub mod validation;

pub use error::{ApiError, AuthError};
pub use handlers::{
    auth::{
        check_username, complete_registration, generate_relink_provider_start_url,
        internal_provider_token, jwks, login, oauth_callback, oauth_link_start, oauth_login_start,
        relink_provider_callback, resend_verification_email, revoke_provider_token, signup,
        verify_email,
    },
    password_reset::{
        request_password_reset, reset_password_authenticated, reset_password_unauthenticated,
        validate_reset_token,
    },
    token::refresh_token,
    user::get_user,
};

pub const SERVICE_PREFIX: &str = "/iam";

/// Create the application routes using the fluent builder API
pub fn create_router(state: AppState) -> Router {
    RouteBuilder::new(state.clone())
        .health_check()
        // Public authentication routes
        .get("/.well-known/jwks.json", jwks)
        .post("/api/auth/signup", signup)
        .post("/api/auth/login", login)
        .get("/api/auth/verify", verify_email)
        .post("/api/auth/resend-verification", resend_verification_email)
        .post("/api/auth/complete-registration", complete_registration)
        .get("/api/auth/username/check", check_username)
        .post("/api/auth/password/reset-request", request_password_reset)
        .post("/api/auth/password/reset-validate", validate_reset_token)
        .post(
            "/api/auth/password/reset-confirm",
            reset_password_unauthenticated,
        )
        .get("/api/auth/{provider_name}/login", oauth_login_start)
        .get("/api/auth/{provider_name}/callback", oauth_callback)
        .post("/api/token/refresh", refresh_token)
        .get(
            "/api/auth/{provider_name}/relink-start",
            generate_relink_provider_start_url,
        )
        // Authenticated routes
        .get("/api/me", get_user)
        .authenticated()
        .post(
            "/api/auth/password/reset-authenticated",
            reset_password_authenticated,
        )
        .authenticated()
        .post("/internal/{provider_name}/token", internal_provider_token)
        .authenticated()
        .delete("/internal/{provider_name}/revoke", revoke_provider_token)
        .authenticated()
        .get("/api/auth/{provider_name}/link", oauth_link_start)
        .authenticated()
        .get(
            "/api/auth/{provider_name}/relink-callback",
            relink_provider_callback,
        )
        .authenticated()
        .into_router()
}

/// Create the IAM router under its bounded-context prefix.
pub fn create_prefixed_router(state: AppState) -> Router {
    Router::new().nest(SERVICE_PREFIX, create_router(state))
}

/// Create and start the application routes using the fluent builder API.
pub async fn create_app_routes(state: AppState, config: ServerConfig) -> anyhow::Result<()> {
    rustycog_http::serve_router(create_prefixed_router(state), config).await
}
