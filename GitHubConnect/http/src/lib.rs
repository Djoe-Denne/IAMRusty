//! HTTP routes for GitHub Connect: health plus HMAC-protected `/v1/*`.

use std::sync::Arc;

use axum::Router;
use github_connect_configuration::ServerConfig;
use idp_connect_contract::server::{router as connect_router, ConnectState};
use readiness::{attach_ready, ReadinessProbe};
use rustycog::http::{AppState, RouteBuilder};

/// Bounded-context path for standalone serving (HMAC `PATH` includes this prefix).
pub const SERVICE_PREFIX: &str = "/github-connect";

/// Unprefixed router (`/health`, `/v1/authorize|token|profile`).
///
/// Nested under [`SERVICE_PREFIX`] by [`create_prefixed_router`]. HMAC uses
/// [`axum::extract::OriginalUri`], so the signed path is `/github-connect/v1/token`.
pub fn create_router(state: AppState, connect: ConnectState) -> Router {
    RouteBuilder::new(state)
        .health_check()
        .into_router()
        .merge(connect_router(connect))
}

/// Router nested under [`SERVICE_PREFIX`], with `GET /ready`.
pub fn create_prefixed_router(
    state: AppState,
    connect: ConnectState,
    probe: Arc<ReadinessProbe>,
) -> Router {
    Router::new().nest(
        SERVICE_PREFIX,
        attach_ready(create_router(state, connect), probe),
    )
}

/// Serve the prefixed router (standalone binary).
///
/// # Errors
///
/// Returns an error when the HTTP server cannot bind or serve.
pub async fn create_app_routes(
    state: AppState,
    connect: ConnectState,
    config: ServerConfig,
    probe: Arc<ReadinessProbe>,
) -> anyhow::Result<()> {
    rustycog::http::serve_router(create_prefixed_router(state, connect, probe), config).await
}
