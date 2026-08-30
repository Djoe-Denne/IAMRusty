//! # Telegraph HTTP
//!
//! HTTP layer for the Telegraph communication service.
//! This crate contains HTTP handlers, validation, and error handling
//! for the Telegraph API endpoints.

use axum::Router;
use readiness::{attach_ready, ReadinessProbe};
use rustycog::config::ServerConfig;
use rustycog::http::{AppState, RouteBuilder};
use rustycog::permission::Permission;
use std::sync::Arc;

pub mod error;
pub mod handlers;
pub mod validation;

pub use error::*;
pub use handlers::*;
pub use validation::*;

pub const SERVICE_PREFIX: &str = "/telegraph";

/// Create and start the Telegraph HTTP server.
///
/// Notification ownership is expressed in `OpenFGA` as
/// `notification:{id}#recipient@user:{user_id}` tuples written by
/// sentinel-sync on `NotificationCreated`. The route layer simply asks the
/// centralized checker whether the caller can write the notification.
pub fn create_router(state: AppState) -> Router {
    RouteBuilder::new(state)
        .health_check()
        .get(
            "/api/notifications",
            handlers::notification::get_notifications,
        )
        .authenticated()
        .get(
            "/api/notifications/unread-count",
            handlers::notification::get_unread_count,
        )
        .authenticated()
        .put(
            "/api/notifications/{id}/read",
            handlers::notification::mark_notification_read,
        )
        .authenticated()
        .with_permission_on(Permission::Write, "notification")
        .into_router()
}

/// Create the Telegraph router under its bounded-context prefix.
pub fn create_prefixed_router(state: AppState, probe: Arc<ReadinessProbe>) -> Router {
    Router::new().nest(SERVICE_PREFIX, attach_ready(create_router(state), probe))
}

/// Create and start the Telegraph HTTP server.
///
/// # Errors
///
/// Returns an error when the HTTP server cannot bind or serve the router.
pub async fn create_app_routes(
    state: AppState,
    config: ServerConfig,
    probe: Arc<ReadinessProbe>,
) -> anyhow::Result<()> {
    rustycog::http::serve_router(create_prefixed_router(state, probe), config).await
}
