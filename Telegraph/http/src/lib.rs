//! # Telegraph HTTP
//!
//! HTTP layer for the Telegraph communication service.
//! This crate contains HTTP handlers, validation, and error handling
//! for the Telegraph API endpoints.

use rustycog_config::ServerConfig;
use rustycog_http::{AppState, RouteBuilder};
use rustycog_permission::PermissionsFetcher;
use rustycog_permission::Permission;
use std::sync::Arc;

pub mod error;
pub mod handlers;
pub mod validation;

// Re-export commonly used types
pub use error::*;
pub use handlers::*;
pub use validation::*;

/// Create and start the Telegraph HTTP server
pub async fn create_app_routes(state: AppState, config: ServerConfig, permission_fetcher: Arc<dyn PermissionsFetcher>) -> anyhow::Result<()> {
    RouteBuilder::new(state)
        .health_check()
        .permissions_dir(std::path::Path::new("resources/permissions").to_path_buf())
        .resource("notification")
        .with_permission_fetcher(permission_fetcher)
        .get(
            "/api/notifications",
            handlers::notification::get_notifications,
        ).authenticated()
        .get(
            "/api/notifications/unread-count",
            handlers::notification::get_unread_count,
        ).authenticated()
        .put(
            "/api/notifications/{id}/read",
            handlers::notification::mark_notification_read,
        ).authenticated()
        .with_permission(Permission::Write)
        .build(config)
        .await
}
