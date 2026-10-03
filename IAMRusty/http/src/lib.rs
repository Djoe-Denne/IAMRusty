//! HTTP layer: Axum web server and endpoints
//!
//! This crate provides the HTTP interface for the application,
//! implementing the `OpenAPI` specification.

use axum::{middleware, Extension, Router};
use iam_configuration::{IdpConfig, ServerConfig};
use readiness::{attach_ready, ReadinessProbe};
use rustycog::http::{AppState, RouteBuilder};
use std::sync::Arc;

use crate::handlers::mesh_echo::mesh_echo_headers;
use crate::rate_limit::rate_limit_auth;

pub mod error;
pub mod handlers;
pub mod idp_registry;
pub mod oauth_state;
pub mod rate_limit;
pub mod validation;

pub use error::{ApiError, AuthError};
pub use handlers::{
    auth::{
        check_username, complete_registration, generate_relink_provider_start_url,
        internal_provider_token, jwks, login, oauth_callback, oauth_link_start, oauth_login_start,
        relink_provider_callback, resend_verification_email, revoke_provider_token, signup,
        verify_email,
    },
    organization_signer::{
        configure_organization_signer, create_organization_managed_identity,
        disable_organization_signer, rotate_organization_signer, test_organization_signer,
        SignerRouteContext,
    },
    password_reset::{
        request_password_reset, reset_password_authenticated, reset_password_unauthenticated,
        validate_reset_token,
    },
    token::refresh_token,
    user::get_user,
};
pub use oauth_state::{configure_oauth_state_secret, OAuthState};
pub use rate_limit::configure_internal_service_token;

pub const SERVICE_PREFIX: &str = "/iam";

/// Create the application routes using the fluent builder API.
///
/// `idp` is attached as an axum [`Extension`] so OAuth handlers resolve
/// redirect URIs from this app instance, not a process-wide lock.
/// `signer` enables Hive→IAM org-signer internal RPC (ADR-0306).
pub fn create_router(
    state: AppState,
    idp: Arc<IdpConfig>,
    signer: Option<Arc<SignerRouteContext>>,
) -> Router {
    let builder = RouteBuilder::new(state)
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
        // Internal org-signer RPC — NOT `.authenticated()`; token gate only.
        .post(
            "/internal/organizations/{org_id}/signer/configure",
            configure_organization_signer,
        )
        .post(
            "/internal/organizations/{org_id}/signer/test",
            test_organization_signer,
        )
        .post(
            "/internal/organizations/{org_id}/signer/rotate",
            rotate_organization_signer,
        )
        .post(
            "/internal/organizations/{org_id}/signer/disable",
            disable_organization_signer,
        )
        .post(
            "/internal/organizations/{org_id}/identities",
            create_organization_managed_identity,
        )
        // Authenticated routes
        .get("/api/me", get_user)
        .authenticated()
        .get(
            "/api/auth/{provider_name}/relink-start",
            generate_relink_provider_start_url,
        )
        .authenticated()
        .post(
            "/api/auth/password/reset-authenticated",
            reset_password_authenticated,
        )
        .authenticated()
        // In mesh mode AuthUser comes from the envoy-mesh peer's principal,
        // not a user JWT. Both S2S handlers also require the internal token gate.
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
        .authenticated();

    let mut router = builder
        .into_router()
        .route(
            "/mesh/echo-headers",
            axum::routing::get(mesh_echo_headers).post(mesh_echo_headers),
        )
        .layer(middleware::from_fn(rate_limit_auth))
        .layer(Extension(idp));
    if let Some(signer) = signer {
        router = router.layer(Extension(signer));
    }
    router
}

/// Create the IAM router under its bounded-context prefix.
pub fn create_prefixed_router(
    state: AppState,
    probe: Arc<ReadinessProbe>,
    idp: Arc<IdpConfig>,
    signer: Option<Arc<SignerRouteContext>>,
) -> Router {
    Router::new().nest(
        SERVICE_PREFIX,
        attach_ready(create_router(state, idp, signer), probe),
    )
}

/// Create and start the application routes using the fluent builder API.
///
/// # Errors
///
/// Returns an error when the HTTP server cannot bind or serve the router.
pub async fn create_app_routes(
    state: AppState,
    config: ServerConfig,
    probe: Arc<ReadinessProbe>,
    idp: Arc<IdpConfig>,
    signer: Option<Arc<SignerRouteContext>>,
) -> anyhow::Result<()> {
    rustycog::http::serve_router(create_prefixed_router(state, probe, idp, signer), config).await
}
