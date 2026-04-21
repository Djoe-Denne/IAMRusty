//! # Telegraph HTTP
//!
//! HTTP layer for the Telegraph communication service.
//! This crate contains HTTP handlers, validation, and error handling
//! for the Telegraph API endpoints.

use rustycog_config::ServerConfig;
use rustycog_http::{AppState, RouteBuilder};
use rustycog_permission::Permission;

pub mod error;
pub mod handlers;
pub mod validation;

pub use error::*;
pub use handlers::*;
pub use validation::*;

/// Create and start the Telegraph HTTP server.
///
/// Notification ownership is expressed in OpenFGA as
/// `notification:{id}#recipient@user:{user_id}` tuples written by
/// sentinel-sync on `NotificationCreated`. The route layer simply asks the
/// centralized checker whether the caller can write the notification.
pub async fn create_app_routes(state: AppState, config: ServerConfig) -> anyhow::Result<()> {
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
        .build(config)
        .await
}
