//! Domain error types for Telegraph service

use thiserror::Error;

/// Domain-specific errors for Telegraph communication service
#[derive(Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    /// Invalid message content
    #[error("Invalid message content: {0}")]
    InvalidMessage(String),
    
    /// Invalid recipient information
    #[error("Invalid recipient: {0}")]
    InvalidRecipient(String),
    
    /// Invalid email address format
    #[error("Invalid email address: {0}")]
    InvalidEmail(String),
    
    /// Invalid phone number format
    #[error("Invalid phone number: {0}")]
    InvalidPhoneNumber(String),
    
    /// Message template not found
    #[error("Message template not found: {0}")]
    TemplateNotFound(String),
    
    /// Template loading error
    #[error("Template load error: {0}")]
    TemplateLoadError(String),
    
    /// Template rendering error
    #[error("Template render error: {0}")]
    TemplateRenderError(String),
    
    /// Operation not supported
    #[error("Operation not supported: {0}")]
    OperationNotSupported(String),
    
    /// Communication mode not supported
    #[error("Communication mode not supported: {0}")]
    UnsupportedMode(String),
    
    /// Message delivery failed
    #[error("Message delivery failed: {0}")]
    DeliveryFailed(String),
    
    /// Rate limit exceeded
    #[error("Rate limit exceeded for recipient: {0}")]
    RateLimitExceeded(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    /// Event processing error
    #[error("Event processing error: {0}")]
    EventProcessingError(String),
    
    /// Internal service error
    #[error("Internal service error: {0}")]
    InternalError(String),
    
    /// Communication service unavailable
    #[error("Communication service unavailable: {0}")]
    ServiceUnavailable(String),
    
    /// Infrastructure error (external service failure)
    #[error("Infrastructure error: {0}")]
    InfrastructureError(String),
}

impl DomainError {
    /// Create an invalid message error
    pub fn invalid_message(msg: impl Into<String>) -> Self {
        Self::InvalidMessage(msg.into())
    }
    
    /// Create an invalid recipient error
    pub fn invalid_recipient(msg: impl Into<String>) -> Self {
        Self::InvalidRecipient(msg.into())
    }
    
    /// Create an invalid email error
    pub fn invalid_email(email: impl Into<String>) -> Self {
        Self::InvalidEmail(email.into())
    }
    
    /// Create an invalid phone number error
    pub fn invalid_phone_number(phone: impl Into<String>) -> Self {
        Self::InvalidPhoneNumber(phone.into())
    }
    
    /// Create a template not found error
    pub fn template_not_found(template: impl Into<String>) -> Self {
        Self::TemplateNotFound(template.into())
    }
    
    /// Create a template load error
    pub fn template_load_error(msg: impl Into<String>) -> Self {
        Self::TemplateLoadError(msg.into())
    }
    
    /// Create a template render error
    pub fn template_render_error(msg: impl Into<String>) -> Self {
        Self::TemplateRenderError(msg.into())
    }
    
    /// Create an operation not supported error
    pub fn operation_not_supported(msg: impl Into<String>) -> Self {
        Self::OperationNotSupported(msg.into())
    }
    
    /// Create an unsupported mode error
    pub fn unsupported_mode(mode: impl Into<String>) -> Self {
        Self::UnsupportedMode(mode.into())
    }
    
    /// Create a delivery failed error
    pub fn delivery_failed(msg: impl Into<String>) -> Self {
        Self::DeliveryFailed(msg.into())
    }
    
    /// Create a rate limit exceeded error
    pub fn rate_limit_exceeded(recipient: impl Into<String>) -> Self {
        Self::RateLimitExceeded(recipient.into())
    }
    
    /// Create a configuration error
    pub fn configuration_error(msg: impl Into<String>) -> Self {
        Self::ConfigurationError(msg.into())
    }
    
    /// Create an event processing error
    pub fn event_processing_error(msg: impl Into<String>) -> Self {
        Self::EventProcessingError(msg.into())
    }
    
    /// Create an internal error
    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self::InternalError(msg.into())
    }
    
    /// Create a service unavailable error
    pub fn service_unavailable(service: impl Into<String>) -> Self {
        Self::ServiceUnavailable(service.into())
    }
    
    /// Create an infrastructure error
    pub fn infrastructure_error(msg: impl Into<String>) -> Self {
        Self::InfrastructureError(msg.into())
    }
    
    /// Check if this is a recoverable error (should retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            DomainError::ServiceUnavailable(_) | DomainError::DeliveryFailed(_) | DomainError::InfrastructureError(_)
        )
    }
    
    /// Check if this is a configuration-related error
    pub fn is_configuration_error(&self) -> bool {
        matches!(self, DomainError::ConfigurationError(_))
    }
    
    /// Check if this is a validation error
    pub fn is_validation_error(&self) -> bool {
        matches!(
            self,
            DomainError::InvalidMessage(_)
                | DomainError::InvalidRecipient(_)
                | DomainError::InvalidEmail(_)
                | DomainError::InvalidPhoneNumber(_)
        )
    }
} 