//! Generic adapter system for integrating different domain types with rustycog-events
//! 
//! This module provides a flexible adapter pattern that allows any project to integrate
//! their own domain events and error types with rustycog-events, while maintaining
//! type safety and allowing custom mappings.

use std::sync::Arc;
use std::collections::HashMap;
use rustycog_core::error::ServiceError;
use crate::{ConcreteEventPublisher, DomainEvent as RustycogDomainEvent, EventPublisher};
use rustycog_config::{KafkaConfig, QueueConfig};

/// Trait for bidirectional mapping between custom error types and ServiceError
pub trait ErrorMapper<E>: Send + Sync {
    /// Map a custom error type to ServiceError
    fn to_service_error(&self, error: E) -> ServiceError;
    
    /// Map a ServiceError back to custom error type
    fn from_service_error(&self, error: ServiceError) -> E;
}

/// Trait for adapting custom domain events to rustycog DomainEvent
pub trait EventAdapter<E>: Send + Sync {
    /// Adapt a custom domain event to rustycog DomainEvent
    fn adapt_event(&self, event: E) -> Box<dyn RustycogDomainEvent>;
}

/// Generic event publisher adapter that can work with any domain event and error type
pub struct GenericEventPublisherAdapter<TEvent, TError> {
    inner: Arc<ConcreteEventPublisher>,
    error_mapper: Arc<dyn ErrorMapper<TError>>,
    event_adapter: Arc<dyn EventAdapter<TEvent>>,
}

impl<TEvent, TError> GenericEventPublisherAdapter<TEvent, TError> {
    /// Create a new generic event publisher adapter
    pub fn new(
        inner: Arc<ConcreteEventPublisher>,
        error_mapper: Arc<dyn ErrorMapper<TError>>,
        event_adapter: Arc<dyn EventAdapter<TEvent>>,
    ) -> Self {
        Self {
            inner,
            error_mapper,
            event_adapter,
        }
    }

    /// Publish a single event
    pub async fn publish(&self, event: TEvent) -> Result<(), TError> {
        let rustycog_event = self.event_adapter.adapt_event(event);
        
        self.inner
            .publish(rustycog_event)
            .await
            .map_err(|service_error| self.error_mapper.from_service_error(service_error))
    }

    /// Publish multiple events in a batch
    pub async fn publish_batch(&self, events: Vec<TEvent>) -> Result<(), TError> {
        let rustycog_events: Vec<Box<dyn RustycogDomainEvent>> = events
            .into_iter()
            .map(|event| self.event_adapter.adapt_event(event))
            .collect();

        self.inner
            .publish_batch(rustycog_events)
            .await
            .map_err(|service_error| self.error_mapper.from_service_error(service_error))
    }

    /// Health check
    pub async fn health_check(&self) -> Result<(), TError> {
        self.inner
            .health_check()
            .await
            .map_err(|service_error| self.error_mapper.from_service_error(service_error))
    }
}

/// A registry for custom error mappers that allows overriding default mappings
pub struct ErrorMapperRegistry<TError> {
    mappers: HashMap<String, Arc<dyn ErrorMapper<TError>>>,
    default_mapper: Option<Arc<dyn ErrorMapper<TError>>>,
}

impl<TError> ErrorMapperRegistry<TError> {
    /// Create a new error mapper registry
    pub fn new() -> Self {
        Self {
            mappers: HashMap::new(),
            default_mapper: None,
        }
    }

    /// Register a custom error mapper (can override existing ones)
    pub fn register_mapper(&mut self, name: String, mapper: Arc<dyn ErrorMapper<TError>>) {
        if self.mappers.contains_key(&name) {
            tracing::warn!("Overriding existing error mapper: {}", name);
        }
        self.mappers.insert(name, mapper);
    }

    /// Set the default error mapper
    pub fn set_default_mapper(&mut self, mapper: Arc<dyn ErrorMapper<TError>>) {
        self.default_mapper = Some(mapper);
    }

