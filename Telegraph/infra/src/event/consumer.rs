//! Event consumer for processing IAM domain events

use async_trait::async_trait;
use iam_events::{IamDomainEvent, DomainEvent};
use rustycog_events::{EventConsumer as RustycogEventConsumer, EventHandler, ConcreteEventConsumer, create_event_consumer_from_queue_config};
use rustycog_core::error::ServiceError;
use std::sync::Arc;
use serde_json;
use tracing::{info, error, debug};
use crate::event::processors::{CommunicationEventProcessor, CompositeEventProcessor};
use telegraph_configuration::TelegraphConfig;

/// Telegraph event consumer using rustycog-events
pub struct EventConsumer {
    inner_consumer: Arc<ConcreteEventConsumer>,
    config: TelegraphConfig,
}

impl EventConsumer {
    /// Create a new event consumer from configuration
    pub async fn new(
        config: TelegraphConfig,
    ) -> Result<Self, telegraph_domain::DomainError> {
        let inner_consumer = create_event_consumer_from_queue_config(&config.queue)
            .await
            .map_err(|e| telegraph_domain::DomainError::InfrastructureError(format!("Failed to create event consumer: {}", e)))?;
        
        Ok(Self {
            inner_consumer,
            config,
        })
    }
    
    /// Start consuming events from queues
    pub async fn start(
        &self,
        event_processor: Arc<dyn CommunicationEventProcessor>,
    ) -> Result<(), telegraph_domain::DomainError> {
        info!("Starting Telegraph event consumer");
        
        // Log the configured queues and events for debugging
        for (queue_name, queue_config) in &self.config.queues {
            info!(
                queue_name = %queue_name,
                events = ?queue_config.events,
                "📋 Queue configuration loaded"
            );
        }
        
        // Create a handler that processes IAM events directly
        let handler = TelegraphEventHandler::new(self.config.clone(), event_processor);
        
        self.inner_consumer
            .start(handler)
            .await
            .map_err(|e| telegraph_domain::DomainError::InfrastructureError(format!("Event consumer error: {}", e)))?;
        
        Ok(())
    }
    
    /// Stop the event consumer
    pub async fn stop(&self) -> Result<(), telegraph_domain::DomainError> {
        info!("Stopping Telegraph event consumer");
        
        self.inner_consumer
            .stop()
            .await
            .map_err(|e| telegraph_domain::DomainError::InfrastructureError(format!("Failed to stop event consumer: {}", e)))?;
        
        Ok(())
    }
    
    /// Health check for the event consumer
    pub async fn health_check(&self) -> Result<(), telegraph_domain::DomainError> {
        self.inner_consumer
            .health_check()
            .await
            .map_err(|e| telegraph_domain::DomainError::InfrastructureError(format!("Event consumer health check failed: {}", e)))?;
        
        Ok(())
    }
}

/// Telegraph event handler that processes IAM events directly
pub struct TelegraphEventHandler {
    config: TelegraphConfig,
    event_processor: Arc<dyn CommunicationEventProcessor>,
}

impl TelegraphEventHandler {
    /// Create a new Telegraph event handler
    pub fn new(
        config: TelegraphConfig,
        event_processor: Arc<dyn CommunicationEventProcessor>,
    ) -> Self {
        Self {
            config,
            event_processor,
        }
    }
}

#[async_trait]
impl EventHandler for TelegraphEventHandler {
    async fn handle_event(&self, event: Box<dyn rustycog_events::DomainEvent>) -> Result<(), ServiceError> {
        info!(
            event_id = %event.event_id(),
            event_type = event.event_type(),
            "🎯 Telegraph received event from queue!"
        );
        
        // Convert the generic domain event to an IAM domain event
        match self.convert_to_iam_event(event.as_ref()) {
            Ok(iam_event) => {
                info!(
                    event_id = %iam_event.event_id(),
                    event_type = iam_event.event_type(),
                    user_id = %iam_event.user_id(),
                    "Processing IAM domain event in Telegraph"
                );
                
                // Process the event through communication processors
                match self.event_processor.process_event(&iam_event).await {
                    Ok(()) => {
                        info!(
                            event_id = %iam_event.event_id(),
                            event_type = iam_event.event_type(),
                            "Event processed successfully by communication processors"
                        );
                        Ok(())
                    }
                    Err(e) => {
                        error!(
                            event_id = %iam_event.event_id(),
                            event_type = iam_event.event_type(),
                            error = %e,
                            "Failed to process event through communication processors"
                        );
                        Err(ServiceError::infrastructure(format!("Event processing failed: {}", e)))
                    }
                }
            }
            Err(e) => {
                error!(
                    event_id = %event.event_id(),
                    event_type = event.event_type(),
                    error = %e,
                    "Failed to convert event to IAM event"
                );
                Err(e)
            }
        }
    }
    
    fn supports_event_type(&self, event_type: &str) -> bool {
        // Check if any configured queue supports this event type
        let mut supports = false;
        let mut supporting_queues = Vec::new();
        
        for (queue_name, queue_config) in &self.config.queues {
            if queue_config.events.contains(&event_type.to_string()) {
                supports = true;
                supporting_queues.push(queue_name.clone());
            }
        }
        
        if supports {
            info!(
                event_type = event_type,
                supporting_queues = ?supporting_queues,
                "✅ Event type supported by configuration"
            );
        } else {
            // Log discarded event - base data at INFO level
            info!(
                event_type = event_type,
                configured_queues = ?self.config.queues.keys().collect::<Vec<_>>(),
                "❌ Event type not supported by any configured queue - discarding"
            );
            
            // Log full configuration details at DEBUG level
            debug!(
                event_type = event_type,
                full_queue_config = ?self.config.queues,
                "🔍 Full queue configuration for discarded event"
            );
        }
        
        supports
    }
}

impl TelegraphEventHandler {
    /// Convert a generic domain event to an IAM domain event
    fn convert_to_iam_event(&self, event: &dyn rustycog_events::DomainEvent) -> Result<IamDomainEvent, ServiceError> {
        // With the producer fix, the event data is now properly structured as a JSON object
        let event_json = event.to_json()?;
        
        // Deserialize the properly formatted JSON directly as an IamDomainEvent
        serde_json::from_str(&event_json)
            .map_err(|e| ServiceError::infrastructure(format!("Failed to deserialize IAM event: {}", e)))
    }
} 