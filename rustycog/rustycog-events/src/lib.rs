//! # RustyCog Events
//! 
//! Event publishing and subscription utilities.

pub mod kafka;
pub mod sqs;
pub mod no_op;
pub mod event;
pub mod adapter;

use rustycog_config::{KafkaConfig, SqsConfig, QueueConfig};
use rustycog_core::error::ServiceError;
use std::sync::Arc;
use async_trait::async_trait;

pub use kafka::KafkaEventPublisher;
pub use sqs::SqsEventPublisher;
pub use no_op::*;
pub use event::*;
pub use adapter::*;

/// Concrete event publisher that can be Kafka, SQS, or NoOp
pub enum ConcreteEventPublisher {
    Kafka(KafkaEventPublisher),
    Sqs(SqsEventPublisher),
    NoOp(NoOpEventPublisher),
}

#[async_trait]
impl EventPublisher for ConcreteEventPublisher {
    async fn publish(&self, event: Box<dyn DomainEvent>) -> Result<(), ServiceError> {
        match self {
            ConcreteEventPublisher::Kafka(kafka) => kafka.publish(event).await,
            ConcreteEventPublisher::Sqs(sqs) => sqs.publish(event).await,
            ConcreteEventPublisher::NoOp(no_op) => no_op.publish(event).await,
        }
    }

    async fn publish_batch(&self, events: Vec<Box<dyn DomainEvent>>) -> Result<(), ServiceError> {
        match self {
            ConcreteEventPublisher::Kafka(kafka) => kafka.publish_batch(events).await,
            ConcreteEventPublisher::Sqs(sqs) => sqs.publish_batch(events).await,
            ConcreteEventPublisher::NoOp(no_op) => no_op.publish_batch(events).await,
        }
    }

    async fn health_check(&self) -> Result<(), ServiceError> {
        match self {
            ConcreteEventPublisher::Kafka(kafka) => kafka.health_check().await,
            ConcreteEventPublisher::Sqs(sqs) => sqs.health_check().await,
            ConcreteEventPublisher::NoOp(no_op) => no_op.health_check().await,
        }
    }
}

/// Check if a Kafka test container is currently running
#[cfg(any(test, feature = "test-utils"))]
fn is_test_kafka_container_running() -> bool {
    // The kafka_testcontainer.rs sets these environment variables when a container is started
    // We check for these specific test environment variables to detect if a test container is active
    std::env::var("RUSTYCOG_KAFKA__HOST").is_ok() && 
    std::env::var("RUSTYCOG_KAFKA__PORT").is_ok() &&
    std::env::var("RUSTYCOG_KAFKA__ENABLED").map(|v| v == "true").unwrap_or(false)
}

/// Check if we're running in test mode
fn is_test_mode() -> bool {
    cfg!(test) || cfg!(feature = "test-utils")
}

/// Factory function to create an event publisher based on queue configuration
pub fn create_event_publisher_from_queue_config(config: &QueueConfig) -> Result<Arc<ConcreteEventPublisher>, ServiceError> {
    match config {
        QueueConfig::Kafka(kafka_config) => create_kafka_event_publisher(kafka_config),
        QueueConfig::Sqs(_sqs_config) => {
            // SQS creation is async, so we need to use a runtime context or make this function async
            // For now, we'll need to handle this at the application level
            Err(ServiceError::internal("SQS publisher creation must be done with create_sqs_event_publisher directly (async function)"))
        },
        QueueConfig::Disabled => {
            tracing::info!("Queue disabled, using no-op event publisher");
            Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())))
        }
    }
}

/// Factory function to create a Kafka event publisher based on configuration (legacy support)
pub fn create_event_publisher(config: &KafkaConfig) -> Result<Arc<ConcreteEventPublisher>, ServiceError> {
    create_kafka_event_publisher(config)
}

/// Factory function to create a Kafka event publisher
pub fn create_kafka_event_publisher(config: &KafkaConfig) -> Result<Arc<ConcreteEventPublisher>, ServiceError> {
    // In test mode, only use Kafka if explicitly enabled AND a test container is running
    if is_test_mode() {
        #[cfg(any(test, feature = "test-utils"))]
        {
            if config.enabled && is_test_kafka_container_running() {
                tracing::info!("Test mode: Test Kafka container detected, using Kafka event publisher");
                match KafkaEventPublisher::new(config.clone()) {
                    Ok(publisher) => {
                        return Ok(Arc::new(ConcreteEventPublisher::Kafka(publisher)));
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create Kafka event publisher in test mode, falling back to no-op: {}", e);
                        return Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())));
                    }
                }
            } else {
                tracing::info!("Test mode: No Kafka test container detected or Kafka disabled, using no-op event publisher");
                return Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())));
            }
        }
        
        #[cfg(not(any(test, feature = "test-utils")))]
        {
            // This branch should never be reached due to is_test_mode() check above,
            // but included for completeness
            tracing::info!("Test mode detected but test-utils feature not available, using no-op event publisher");
            return Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())));
        }
    }
    
    // Production mode: use the original logic
    if config.enabled {
        match KafkaEventPublisher::new(config.clone()) {
            Ok(publisher) => {
                tracing::info!("Created Kafka event publisher");
                Ok(Arc::new(ConcreteEventPublisher::Kafka(publisher)))
            }
            Err(e) => {
                tracing::warn!("Failed to create Kafka event publisher, falling back to no-op: {}", e);
                // Fall back to no-op publisher if Kafka creation fails
                Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())))
            }
        }
    } else {
        tracing::info!("Kafka disabled, using no-op event publisher");
        Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())))
    }
}

/// Factory function to create an SQS event publisher
pub async fn create_sqs_event_publisher(config: &SqsConfig) -> Result<Arc<ConcreteEventPublisher>, ServiceError> {
    // In test mode, only use SQS if explicitly enabled
    if is_test_mode() {
        #[cfg(any(test, feature = "test-utils"))]
        {
            if config.enabled {
                tracing::info!("Test mode: SQS enabled, using SQS event publisher");
                match SqsEventPublisher::new(config.clone()).await {
                    Ok(publisher) => {
                        return Ok(Arc::new(ConcreteEventPublisher::Sqs(publisher)));
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create SQS event publisher in test mode, falling back to no-op: {}", e);
                        return Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())));
                    }
                }
            } else {
                tracing::info!("Test mode: SQS disabled, using no-op event publisher");
                return Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())));
            }
        }
        
        #[cfg(not(any(test, feature = "test-utils")))]
        {
            tracing::info!("Test mode detected but test-utils feature not available, using no-op event publisher");
            return Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())));
        }
    }
    
    // Production mode: use the original logic
    if config.enabled {
        match SqsEventPublisher::new(config.clone()).await {
            Ok(publisher) => {
                tracing::info!("Created SQS event publisher");
                Ok(Arc::new(ConcreteEventPublisher::Sqs(publisher)))
            }
            Err(e) => {
                tracing::warn!("Failed to create SQS event publisher, falling back to no-op: {}", e);
                // Fall back to no-op publisher if SQS creation fails
                Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())))
            }
        }
    } else {
        tracing::info!("SQS disabled, using no-op event publisher");
        Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())))
    }
}

