//! IAMRusty event adapter implementation using rustycog-events generic adapter system
//!
//! This module provides IAMRusty-specific implementations of the EventAdapter and ErrorMapper
//! traits from rustycog-events, allowing seamless integration while maintaining architectural
//! separation.

use iam_domain::error::DomainError;
use rustycog_config::QueueConfig;
use rustycog_core::error::ServiceError;
use rustycog_events::{
    adapter::ErrorMapper, create_event_publisher_from_queue_config, ConcreteEventPublisher,
};
use std::sync::Arc;

/// IAMRusty-specific error mapper implementation
pub struct IAMErrorMapper;

impl ErrorMapper<DomainError> for IAMErrorMapper {
    fn to_service_error(&self, error: DomainError) -> ServiceError {
        match error {
            DomainError::UserNotFound => ServiceError::not_found("User not found"),
            DomainError::ProviderNotSupported(provider) => {
                ServiceError::validation(format!("Provider not supported: {}", provider))
            }
            DomainError::BusinessRuleViolation(message) => ServiceError::business(message),
            DomainError::InvalidToken => ServiceError::authentication("Invalid token"),
            DomainError::TokenExpired => ServiceError::authentication("Token expired"),
            DomainError::AuthorizationError(message) => ServiceError::authorization(message),
            DomainError::OAuth2Error(message) => {
                ServiceError::infrastructure(format!("OAuth2 error: {}", message))
            }
            DomainError::UserProfileError(message) => {
                ServiceError::infrastructure(format!("User profile error: {}", message))
            }
            DomainError::NoTokenForProvider => {
                ServiceError::not_found("No token found for provider and user".to_string())
            }
            DomainError::TokenGenerationFailed(message) => {
                ServiceError::internal(format!("Token generation failed: {}", message))
            }
            DomainError::TokenValidationFailed(message) => {
                ServiceError::validation(format!("Token validation failed: {}", message))
            }
            DomainError::RepositoryError(message) => {
                ServiceError::infrastructure(format!("Repository error: {}", message))
            }
            // Registration-specific errors
            DomainError::UsernameTaken => {
                ServiceError::business("Username already taken".to_string())
            }
            DomainError::InvalidUsername => {
                ServiceError::validation("Invalid username format".to_string())
            }
            DomainError::RegistrationAlreadyComplete => {
                ServiceError::business("Registration already completed".to_string())
            }
            DomainError::TokenServiceError(message) => {
                ServiceError::infrastructure(format!("Token service error: {}", message))
            }
            DomainError::EventError(message) => {
                ServiceError::infrastructure(format!("Event error: {}", message))
            }
            DomainError::TokenNotFound => ServiceError::authentication("Token not found"),
        }
    }

    fn from_service_error(&self, error: ServiceError) -> DomainError {
        match error {
            ServiceError::Authentication { message, .. } => {
                DomainError::AuthorizationError(message)
            }
            ServiceError::Authorization { message, .. } => DomainError::AuthorizationError(message),
            ServiceError::NotFound { .. } => DomainError::UserNotFound,
            ServiceError::Infrastructure { message, .. } => DomainError::RepositoryError(message),
            ServiceError::Validation { message, .. } => DomainError::TokenValidationFailed(message),
            ServiceError::Business { message, .. } => DomainError::BusinessRuleViolation(message),
            ServiceError::Timeout { message, .. } => {
                DomainError::RepositoryError(format!("Timeout: {}", message))
            }
            ServiceError::ServiceUnavailable { message, .. } => {
                DomainError::RepositoryError(format!("Service unavailable: {}", message))
            }
            ServiceError::Internal { message, .. } => {
                DomainError::RepositoryError(format!("Internal error: {}", message))
            }
            ServiceError::Conflict { message, .. } => {
                DomainError::RepositoryError(format!("Conflict: {}", message))
            }
            ServiceError::RateLimit { message, .. } => {
                DomainError::RepositoryError(format!("Rate limit: {}", message))
            }
        }
    }
}

/// Factory function to create an event publisher with queue config for IAMRusty domain layer
pub async fn create_event_publisher_with_queue_config(
    config: &QueueConfig,
) -> Result<Arc<ConcreteEventPublisher>, DomainError> {
    create_event_publisher_from_queue_config(config)
        .await
        .map_err(|service_error| IAMErrorMapper.from_service_error(service_error))
}
