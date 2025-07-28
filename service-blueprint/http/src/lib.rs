//! HTTP layer: Axum web server and endpoints
//!
//! This crate provides the HTTP interface for the application,
//! implementing the RESTful API specification.

use rustycog_config::ServerConfig;
use rustycog_http::{RouteBuilder, AppState};

pub mod error;
pub mod handlers;
pub mod validation;

pub use error::{ApiError, HttpError};
pub use handlers::*;
pub use validation::*;

/// Create the application routes using the fluent builder API
pub async fn create_app_routes(state: AppState, config: ServerConfig) -> anyhow::Result<()> {
    RouteBuilder::new(state.clone())
        .health_check()
        // Public routes
        .get("/api/entities", handlers::entity::list_entities)
        .post("/api/entities", handlers::entity::create_entity)
        .get("/api/entities/{id}", handlers::entity::get_entity)
        .put("/api/entities/{id}", handlers::entity::update_entity)
        .delete("/api/entities/{id}", handlers::entity::delete_entity)
        
        // Entity management routes
        .post("/api/entities/{id}/activate", handlers::entity::activate_entity)
        .post("/api/entities/{id}/deactivate", handlers::entity::deactivate_entity)
        .post("/api/entities/{id}/archive", handlers::entity::archive_entity)
        
        // Search routes
        .get("/api/entities/search", handlers::entity::search_entities)
        .get("/api/entities/count", handlers::entity::get_entity_count)
        
        .build(config)
        .await
} 