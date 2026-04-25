//! Hive event adapter implementation using rustycog-events generic adapter system
//!
//! This module provides Hive-specific implementations of the EventAdapter and ErrorMapper
//! traits from rustycog-events, allowing seamless integration while maintaining architectural
//! separation.

use rustycog_core::error::DomainError;
use rustycog_config::QueueConfig;
use rustycog_core::error::ServiceError;
use rustycog_events::{
    adapter::{ErrorMapper, GenericEventPublisherAdapter, MultiQueueEventPublisher},
    ConcreteEventPublisher, 
    create_event_publisher_from_queue_config,
};
use std::collections::HashSet;
use std::sync::Arc;

/// Hive-specific error mapper implementation
pub struct HiveErrorMapper;

impl ErrorMapper<DomainError> for HiveErrorMapper {
    fn to_service_error(&self, error: DomainError) -> ServiceError {
        match error {
            DomainError::EntityNotFound { entity_type, id } => {
                ServiceError::not_found(format!("{} with id '{}' not found", entity_type, id))
            }
            DomainError::InvalidInput { message } => {
                ServiceError::validation(message)
            }
            DomainError::BusinessRuleViolation { rule } => {
                ServiceError::business(format!("Business rule violation: {}", rule))
            }
            DomainError::Unauthorized { operation } => {
                ServiceError::authentication(format!("Unauthorized: {}", operation))
            }
            DomainError::ResourceAlreadyExists { resource_type, identifier } => {
                ServiceError::conflict(format!("{} with identifier '{}' already exists", resource_type, identifier))
            }
            DomainError::ExternalServiceError { service, message } => {
                ServiceError::infrastructure(format!("External service error from {}: {}", service, message))
            }
            DomainError::PermissionDenied { message } => {
                ServiceError::authorization(message)
            }
            DomainError::Internal { message } => {
                ServiceError::internal(message)
            }
        }
    }

    fn from_service_error(&self, error: ServiceError) -> DomainError {
        match error {
            ServiceError::Authentication { message, .. } => {
                DomainError::Unauthorized { operation: message }
            }
            ServiceError::Authorization { message, .. } => {
                DomainError::PermissionDenied { message }
            }
            ServiceError::NotFound { message, .. } => {
                DomainError::EntityNotFound { 
                    entity_type: "Resource".to_string(), 
                    id: message 
                }
            }
            ServiceError::Infrastructure { message, .. } => {
                DomainError::ExternalServiceError { 
                    service: "Infrastructure".to_string(), 
                    message 
                }
            }
            ServiceError::Validation { message, .. } => {
                DomainError::InvalidInput { message }
            }
            ServiceError::Business { message, .. } => {
                DomainError::BusinessRuleViolation { rule: message }
            }
            ServiceError::Timeout { message, .. } => {
                DomainError::ExternalServiceError { 
                    service: "Timeout".to_string(), 
                    message 
                }
            }
            ServiceError::ServiceUnavailable { message, .. } => {
                DomainError::ExternalServiceError { 
                    service: "Unavailable".to_string(), 
                    message 
                }
            }
            ServiceError::Internal { message, .. } => {
                DomainError::Internal { message }
            }
            ServiceError::Conflict { message, .. } => {
                DomainError::ExternalServiceError { 
                    service: "Conflict".to_string(), 
                    message 
                }
            }
            ServiceError::RateLimit { message, .. } => {
                DomainError::ExternalServiceError { 
                    service: "RateLimit".to_string(), 
                    message 
                }
            }
        }
    }
}

/// Factory function to create an event publisher with queue config for Hive domain layer
pub async fn create_event_publisher_with_queue_config(
    config: &QueueConfig,
) -> Result<Arc<ConcreteEventPublisher>, DomainError> {
    create_event_publisher_from_queue_config(config).await
    .map_err(|service_error| HiveErrorMapper.from_service_error(service_error))
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
/// specific_queues.insert("organization-events".to_string());
/// specific_queues.insert("invitation-events".to_string());
/// let publisher = create_multi_queue_event_publisher_async(&config, Some(specific_queues)).await?;
/// ```
pub async fn create_multi_queue_event_publisher_async(
    config: &QueueConfig,
    queue_names: Option<HashSet<String>>,
) -> Result<Arc<MultiQueueEventPublisher<DomainError>>, DomainError> {
    let error_mapper = Arc::new(HiveErrorMapper);

    let queue_names = queue_names.unwrap_or_else(|| {
        // If no specific queue names provided, use all configured queues
        match config {
            QueueConfig::Disabled => HashSet::new(),
            QueueConfig::Sqs(sqs_config) => sqs_config.all_queue_names(),
            QueueConfig::Kafka(kafka_config) => {
                let mut all_queues = HashSet::new();
                all_queues.insert(kafka_config.user_events_topic.clone());
                all_queues
            }
        }
    });

    // Create a single publisher (can be extended for multiple publishers for different queues)
    let adapted_publisher = create_event_publisher_with_queue_config(config).await?;
    let publisher = GenericEventPublisherAdapter::<DomainError>::new(adapted_publisher, error_mapper);

    Ok(Arc::new(MultiQueueEventPublisher::new(
        vec![publisher],
        queue_names,
    )))
} 