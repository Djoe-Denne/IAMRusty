//! Core error types for RustyCog
//! 
//! This module defines the fundamental error types used throughout the RustyCog ecosystem.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Core service error type that all RustyCog services should use
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ServiceError {
    /// Validation error - input data is invalid
    #[error("Validation error: {message}")]
    Validation { 
        message: String,
        field: Option<String>,
        code: Option<String>,
    },

    /// Authentication error - user is not authenticated
    #[error("Authentication error: {message}")]
    Authentication { 
        message: String,
        code: Option<String>,
    },

    /// Authorization error - user is not authorized to perform this action
    #[error("Authorization error: {message}")]
    Authorization { 
        message: String,
        resource: Option<String>,
        action: Option<String>,
    },

    /// Business logic error - domain rules violated
    #[error("Business error: {message}")]
    Business { 
        message: String,
        code: Option<String>,
    },

    /// Infrastructure error - external systems, database, etc.
    #[error("Infrastructure error: {message}")]
    Infrastructure { 
        message: String,
        error_source: Option<String>,
    },

    /// Not found error - requested resource doesn't exist
    #[error("Not found: {message}")]
    NotFound { 
        message: String,
        resource_type: Option<String>,
        resource_id: Option<String>,
    },

    /// Conflict error - resource already exists or state conflict
    #[error("Conflict: {message}")]
    Conflict { 
        message: String,
        resource_type: Option<String>,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {message}")]
    RateLimit { 
        message: String,
        retry_after: Option<u64>,
    },

    /// Service unavailable - temporary failure
    #[error("Service unavailable: {message}")]
    ServiceUnavailable { 
        message: String,
        retry_after: Option<u64>,
    },

    /// Timeout error
    #[error("Timeout: {message}")]
    Timeout { 
        message: String,
        operation: Option<String>,
    },

    /// Internal error - unexpected system error
    #[error("Internal error: {message}")]
    Internal { 
        message: String,
        error_id: Option<String>,
    },
}

impl ServiceError {
    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            field: None,
            code: None,
        }
    }

    /// Create a validation error with field information
    pub fn validation_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            field: Some(field.into()),
            code: None,
        }
    }

    /// Create a validation error with field and code
    pub fn validation_with_code(
        message: impl Into<String>, 
        field: impl Into<String>, 
        code: impl Into<String>
    ) -> Self {
        Self::Validation {
            message: message.into(),
            field: Some(field.into()),
            code: Some(code.into()),
        }
    }

    /// Create an authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
            code: None,
        }
    }

    /// Create an authentication error with code
    pub fn authentication_with_code(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
            code: Some(code.into()),
        }
    }

    /// Create an authorization error
    pub fn authorization(message: impl Into<String>) -> Self {
        Self::Authorization {
            message: message.into(),
            resource: None,
            action: None,
        }
    }

    /// Create a business logic error
    pub fn business(message: impl Into<String>) -> Self {
        Self::Business {
            message: message.into(),
            code: None,
        }
    }

    /// Create a business logic error with code
    pub fn business_with_code(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::Business {
            message: message.into(),
            code: Some(code.into()),
        }
    }

    /// Create an infrastructure error
    pub fn infrastructure(message: impl Into<String>) -> Self {
        Self::Infrastructure {
            message: message.into(),
            error_source: None,
        }
    }

    /// Create an infrastructure error with source
    pub fn infrastructure_with_source(message: impl Into<String>, source: impl Into<String>) -> Self {
        Self::Infrastructure {
            message: message.into(),
            error_source: Some(source.into()),
        }
    }

    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
            resource_type: None,
            resource_id: None,
        }
    }

    /// Create a not found error with resource information
    pub fn not_found_resource(
        message: impl Into<String>, 
        resource_type: impl Into<String>, 
        resource_id: impl Into<String>
    ) -> Self {
        Self::NotFound {
            message: message.into(),
            resource_type: Some(resource_type.into()),
            resource_id: Some(resource_id.into()),
        }
    }

    /// Create a conflict error
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
            resource_type: None,
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            error_id: None,
        }
    }

    /// Get the error category for metrics and logging
    pub fn category(&self) -> &'static str {
        match self {
            Self::Validation { .. } => "validation",
            Self::Authentication { .. } => "authentication",
            Self::Authorization { .. } => "authorization",
            Self::Business { .. } => "business",
            Self::Infrastructure { .. } => "infrastructure",
            Self::NotFound { .. } => "not_found",
            Self::Conflict { .. } => "conflict",
            Self::RateLimit { .. } => "rate_limit",
            Self::ServiceUnavailable { .. } => "service_unavailable",
            Self::Timeout { .. } => "timeout",
            Self::Internal { .. } => "internal",
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Infrastructure { .. } | 
            Self::ServiceUnavailable { .. } | 
            Self::Timeout { .. } |
            Self::RateLimit { .. }
        )
    }

    /// Get the HTTP status code that should be returned for this error
    pub fn http_status_code(&self) -> u16 {
        match self {
            Self::Validation { .. } => 400,
            Self::Authentication { .. } => 401,
            Self::Authorization { .. } => 403,
            Self::NotFound { .. } => 404,
            Self::Conflict { .. } => 409,
            Self::Business { .. } => 422,
            Self::RateLimit { .. } => 429,
            Self::ServiceUnavailable { .. } => 503,
            Self::Timeout { .. } => 504,
            Self::Infrastructure { .. } | Self::Internal { .. } => 500,
        }
    }
}

/// Domain error type for business logic errors
/// 
/// This is a more specific error type that domain services can use
/// and will be mapped to ServiceError by the application layer.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum DomainError {
    /// Business rule violation
    #[error("Business rule violation: {message}")]
    BusinessRuleViolation { 
        message: String,
        rule: String,
    },

    /// Invalid state transition
    #[error("Invalid state transition: {message}")]
    InvalidStateTransition { 
        message: String,
        from_state: String,
        to_state: String,
    },

    /// Resource not found in domain
    #[error("Resource not found: {message}")]
    ResourceNotFound { 
        message: String,
        resource_type: String,
    },

    /// Invariant violation
    #[error("Invariant violation: {message}")]
    InvariantViolation { 
        message: String,
        invariant: String,
    },
}

impl From<DomainError> for ServiceError {
    fn from(domain_error: DomainError) -> Self {
        match domain_error {
            DomainError::BusinessRuleViolation { message, rule } => {
                ServiceError::business_with_code(message, rule)
            }
            DomainError::InvalidStateTransition { message, .. } => {
                ServiceError::business_with_code(message, "invalid_state_transition")
            }
            DomainError::ResourceNotFound { message, resource_type } => {
                ServiceError::not_found_resource(message, resource_type, "unknown")
            }
            DomainError::InvariantViolation { message, invariant } => {
                ServiceError::business_with_code(message, invariant)
            }
        }
    }
} 