    /// Get a mapper by name, or the default mapper
    pub fn get_mapper(&self, name: Option<&str>) -> Option<Arc<dyn ErrorMapper<TError>>> {
        if let Some(name) = name {
            self.mappers.get(name).cloned()
        } else {
            self.default_mapper.clone()
        }
    }

    /// List all registered mapper names
    pub fn list_mappers(&self) -> Vec<String> {
        self.mappers.keys().cloned().collect()
    }

    /// Remove a mapper by name
    pub fn remove_mapper(&mut self, name: &str) -> Option<Arc<dyn ErrorMapper<TError>>> {
        self.mappers.remove(name)
    }
}

impl<TError> Default for ErrorMapperRegistry<TError> {
    fn default() -> Self {
        Self::new()
    }
}

/// A registry for custom event adapters that allows overriding default mappings
pub struct EventAdapterRegistry<TEvent> {
    adapters: HashMap<String, Arc<dyn EventAdapter<TEvent>>>,
    default_adapter: Option<Arc<dyn EventAdapter<TEvent>>>,
}

impl<TEvent> EventAdapterRegistry<TEvent> {
    /// Create a new event adapter registry
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
            default_adapter: None,
        }
    }

    /// Register a custom event adapter (can override existing ones)
    pub fn register_adapter(&mut self, name: String, adapter: Arc<dyn EventAdapter<TEvent>>) {
        if self.adapters.contains_key(&name) {
            tracing::warn!("Overriding existing event adapter: {}", name);
        }
        self.adapters.insert(name, adapter);
    }

    /// Set the default event adapter
    pub fn set_default_adapter(&mut self, adapter: Arc<dyn EventAdapter<TEvent>>) {
        self.default_adapter = Some(adapter);
    }

    /// Get an adapter by name, or the default adapter
    pub fn get_adapter(&self, name: Option<&str>) -> Option<Arc<dyn EventAdapter<TEvent>>> {
        if let Some(name) = name {
            self.adapters.get(name).cloned()
        } else {
            self.default_adapter.clone()
        }
    }

    /// List all registered adapter names
    pub fn list_adapters(&self) -> Vec<String> {
        self.adapters.keys().cloned().collect()
    }

    /// Remove an adapter by name
    pub fn remove_adapter(&mut self, name: &str) -> Option<Arc<dyn EventAdapter<TEvent>>> {
        self.adapters.remove(name)
    }
}

impl<TEvent> Default for EventAdapterRegistry<TEvent> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating adapted event publishers
pub struct AdaptedEventPublisherBuilder<TEvent, TError> {
    config: Option<QueueConfig>,
    error_mapper: Option<Arc<dyn ErrorMapper<TError>>>,
    event_adapter: Option<Arc<dyn EventAdapter<TEvent>>>,
}

