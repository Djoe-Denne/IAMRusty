//! HTTP routes for Lazaret: health plus workload enroll/session (no IAM middleware).

use std::sync::Arc;

use axum::extract::Extension;
use axum::Router;
use lazaret_application::{IdentityService, InvokeService};
use lazaret_configuration::ServerConfig;
use readiness::{attach_ready, ReadinessProbe};
use rustycog::http::{AppState, RouteBuilder};
use tower::layer::util::Identity;

mod error;
mod handlers;
mod invoke;

/// Bounded-context path for standalone and monolith embedding.
pub const SERVICE_PREFIX: &str = "/lazaret";

/// Unprefixed router (`/health`, `/enroll`, `/session`). Nested under [`SERVICE_PREFIX`] by the monolith.
pub fn create_router(
    state: AppState,
    identity: Arc<IdentityService>,
    invoke: Arc<InvokeService>,
) -> Router {
    RouteBuilder::new(state)
        .health_check()
        .post("/enroll", handlers::enroll)
        .post("/session", handlers::session)
        .post("/invoke", invoke::invoke)
        .into_router()
        .layer(Extension(identity))
        .layer(Extension(invoke))
        // rustycog HTTP does not expose a TLS client certificate. Cleartext
        // requests therefore never carry `VerifiedClientCertificate`; `/session`
        // returns 401 until a terminator or test inserts the extension.
        .layer(Identity::new())
}

/// Router nested under [`SERVICE_PREFIX`], with `GET /ready`.
pub fn create_prefixed_router(
    state: AppState,
    probe: Arc<ReadinessProbe>,
    identity: Arc<IdentityService>,
    invoke: Arc<InvokeService>,
) -> Router {
    Router::new().nest(
        SERVICE_PREFIX,
        attach_ready(create_router(state, identity, invoke), probe),
    )
}

/// Serve the prefixed router (standalone binary).
///
/// # Errors
///
/// Returns an error when the HTTP server cannot bind or serve.
pub async fn create_app_routes(
    state: AppState,
    config: ServerConfig,
    probe: Arc<ReadinessProbe>,
    identity: Arc<IdentityService>,
    invoke: Arc<InvokeService>,
) -> anyhow::Result<()> {
    rustycog::http::serve_router(
        create_prefixed_router(state, probe, identity, invoke),
        config,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::SERVICE_PREFIX;

    #[test]
    fn service_prefix_nests_health_and_ready() {
        assert_eq!(SERVICE_PREFIX, "/lazaret");
        assert_eq!(format!("{SERVICE_PREFIX}/health"), "/lazaret/health");
        assert_eq!(format!("{SERVICE_PREFIX}/ready"), "/lazaret/ready");
    }
}
