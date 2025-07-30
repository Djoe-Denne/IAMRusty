//! Generic HTTP framework built on top of Axum
//!
//! This crate provides reusable HTTP components and utilities for building
//! web APIs with consistent error handling, validation, and middleware.

pub mod builder;
pub mod error;
pub mod extractors;
pub mod jwt;
pub mod jwt_handler;
pub mod middleware_auth;

pub use builder::{AppState, RouteBuilder};
pub use error::{GenericHttpError, ValidationError};
pub use extractors::ValidatedJson;
pub use jwt::TokenClaims;
pub use jwt_handler::{UserIdExtractionHandler, UserIdExtractor};
pub use middleware_auth::{auth_middleware, AuthUser};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::json;

/// Health check handler
pub async fn health_check() -> &'static str {
    "OK"
}

/// Handle panic in middleware
pub fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> axum::response::Response {
    let details = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic".to_string()
    };

    tracing::error!("Service panicked: {}", details);

    let body = Json(json!({
        "error": {
            "message": "Internal server error",
            "status": 500,
        }
    }));

    (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
}
