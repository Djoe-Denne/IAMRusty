use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use chrono;
use hive_application::{ApiErrorResponse, ApplicationError};
use rustycog_core::error::DomainError;
use thiserror::Error;

/// HTTP-specific errors
#[derive(Debug, Error)]
pub enum HttpError {
    #[error("Application error: {0}")]
    Application(#[from] ApplicationError),

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Authentication required")]
    Unauthorized,

    #[error("Access forbidden")]
    Forbidden,

    #[error("Not found")]
    NotFound,

    #[error("Bad request: {message}")]
    BadRequest { message: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },

    #[error("Request entity too large")]
    PayloadTooLarge,

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Internal server error: {message}")]
    Internal { message: String },
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, error_response) = match self {
            Self::Application(app_error) => {
                // Convert ApplicationError to appropriate HTTP status and response
                match &app_error {
                    ApplicationError::Domain(domain_error) => match domain_error {
                        DomainError::EntityNotFound { .. } => {
                            (StatusCode::NOT_FOUND, ApiErrorResponse::from(app_error))
                        }
                        DomainError::InvalidInput { .. } => {
                            (StatusCode::BAD_REQUEST, ApiErrorResponse::from(app_error))
                        }
                        DomainError::BusinessRuleViolation { .. } => (
                            StatusCode::UNPROCESSABLE_ENTITY,
                            ApiErrorResponse::from(app_error),
                        ),
                        DomainError::Unauthorized { .. } => {
                            (StatusCode::UNAUTHORIZED, ApiErrorResponse::from(app_error))
                        }
                        DomainError::ResourceAlreadyExists { .. } => {
                            (StatusCode::CONFLICT, ApiErrorResponse::from(app_error))
                        }
                        DomainError::PermissionDenied { .. } => {
                            (StatusCode::FORBIDDEN, ApiErrorResponse::from(app_error))
                        }
                        DomainError::ExternalServiceError { .. } => {
                            (StatusCode::BAD_GATEWAY, ApiErrorResponse::from(app_error))
                        }
                        DomainError::Internal { .. } => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            ApiErrorResponse::from(app_error),
                        ),
                    },
                    ApplicationError::ValidationError(_) => {
                        (StatusCode::BAD_REQUEST, ApiErrorResponse::from(app_error))
                    }
                    ApplicationError::ExternalService { .. } => {
                        (StatusCode::BAD_GATEWAY, ApiErrorResponse::from(app_error))
                    }
                    ApplicationError::RateLimit { .. } => (
                        StatusCode::TOO_MANY_REQUESTS,
                        ApiErrorResponse::from(app_error),
                    ),
                    ApplicationError::Internal { .. } => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ApiErrorResponse::from(app_error),
                    ),
                }
            }
            Self::BadRequest { message } => (
                StatusCode::BAD_REQUEST,
                ApiErrorResponse {
                    error_type: "bad_request".to_string(),
                    message,
                    timestamp: chrono::Utc::now(),
                    request_id: None,
                    details: None,
                    validation_errors: None,
                },
            ),
            Self::Validation { message } => (
                StatusCode::BAD_REQUEST,
                ApiErrorResponse {
                    error_type: "validation_error".to_string(),
                    message,
                    timestamp: chrono::Utc::now(),
                    request_id: None,
                    details: None,
                    validation_errors: None,
                },
            ),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                ApiErrorResponse {
                    error_type: "unauthorized".to_string(),
                    message: "Authentication required".to_string(),
                    timestamp: chrono::Utc::now(),
                    request_id: None,
                    details: None,
                    validation_errors: None,
                },
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                ApiErrorResponse {
                    error_type: "forbidden".to_string(),
                    message: "Access forbidden".to_string(),
                    timestamp: chrono::Utc::now(),
                    request_id: None,
                    details: None,
                    validation_errors: None,
                },
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                ApiErrorResponse {
                    error_type: "not_found".to_string(),
                    message: "Resource not found".to_string(),
                    timestamp: chrono::Utc::now(),
                    request_id: None,
                    details: None,
                    validation_errors: None,
                },
            ),
            Self::Conflict { message } => (
                StatusCode::CONFLICT,
                ApiErrorResponse {
                    error_type: "conflict".to_string(),
                    message,
                    timestamp: chrono::Utc::now(),
                    request_id: None,
                    details: None,
                    validation_errors: None,
                },
            ),
            Self::PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                ApiErrorResponse {
                    error_type: "payload_too_large".to_string(),
                    message: "Request entity too large".to_string(),
                    timestamp: chrono::Utc::now(),
                    request_id: None,
                    details: None,
                    validation_errors: None,
                },
            ),
            Self::RateLimit => (
                StatusCode::TOO_MANY_REQUESTS,
                ApiErrorResponse {
                    error_type: "rate_limit_exceeded".to_string(),
                    message: "Rate limit exceeded".to_string(),
                    timestamp: chrono::Utc::now(),
                    request_id: None,
                    details: None,
                    validation_errors: None,
                },
            ),
            Self::Internal { message } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorResponse {
                    error_type: "internal_error".to_string(),
                    message,
                    timestamp: chrono::Utc::now(),
                    request_id: None,
                    details: None,
                    validation_errors: None,
                },
            ),
        };

        (status, Json(error_response)).into_response()
    }
}
