//! HTTP layer: Axum web server and endpoints
//!
//! This crate provides the HTTP interface for the application,
//! implementing the OpenAPI specification.

use rustycog_http::{
    RouteBuilder, ServerConfig, AppState
};

pub mod error;
pub mod handlers;
pub mod oauth_state;
pub mod validation;

pub use error::{ApiError, AuthError};
pub use handlers::{
    auth::{
        check_username, complete_registration, internal_provider_token, jwks, login,
        oauth_callback, oauth_login_start, oauth_link_start, resend_verification_email, signup, verify_email,
        revoke_provider_token, relink_provider_callback, generate_relink_provider_start_url,
    },
    password_reset::{
        request_password_reset, validate_reset_token, reset_password_unauthenticated,
        reset_password_authenticated,
    },
    token::refresh_token,
    user::get_user,
};

/// Create the application routes using the fluent builder API
pub async fn create_app_routes(state: AppState, config: ServerConfig) -> anyhow::Result<()> {
    RouteBuilder::new(state.clone())
        .health_check()
        // Public authentication routes
        .get("/.well-known/jwks.json", jwks)
        .post("/api/auth/signup", signup)
        .post("/api/auth/login", login)
        .post("/api/auth/verify", verify_email)
        .post("/api/auth/resend-verification", resend_verification_email)
        .post("/api/auth/complete-registration", complete_registration)
        .get("/api/auth/username/check", check_username)
        .post("/api/auth/password/reset-request", request_password_reset)
        .post("/api/auth/password/reset-validate", validate_reset_token)
        .post("/api/auth/password/reset-confirm", reset_password_unauthenticated)
        .get("/api/auth/{provider_name}/login", oauth_login_start)
        .get("/api/auth/{provider_name}/callback", oauth_callback)
        .post("/api/token/refresh", refresh_token)
        .get("/api/auth/{provider_name}/relink-start", generate_relink_provider_start_url)
        // Authenticated routes
        .authenticated_get("/api/me", get_user)
        .authenticated_post("/api/auth/password/reset-authenticated", reset_password_authenticated)
        .authenticated_post("/internal/{provider_name}/token", internal_provider_token)
        .authenticated_delete("/internal/{provider_name}/revoke", revoke_provider_token)
        .authenticated_get("/api/auth/{provider_name}/link", oauth_link_start)
        .authenticated_get("/api/auth/{provider_name}/relink-callback", relink_provider_callback)
        .build(config).await
}
