//! IAMRusty event adapter implementation using rustycog-events generic adapter system
//! 
//! This module provides IAMRusty-specific implementations of the EventAdapter and ErrorMapper
//! traits from rustycog-events, allowing seamless integration while maintaining architectural
//! separation.

use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use async_trait::async_trait;
use domain::error::DomainError;
use domain::port::event_publisher::EventPublisher;
use domain::entity::events::DomainEvent as IAMDomainEvent;
use rustycog_events::{
    adapter::{
        EventAdapter, ErrorMapper, GenericEventPublisherAdapter, 
        create_adapted_event_publisher
    },
    DomainEvent as RustycogDomainEvent,
};
use rustycog_core::error::ServiceError;
use rustycog_config::{KafkaConfig, QueueConfig};

/// IAMRusty-specific error mapper implementation
pub struct IAMErrorMapper;

impl ErrorMapper<DomainError> for IAMErrorMapper {
    fn to_service_error(&self, error: DomainError) -> ServiceError {
        match error {
            DomainError::UserNotFound => ServiceError::not_found("User not found"),
            DomainError::ProviderNotSupported(provider) => {
                ServiceError::validation(format!("Provider not supported: {}", provider))
            }
            DomainError::BusinessRuleViolation(message) => {
                ServiceError::business(message)
            }
            DomainError::InvalidToken => ServiceError::authentication("Invalid token"),
            DomainError::TokenExpired => ServiceError::authentication("Token expired"),
            DomainError::AuthorizationError(message) => ServiceError::authorization(message),
            DomainError::OAuth2Error(message) => ServiceError::infrastructure(format!("OAuth2 error: {}", message)),
            DomainError::UserProfileError(message) => {
                ServiceError::infrastructure(format!("User profile error: {}", message))
            }
            DomainError::NoTokenForProvider(provider, user) => {
                ServiceError::not_found(format!("No token found for provider {} and user {}", provider, user))
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
            DomainError::UsernameTaken => ServiceError::business("Username already taken".to_string()),
            DomainError::InvalidUsername => ServiceError::validation("Invalid username format".to_string()),
            DomainError::RegistrationAlreadyComplete => ServiceError::business("Registration already completed".to_string()),
            DomainError::TokenServiceError(message) => ServiceError::infrastructure(format!("Token service error: {}", message)),
            DomainError::EventError(message) => ServiceError::infrastructure(format!("Event error: {}", message)),
            DomainError::TokenNotFound => ServiceError::authentication("Token not found"),
        }
    }

    fn from_service_error(&self, error: ServiceError) -> DomainError {
        match error {
            ServiceError::Authentication { message, .. } => DomainError::AuthorizationError(message),
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

/// IAMRusty-specific event adapter implementation
pub struct IAMEventAdapter;

impl EventAdapter<IAMDomainEvent> for IAMEventAdapter {
    fn adapt_event(&self, event: IAMDomainEvent) -> Box<dyn RustycogDomainEvent> {
        Box::new(IAMDomainEventAdapter::new(event))
    }
}

/// Adapter that wraps IAMRusty domain events to implement rustycog DomainEvent trait
struct IAMDomainEventAdapter {
    inner: IAMDomainEvent,
}

impl IAMDomainEventAdapter {
    fn new(event: IAMDomainEvent) -> Self {
        Self { inner: event }
    }
}

impl std::fmt::Debug for IAMDomainEventAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IAMDomainEventAdapter({:?})", self.inner)
    }
}

impl RustycogDomainEvent for IAMDomainEventAdapter {
    fn event_type(&self) -> & str {
        self.inner.event_type()
    }
    
    fn event_id(&self) -> uuid::Uuid {
        self.inner.event_id()
    }
    
    fn aggregate_id(&self) -> uuid::Uuid {
        self.inner.user_id() // In IAMRusty, the user_id serves as the aggregate_id
    }
    
    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc> {
        match &self.inner {
            IAMDomainEvent::UserSignedUp(event) => event.base.occurred_at,
            IAMDomainEvent::UserEmailVerified(event) => event.base.occurred_at,
            IAMDomainEvent::UserLoggedIn(event) => event.base.occurred_at,
        }
    }

    fn version(&self) -> u32 {
        match &self.inner {
            IAMDomainEvent::UserSignedUp(event) => event.base.version,
            IAMDomainEvent::UserEmailVerified(event) => event.base.version,
            IAMDomainEvent::UserLoggedIn(event) => event.base.version,
        }
    }
    
    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(&self.inner)
            .map_err(|e| ServiceError::infrastructure(
                format!("Failed to serialize domain event: {}", e)
            ))
    }
    
    fn metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        
        // Add common metadata
        metadata.insert("source".to_string(), "iam-rusty".to_string());
        metadata.insert("version".to_string(), "1".to_string());
        
        // Add event-specific metadata
        match &self.inner {
            IAMDomainEvent::UserSignedUp(event) => {
                metadata.insert("email".to_string(), event.email.clone());
                metadata.insert("username".to_string(), event.username.clone());
                metadata.insert("email_verified".to_string(), event.email_verified.to_string());
            }
            IAMDomainEvent::UserEmailVerified(event) => {
                metadata.insert("email".to_string(), event.email.clone());
            }
            IAMDomainEvent::UserLoggedIn(event) => {
                metadata.insert("email".to_string(), event.email.clone());
                metadata.insert("login_method".to_string(), event.login_method.clone());
            }
        }
        
        metadata
    }
}

