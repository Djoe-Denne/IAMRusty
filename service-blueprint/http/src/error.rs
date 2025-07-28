use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

use {{SERVICE_NAME}}_application::{ApplicationError, ValidationError};
use {{SERVICE_NAME}}_domain::DomainError;

/// HTTP-specific errors
#[derive(Debug, Error)]
pub enum HttpError {
    #[error("Application error: {0}")]
    Application(#[from] ApplicationError),

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Not found: {resource}")]
    NotFound { resource: String },

    #[error("Bad request: {message}")]
    BadRequest { message: String },

    #[error("Internal server error")]
    InternalServerError,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,
}

/// API error response structure
#[derive(Debug, serde::Serialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_errors: Option<Vec<ValidationError>>,
}

impl HttpError {
    pub fn not_found(resource: &str) -> Self {
        Self::NotFound {
            resource: resource.to_string(),
        }
    }

    pub fn bad_request(message: &str) -> Self {
        Self::BadRequest {
            message: message.to_string(),
        }
    }

    pub fn validation_error(message: &str) -> Self {
        Self::Validation {
            message: message.to_string(),
        }
    }
}

impl From<DomainError> for HttpError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::EntityNotFound { .. } => HttpError::NotFound {
                resource: "Entity".to_string(),
            },
            DomainError::InvalidInput { .. } => HttpError::BadRequest {
                message: error.to_string(),
            },
            DomainError::BusinessRuleViolation { .. } => HttpError::BadRequest {
                message: error.to_string(),
            },
            DomainError::Unauthorized { .. } => HttpError::Unauthorized,
            DomainError::ResourceAlreadyExists { .. } => HttpError::BadRequest {
                message: error.to_string(),
            },
            _ => HttpError::InternalServerError,
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status_code, error_type, message, details, validation_errors) = match self {
            HttpError::Application(ApplicationError::Domain(domain_error)) => {
                match domain_error {
                    DomainError::EntityNotFound { entity_type, id } => (
                        StatusCode::NOT_FOUND,
                        "entity_not_found",
                        format!("{} not found", entity_type),
                        Some(json!({ "id": id })),
                        None,
                    ),
                    DomainError::InvalidInput { message } => (
                        StatusCode::BAD_REQUEST,
                        "invalid_input",
                        message,
                        None,
                        None,
                    ),
                    DomainError::BusinessRuleViolation { rule } => (
                        StatusCode::BAD_REQUEST,
                        "business_rule_violation",
                        rule,
                        None,
                        None,
                    ),
                    DomainError::Unauthorized { operation } => (
                        StatusCode::UNAUTHORIZED,
                        "unauthorized",
                        format!("Unauthorized: {}", operation),
                        None,
                        None,
                    ),
                    DomainError::ResourceAlreadyExists { resource_type, identifier } => (
                        StatusCode::CONFLICT,
                        "resource_already_exists",
                        format!("{} already exists", resource_type),
                        Some(json!({ "identifier": identifier })),
                        None,
                    ),
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "internal_error",
                        "An internal error occurred".to_string(),
                        None,
                        None,
                    ),
                }
            }
            HttpError::Application(ApplicationError::ValidationError(validation_errors)) => (
                StatusCode::BAD_REQUEST,
                "validation_error",
                "Input validation failed".to_string(),
                None,
                Some(validation_errors),
            ),
            HttpError::Application(ApplicationError::ExternalService { service, message }) => (
                StatusCode::BAD_GATEWAY,
                "external_service_error",
                format!("External service error: {}", service),
                Some(json!({ "service": service, "error": message })),
                None,
            ),
            HttpError::Application(ApplicationError::Internal { message }) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "An internal error occurred".to_string(),
                Some(json!({ "error": message })),
                None,
            ),
            HttpError::Validation { message } => (
                StatusCode::BAD_REQUEST,
                "validation_error",
                message,
                None,
                None,
            ),
            HttpError::NotFound { resource } => (
                StatusCode::NOT_FOUND,
                "not_found",
                format!("{} not found", resource),
                None,
                None,
            ),
            HttpError::BadRequest { message } => (
                StatusCode::BAD_REQUEST,
                "bad_request",
                message,
                None,
                None,
            ),
            HttpError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "An internal error occurred".to_string(),
                None,
                None,
            ),
            HttpError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Unauthorized".to_string(),
                None,
                None,
            ),
            HttpError::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "Forbidden".to_string(),
                None,
                None,
            ),
        };

        let api_error = ApiError {
            error: error_type.to_string(),
            message,
            details,
            validation_errors,
        };

        (status_code, Json(api_error)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_error_conversion() {
        let domain_error = DomainError::entity_not_found("User", "123");
        let http_error = HttpError::from(domain_error);
        
        assert!(matches!(http_error, HttpError::NotFound { .. }));
    }

    #[test]
    fn test_api_error_serialization() {
        let api_error = ApiError {
            error: "test_error".to_string(),
            message: "Test message".to_string(),
            details: Some(json!({ "key": "value" })),
            validation_errors: None,
        };

        let json = serde_json::to_string(&api_error).unwrap();
        assert!(json.contains("test_error"));
        assert!(json.contains("Test message"));
    }
} 