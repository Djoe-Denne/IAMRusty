use rustycog_core::error::DomainError;
use thiserror::Error;

/// Application-specific errors
#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Validation error")]
    ValidationError(Vec<ValidationError>),

    #[error("External service error: {service}: {message}")]
    ExternalService { service: String, message: String },

    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    #[error("Internal application error: {message}")]
    Internal { message: String },
}

/// Validation error details
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub code: Option<String>,
}

impl ApplicationError {
    /// Create an external service error
    #[must_use]
    pub fn external_service_error(service: &str, message: &str) -> Self {
        Self::ExternalService {
            service: service.to_string(),
            message: message.to_string(),
        }
    }

    /// Create a rate limit error
    #[must_use]
    pub fn rate_limit(message: &str) -> Self {
        Self::RateLimit {
            message: message.to_string(),
        }
    }

    /// Create an internal error
    #[must_use]
    pub fn internal_error(message: &str) -> Self {
        Self::Internal {
            message: message.to_string(),
        }
    }

    /// Create a validation error
    #[must_use]
    pub const fn validation_error(errors: Vec<ValidationError>) -> Self {
        Self::ValidationError(errors)
    }

    /// Create a simple validation error with single field
    #[must_use]
    pub fn single_validation_error(field: &str, message: &str) -> Self {
        Self::ValidationError(vec![ValidationError {
            field: field.to_string(),
            message: message.to_string(),
            code: None,
        }])
    }
}

impl ValidationError {
    /// Create a new validation error
    #[must_use]
    pub const fn new(field: String, message: String, code: Option<String>) -> Self {
        Self {
            field,
            message,
            code,
        }
    }

    /// Create a validation error with just field and message
    #[must_use]
    pub fn simple(field: &str, message: &str) -> Self {
        Self {
            field: field.to_string(),
            message: message.to_string(),
            code: None,
        }
    }

    /// Create a validation error with a code
    #[must_use]
    pub fn with_code(field: &str, message: &str, code: &str) -> Self {
        Self {
            field: field.to_string(),
            message: message.to_string(),
            code: Some(code.to_string()),
        }
    }
}
