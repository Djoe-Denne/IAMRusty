use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use chrono;
use hive_application::{ApiErrorResponse, ApplicationError};
use rustycog::core::error::DomainError;
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

fn error_body(error_type: &str, message: impl Into<String>) -> ApiErrorResponse {
    ApiErrorResponse {
        error_type: error_type.to_string(),
        message: message.into(),
        timestamp: chrono::Utc::now(),
        request_id: None,
        details: None,
        validation_errors: None,
    }
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
            Self::BadRequest { message } => {
                (StatusCode::BAD_REQUEST, error_body("bad_request", message))
            }
            Self::Validation { message } => (
                StatusCode::BAD_REQUEST,
                error_body("validation_error", message),
            ),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                error_body("unauthorized", "Authentication required"),
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                error_body("forbidden", "Access forbidden"),
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                error_body("not_found", "Resource not found"),
            ),
            Self::Conflict { message } => (StatusCode::CONFLICT, error_body("conflict", message)),
            Self::PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                error_body("payload_too_large", "Request entity too large"),
            ),
            Self::RateLimit => (
                StatusCode::TOO_MANY_REQUESTS,
                error_body("rate_limit_exceeded", "Rate limit exceeded"),
            ),
            Self::Internal { message } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                error_body("internal_error", message),
            ),
        };

        (status, Json(error_response)).into_response()
    }
}
