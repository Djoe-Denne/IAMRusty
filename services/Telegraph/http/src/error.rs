//! HTTP error handling for Telegraph

use axum::http::StatusCode;
use rustycog::command::CommandError;
use serde::Serialize;
use telegraph_application::service_error_from_command_error;
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

/// Map a command error to the rustycog `ServiceError` HTTP status.
#[must_use]
pub fn status_code_for_command_error(error: &CommandError) -> StatusCode {
    StatusCode::from_u16(service_error_from_command_error(error).http_status_code())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}
