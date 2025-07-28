use thiserror::Error;

/// Domain-specific errors
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DomainError {
    /// Entity not found
    #[error("Entity not found: {entity_type} with id {id}")]
    EntityNotFound {
        entity_type: String,
        id: String,
    },

    /// Invalid input
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    /// Business rule violation
    #[error("Business rule violation: {rule}")]
    BusinessRuleViolation { rule: String },

    /// Unauthorized operation
    #[error("Unauthorized operation: {operation}")]
    Unauthorized { operation: String },

    /// Resource already exists
    #[error("Resource already exists: {resource_type} with {identifier}")]
    ResourceAlreadyExists {
        resource_type: String,
        identifier: String,
    },

    /// External service error
    #[error("External service error: {service}: {message}")]
    ExternalServiceError { service: String, message: String },

    /// Internal domain error
    #[error("Internal domain error: {message}")]
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

    /// Create an internal error
    pub fn internal(message: &str) -> Self {
        Self::Internal {
            message: message.to_string(),
        }
    }
} 