impl<TEvent, TError> AdaptedEventPublisherBuilder<TEvent, TError> {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: None,
            error_mapper: None,
            event_adapter: None,
        }
    }

    /// Set the queue configuration (Kafka, SQS, or Disabled)
    pub fn with_config(mut self, config: QueueConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the Kafka configuration (for backward compatibility)
    pub fn with_kafka_config(mut self, config: KafkaConfig) -> Self {
        self.config = Some(QueueConfig::Kafka(config));
        self
    }

    /// Set the error mapper
    pub fn with_error_mapper(mut self, mapper: Arc<dyn ErrorMapper<TError>>) -> Self {
        self.error_mapper = Some(mapper);
        self
    }

    /// Set the event adapter
    pub fn with_event_adapter(mut self, adapter: Arc<dyn EventAdapter<TEvent>>) -> Self {
        self.event_adapter = Some(adapter);
        self
    }

    /// Build the adapted event publisher
    pub fn build(self) -> Result<GenericEventPublisherAdapter<TEvent, TError>, ServiceError> {
        let config = self.config.ok_or_else(|| {
            ServiceError::validation("Queue config is required")
        })?;

        let error_mapper = self.error_mapper.ok_or_else(|| {
            ServiceError::validation("Error mapper is required")
        })?;

        let event_adapter = self.event_adapter.ok_or_else(|| {
            ServiceError::validation("Event adapter is required")
        })?;

        let publisher = match &config {
            QueueConfig::Kafka(kafka_config) => crate::create_event_publisher(kafka_config)?,
            QueueConfig::Sqs(_) => {
                return Err(ServiceError::internal("SQS publisher creation requires async context. Use create_adapted_event_publisher_async instead"));
            },
            QueueConfig::Disabled => {
                tracing::info!("Queue disabled in adapter, using no-op event publisher");
                Arc::new(crate::ConcreteEventPublisher::NoOp(crate::NoOpEventPublisher::new()))
            }
        };

        Ok(GenericEventPublisherAdapter::new(
            publisher,
            error_mapper,
            event_adapter,
        ))
    }

    /// Build the adapted event publisher (async version for SQS)
    pub async fn build_async(self) -> Result<GenericEventPublisherAdapter<TEvent, TError>, ServiceError> {
        let config = self.config.ok_or_else(|| {
            ServiceError::validation("Queue config is required")
        })?;

        let error_mapper = self.error_mapper.ok_or_else(|| {
            ServiceError::validation("Error mapper is required")
        })?;

        let event_adapter = self.event_adapter.ok_or_else(|| {
            ServiceError::validation("Event adapter is required")
        })?;

        let publisher = match &config {
            QueueConfig::Disabled => {
                tracing::info!("Queue disabled in adapter, using no-op event publisher");
                Arc::new(crate::ConcreteEventPublisher::NoOp(crate::NoOpEventPublisher::new()))
            },
            QueueConfig::Kafka(kafka_config) => crate::create_event_publisher(kafka_config)?,
            QueueConfig::Sqs(sqs_config) => crate::create_sqs_event_publisher(sqs_config).await?
        };

        Ok(GenericEventPublisherAdapter::new(
            publisher,
            error_mapper,
            event_adapter,
        ))
    }
}

impl<TEvent, TError> Default for AdaptedEventPublisherBuilder<TEvent, TError> {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create an adapted event publisher (legacy Kafka support)
pub fn create_adapted_event_publisher<TEvent, TError>(
    config: &KafkaConfig,
    error_mapper: Arc<dyn ErrorMapper<TError>>,
    event_adapter: Arc<dyn EventAdapter<TEvent>>,
) -> Result<GenericEventPublisherAdapter<TEvent, TError>, ServiceError> {
    AdaptedEventPublisherBuilder::new()
        .with_kafka_config(config.clone())
        .with_error_mapper(error_mapper)
        .with_event_adapter(event_adapter)
        .build()
}

/// Convenience function to create an adapted event publisher with queue config
pub fn create_adapted_event_publisher_with_queue_config<TEvent, TError>(
    config: &QueueConfig,
    error_mapper: Arc<dyn ErrorMapper<TError>>,
    event_adapter: Arc<dyn EventAdapter<TEvent>>,
) -> Result<GenericEventPublisherAdapter<TEvent, TError>, ServiceError> {
    AdaptedEventPublisherBuilder::new()
        .with_config(config.clone())
        .with_error_mapper(error_mapper)
        .with_event_adapter(event_adapter)
        .build()
}

/// Convenience function to create an adapted event publisher with queue config (async)
pub async fn create_adapted_event_publisher_with_queue_config_async<TEvent, TError>(
    config: &QueueConfig,
    error_mapper: Arc<dyn ErrorMapper<TError>>,
    event_adapter: Arc<dyn EventAdapter<TEvent>>,
) -> Result<GenericEventPublisherAdapter<TEvent, TError>, ServiceError> {
    AdaptedEventPublisherBuilder::new()
        .with_config(config.clone())
        .with_error_mapper(error_mapper)
        .with_event_adapter(event_adapter)
        .build_async()
        .await
} 