/// Type alias for IAMRusty's adapted event publisher
pub type IAMEventPublisherAdapter = GenericEventPublisherAdapter<IAMDomainEvent, DomainError>;

/// Multi-queue event publisher that can publish to multiple queues
pub struct MultiQueueEventPublisher {
    publishers: Vec<IAMEventPublisherAdapter>,
    queue_names: HashSet<String>,
}

impl MultiQueueEventPublisher {
    /// Create a new multi-queue event publisher
    pub fn new(publishers: Vec<IAMEventPublisherAdapter>, queue_names: HashSet<String>) -> Self {
        Self { publishers, queue_names }
    }

    /// Check if this publisher handles the given queue name
    pub fn handles_queue(&self, queue_name: &str) -> bool {
        self.queue_names.is_empty() || self.queue_names.contains(queue_name)
    }

    /// Get the queue names this publisher handles
    pub fn queue_names(&self) -> &HashSet<String> {
        &self.queue_names
    }
}

#[async_trait]
impl EventPublisher for MultiQueueEventPublisher {
    async fn publish(&self, event: IAMDomainEvent) -> Result<(), DomainError> {
        // Publish to all configured publishers
        for publisher in &self.publishers {
            publisher.publish(event.clone()).await?;
        }
        Ok(())
    }

    async fn publish_batch(&self, events: Vec<IAMDomainEvent>) -> Result<(), DomainError> {
        // Publish to all configured publishers
        for publisher in &self.publishers {
            publisher.publish_batch(events.clone()).await?;
        }
        Ok(())
    }

    async fn health_check(&self) -> Result<(), DomainError> {
        // Check health of all publishers
        for publisher in &self.publishers {
            publisher.health_check().await?;
        }
        Ok(())
    }
}

/// Wrapper that implements the domain EventPublisher trait (legacy single publisher support)
pub struct EventPublisherWrapper {
    inner: IAMEventPublisherAdapter,
}

