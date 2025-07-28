//! # Telegraph HTTP
//! 
//! HTTP layer for the Telegraph communication service.
//! This crate contains HTTP handlers, validation, and error handling
//! for the Telegraph API endpoints.

use rustycog_http::{RouteBuilder, AppState};
use rustycog_config::ServerConfig;

pub mod handlers;
pub mod error;
pub mod validation;

// Re-export commonly used types
pub use handlers::*;
pub use error::*;
pub use validation::*;

/// Create and start the Telegraph HTTP server
pub async fn create_app_routes(state: AppState, config: ServerConfig) -> anyhow::Result<()> {
    RouteBuilder::new(state)
        .health_check()
        .authenticated_get("/api/notifications", handlers::notification::get_notifications)
        .authenticated_get("/api/notifications/unread-count", handlers::notification::get_unread_count)
        .authenticated_put("/api/notifications/{id}/read", handlers::notification::mark_notification_read)
        .build(config)
        .await
} 