//! HTTP error handling for Telegraph

use serde::Serialize;
use thiserror::Error;

/// HTTP-specific errors for Telegraph
#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Domain error: {0}")]
    Domain(#[from] telegraph_domain::DomainError),

    #[error("Internal server error")]
    Internal,
}

/// Error response for API endpoints
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