impl EventPublisherWrapper {
    /// Create a new event publisher wrapper
    pub fn new(inner: IAMEventPublisherAdapter) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl EventPublisher for EventPublisherWrapper {
    async fn publish(&self, event: IAMDomainEvent) -> Result<(), DomainError> {
        self.inner.publish(event).await
    }

    async fn publish_batch(&self, events: Vec<IAMDomainEvent>) -> Result<(), DomainError> {
        self.inner.publish_batch(events).await
    }

    async fn health_check(&self) -> Result<(), DomainError> {
        self.inner.health_check().await
    }
}

/// Factory function to create an event publisher adapted for IAMRusty domain layer (legacy Kafka)
pub fn create_event_publisher(config: &KafkaConfig) -> Result<Arc<EventPublisherWrapper>, DomainError> {
    let error_mapper = Arc::new(IAMErrorMapper);
    let event_adapter = Arc::new(IAMEventAdapter);
    
    let adapted_publisher = create_adapted_event_publisher(config, error_mapper, event_adapter)
        .map_err(|service_error| {
            // Convert ServiceError to DomainError for this context
            IAMErrorMapper.from_service_error(service_error)
        })?;
    
    Ok(Arc::new(EventPublisherWrapper::new(adapted_publisher)))
}

/// Factory function to create an event publisher with queue config for IAMRusty domain layer
pub fn create_event_publisher_with_queue_config(config: &QueueConfig) -> Result<Arc<EventPublisherWrapper>, DomainError> {
    let error_mapper = Arc::new(IAMErrorMapper);
    let event_adapter = Arc::new(IAMEventAdapter);
    
    let adapted_publisher = rustycog_events::adapter::create_adapted_event_publisher_with_queue_config(
        config, 
        error_mapper, 
        event_adapter
    ).map_err(|service_error| {
        // Convert ServiceError to DomainError for this context
        IAMErrorMapper.from_service_error(service_error)
    })?;
    
    Ok(Arc::new(EventPublisherWrapper::new(adapted_publisher)))
}

/// Factory function to create an event publisher with queue config for IAMRusty domain layer (async)
pub async fn create_event_publisher_with_queue_config_async(config: &QueueConfig) -> Result<Arc<EventPublisherWrapper>, DomainError> {
    let error_mapper = Arc::new(IAMErrorMapper);
    let event_adapter = Arc::new(IAMEventAdapter);
    
    let adapted_publisher = rustycog_events::adapter::create_adapted_event_publisher_with_queue_config_async(
        config, 
        error_mapper, 
        event_adapter
    ).await.map_err(|service_error| {
        // Convert ServiceError to DomainError for this context
        IAMErrorMapper.from_service_error(service_error)
    })?;
    
    Ok(Arc::new(EventPublisherWrapper::new(adapted_publisher)))
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
) -> Result<Arc<MultiQueueEventPublisher>, DomainError> {
    let error_mapper = Arc::new(IAMErrorMapper);
    let event_adapter = Arc::new(IAMEventAdapter);
    
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
            },
            QueueConfig::Kafka(kafka_config) => {
                let mut all_queues = HashSet::new();
                all_queues.insert(kafka_config.user_events_topic.clone());
                all_queues
            }
        }
    });
    
    // For now, create a single publisher (we can extend this later to create multiple publishers for different queues)
    let adapted_publisher = rustycog_events::adapter::create_adapted_event_publisher_with_queue_config_async(
        config, 
        error_mapper, 
        event_adapter
    ).await.map_err(|service_error| {
        IAMErrorMapper.from_service_error(service_error)
    })?;
    
    Ok(Arc::new(MultiQueueEventPublisher::new(
        vec![adapted_publisher],
        queue_names,
    )))
}

/// Factory function to create multiple event publishers for specific queues
/// This creates one publisher per queue, allowing for fine-grained control
/// 
/// # Examples
/// 
/// ```rust
/// use std::collections::HashSet;
/// 
/// let mut target_queues = HashSet::new();
/// target_queues.insert("user-events".to_string());
/// target_queues.insert("email-events".to_string());
/// 
/// let publishers = create_queue_specific_event_publishers_async(&config, &target_queues).await?;
/// for (queue_name, publisher) in publishers {
///     println!("Created publisher for queue: {}", queue_name);
/// }
/// ```
pub async fn create_queue_specific_event_publishers_async(
    base_config: &QueueConfig,
    target_queues: &HashSet<String>,
) -> Result<Vec<(String, Arc<EventPublisherWrapper>)>, DomainError> {
    let mut publishers = Vec::new();
    
    for queue_name in target_queues {
        // Create a modified config for this specific queue
        let queue_config = match base_config {
            QueueConfig::Sqs(sqs_config) => {
                let mut modified_sqs_config = sqs_config.clone();
                modified_sqs_config.default_queue = queue_name.clone();
                QueueConfig::Sqs(modified_sqs_config)
            }
            QueueConfig::Kafka(kafka_config) => {
                let mut modified_kafka_config = kafka_config.clone();
                modified_kafka_config.user_events_topic = queue_name.clone();
                QueueConfig::Kafka(modified_kafka_config)
            }
            QueueConfig::Disabled => QueueConfig::Disabled,
        };
        
        let publisher = create_event_publisher_with_queue_config_async(&queue_config).await?;
        publishers.push((queue_name.clone(), publisher));
    }
    
    Ok(publishers)
}

/// Create a custom error mapper registry with IAM-specific mappings
pub fn create_iam_error_mapper_registry() -> rustycog_events::adapter::ErrorMapperRegistry<DomainError> {
    let mut registry = rustycog_events::adapter::ErrorMapperRegistry::new();
    registry.set_default_mapper(Arc::new(IAMErrorMapper));
    registry
}

/// Create a custom event adapter registry with IAM-specific mappings
pub fn create_iam_event_adapter_registry() -> rustycog_events::adapter::EventAdapterRegistry<IAMDomainEvent> {
    let mut registry = rustycog_events::adapter::EventAdapterRegistry::new();
    registry.set_default_adapter(Arc::new(IAMEventAdapter));
    registry
}

// Re-export the test consumer functionality when in test mode
#[cfg(any(test, feature = "test-utils"))]
pub use rustycog_events::test_consumer; 