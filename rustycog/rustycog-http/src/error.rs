//! HTTP error handling utilities

use axum::{http::StatusCode, response::IntoResponse, Json};
use rustycog_core::error::ServiceError;
use serde::Serialize;

/// HTTP error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetails {
    pub error_code: String,
    pub message: String,
    pub status: u16,
}

/// Wrapper for ServiceError to implement IntoResponse
pub struct HttpError(pub ServiceError);

impl From<ServiceError> for HttpError {
    fn from(error: ServiceError) -> Self {
        HttpError(error)
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        let status_code = StatusCode::from_u16(self.0.http_status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        
        let error_response = ErrorResponse {
            error: ErrorDetails {
                error_code: self.0.category().to_string(),
                message: self.0.to_string(),
                status: self.0.http_status_code(),
            },
        };
        
        (status_code, Json(error_response)).into_response()
    }
} 