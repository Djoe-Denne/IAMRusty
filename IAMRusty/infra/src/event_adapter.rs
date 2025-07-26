//! IAMRusty event adapter implementation using rustycog-events generic adapter system
//!
//! This module provides IAMRusty-specific implementations of the EventAdapter and ErrorMapper
//! traits from rustycog-events, allowing seamless integration while maintaining architectural
//! separation.

use iam_domain::error::DomainError;
use rustycog_config::QueueConfig;
use rustycog_core::error::ServiceError;
use rustycog_events::{
    adapter::{ErrorMapper, GenericEventPublisherAdapter, MultiQueueEventPublisher},
    ConcreteEventPublisher, 
    create_event_publisher_from_queue_config,
};
use std::collections::HashSet;
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
            DomainError::NoTokenForProvider => ServiceError::not_found(
                "No token found for provider and user".to_string()
            ),
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
    create_event_publisher_from_queue_config(config).await
    .map_err(|service_error| IAMErrorMapper.from_service_error(service_error))
}

/// Factory function to create a multi-queue event publisher with specific queue names
/// If queue_names is empty, it will handle all queues configured in the QueueConfig
///
/// # Examples
///
/// ```rust
/// use std::collections::HashSet;
/// use rustycog_config::QueueConfig;
///
/// // Create a publisher that handles all queues
/// let publisher = create_multi_queue_event_publisher_async(&config, None).await?;
///
/// // Create a publisher that only handles specific queues
/// let mut specific_queues = HashSet::new();
/// specific_queues.insert("user-events".to_string());
/// specific_queues.insert("email-events".to_string());
/// let publisher = create_multi_queue_event_publisher_async(&config, Some(specific_queues)).await?;
/// ```
pub async fn create_multi_queue_event_publisher_async(
    config: &QueueConfig,
    queue_names: Option<HashSet<String>>,
) -> Result<Arc<MultiQueueEventPublisher<DomainError>>, DomainError> {
    let error_mapper = Arc::new(IAMErrorMapper);

    let queue_names = queue_names.unwrap_or_else(|| {
        // If no specific queue names provided, use all configured queues
        match config {
            QueueConfig::Disabled => HashSet::new(),
            QueueConfig::Sqs(sqs_config) => {
                let mut all_queues = HashSet::new();
                // Add all queue names from the configuration
                for queue_name in sqs_config.queues.values() {
                    all_queues.insert(queue_name.clone());
                }
                // Also add the default queue
                all_queues.insert(sqs_config.default_queue.clone());
                all_queues
            }
            QueueConfig::Kafka(kafka_config) => {
                let mut all_queues = HashSet::new();
                all_queues.insert(kafka_config.user_events_topic.clone());
                all_queues
            }
        }
    });

    // For now, create a single publisher (we can extend this later to create multiple publishers for different queues)
    let adapted_publisher = create_event_publisher_with_queue_config(config).await?;
    let publisher = GenericEventPublisherAdapter::<DomainError>::new(adapted_publisher, error_mapper);

    Ok(Arc::new(MultiQueueEventPublisher::new(
        vec![publisher],
        queue_names,
    )))
}
