//! Event consumer for processing apparatus domain events

use apparatus_events::ApparatusDomainEvent;
use async_trait::async_trait;
use manifesto_domain::DomainError;
use rustycog_config::QueueConfig;
use rustycog_core::error::ServiceError;
use rustycog_events::{
    create_event_consumer_from_queue_config, ConcreteEventConsumer, DomainEvent,
    EventConsumer as RustycogEventConsumer, EventHandler,
};
use std::sync::Arc;
use tracing::{debug, error, info};

use super::processors::ComponentStatusProcessor;

/// Event consumer for apparatus domain events
pub struct ApparatusEventConsumer {
    inner_consumer: Arc<ConcreteEventConsumer>,
    component_processor: Arc<ComponentStatusProcessor>,
}

impl ApparatusEventConsumer {
    /// Create a new event consumer from queue configuration
    pub async fn new(
        queue_config: &QueueConfig,
        component_processor: Arc<ComponentStatusProcessor>,
    ) -> Result<Self, DomainError> {
        let inner_consumer = create_event_consumer_from_queue_config(queue_config)
            .await
            .map_err(|e| {
                DomainError::internal_error(&format!("Failed to create event consumer: {}", e))
            })?;

        Ok(Self {
            inner_consumer,
            component_processor,
        })
    }

    /// Start consuming events from queues
    pub async fn start(&self) -> Result<(), DomainError> {
        info!("Starting Apparatus event consumer");

        let handler = ApparatusEventHandler::new(self.component_processor.clone());

        self.inner_consumer
            .start(handler)
            .await
            .map_err(|e| DomainError::internal_error(&format!("Event consumer error: {}", e)))?;

        Ok(())
    }

    /// Stop the event consumer
    pub async fn stop(&self) -> Result<(), DomainError> {
        info!("Stopping Apparatus event consumer");

        self.inner_consumer.stop().await.map_err(|e| {
            DomainError::internal_error(&format!("Failed to stop event consumer: {}", e))
        })?;

        Ok(())
    }

    /// Health check for the event consumer
    pub async fn health_check(&self) -> Result<(), DomainError> {
        self.inner_consumer.health_check().await.map_err(|e| {
            DomainError::internal_error(&format!("Event consumer health check failed: {}", e))
        })?;

        Ok(())
    }
}

/// Event handler for apparatus domain events
pub struct ApparatusEventHandler {
    component_processor: Arc<ComponentStatusProcessor>,
}

impl ApparatusEventHandler {
    /// Create a new apparatus event handler
    pub fn new(component_processor: Arc<ComponentStatusProcessor>) -> Self {
        Self {
            component_processor,
        }
    }

    /// Process an apparatus domain event
    async fn process_apparatus_event(
        &self,
        event: ApparatusDomainEvent,
    ) -> Result<(), ServiceError> {
        debug!("Processing apparatus event: {}", event.event_type());

        match event {
            ApparatusDomainEvent::ComponentStatusChanged(status_event) => {
                self.component_processor.process(status_event).await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl EventHandler for ApparatusEventHandler {
    async fn handle_event(
        &self,
        event: Box<dyn rustycog_events::DomainEvent>,
    ) -> Result<(), ServiceError> {
        let event_id = event.event_id();
        let event_type = event.event_type().to_string();

        info!(
            event_id = %event_id,
            event_type = %event_type,
            "Apparatus received event from queue"
        );

        // Serialize to JSON and deserialize as ApparatusDomainEvent
        let event_json = event.to_json().map_err(|e| {
            error!("Failed to serialize event: {}", e);
            ServiceError::infrastructure(format!("Failed to serialize event: {}", e))
        })?;

        let apparatus_event: ApparatusDomainEvent =
            serde_json::from_str(&event_json).map_err(|e| {
                error!("Failed to parse apparatus event: {}", e);
                ServiceError::validation(format!("Failed to parse event: {}", e))
            })?;

        self.process_apparatus_event(apparatus_event).await?;

        info!(
            event_id = %event_id,
            "Event processed successfully"
        );

        Ok(())
    }

    fn supports_event_type(&self, event_type: &str) -> bool {
        // Support component status changed events
        matches!(
            event_type,
            "component_status_changed" | "ComponentStatusChanged"
        )
    }
}

/// Helper to create event consumer with all processors
pub fn create_apparatus_event_consumer(
    component_processor: Arc<ComponentStatusProcessor>,
) -> Arc<ApparatusEventHandler> {
    Arc::new(ApparatusEventHandler::new(component_processor))
}
