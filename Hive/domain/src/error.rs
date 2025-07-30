use thiserror::Error;

/// Domain-specific errors for the Hive service
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Entity not found: {entity_type} with id '{id}'")]
    EntityNotFound { entity_type: String, id: String },

    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    #[error("Business rule violation: {rule}")]
    BusinessRuleViolation { rule: String },

    #[error("Unauthorized: {operation}")]
    Unauthorized { operation: String },

    #[error("Resource already exists: {resource_type} with identifier '{identifier}'")]
    ResourceAlreadyExists {
        resource_type: String,
        identifier: String,
    },

    #[error("External service error: {service}: {message}")]
    ExternalServiceError { service: String, message: String },

    #[error("Concurrent access error: {message}")]
    ConcurrentAccess { message: String },

    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl DomainError {
    /// Create an entity not found error
    pub fn entity_not_found(entity_type: &str, id: &str) -> Self {
        Self::EntityNotFound {
            entity_type: entity_type.to_string(),
            id: id.to_string(),
        }
    }

    /// Create an invalid input error
    pub fn invalid_input(message: &str) -> Self {
        Self::InvalidInput {
            message: message.to_string(),
        }
    }

    /// Create a business rule violation error
    pub fn business_rule_violation(rule: &str) -> Self {
        Self::BusinessRuleViolation {
            rule: rule.to_string(),
        }
    }

    /// Create an unauthorized error
    pub fn unauthorized(operation: &str) -> Self {
        Self::Unauthorized {
            operation: operation.to_string(),
        }
    }

    /// Create a resource already exists error
    pub fn resource_already_exists(resource_type: &str, identifier: &str) -> Self {
        Self::ResourceAlreadyExists {
            resource_type: resource_type.to_string(),
            identifier: identifier.to_string(),
        }
    }

    /// Create an external service error
    pub fn external_service_error(service: &str, message: &str) -> Self {
        Self::ExternalServiceError {
            service: service.to_string(),
            message: message.to_string(),
        }
    }

    /// Create a concurrent access error
    pub fn concurrent_access(message: &str) -> Self {
        Self::ConcurrentAccess {
            message: message.to_string(),
        }
    }

    /// Create a permission denied error
    pub fn permission_denied(message: &str) -> Self {
        Self::PermissionDenied {
            message: message.to_string(),
        }
    }

    /// Create an internal error
    pub fn internal_error(message: &str) -> Self {
        Self::Internal {
            message: message.to_string(),
        }
    }
}
