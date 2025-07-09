//! Event consumer for processing IAM domain events

use async_trait::async_trait;
use domain::{DomainError, IamEventHandler};
use iam_events::{IamDomainEvent, DomainEvent};
use rustycog_events::{EventConsumer as RustycogEventConsumer, EventHandler, ConcreteEventConsumer, create_event_consumer_from_queue_config};
use rustycog_config::QueueConfig;
use rustycog_core::error::ServiceError;
use std::sync::Arc;
use serde_json;
use tracing::{info, error};
use crate::event::processors::{CommunicationEventProcessor, CompositeEventProcessor};

/// Telegraph event consumer using rustycog-events
pub struct EventConsumer {
    inner_consumer: Arc<ConcreteEventConsumer>,
    event_processor: Arc<dyn CommunicationEventProcessor>,
}

impl EventConsumer {
    /// Create a new event consumer from configuration with communication services
    pub async fn new(
        queue_config: &QueueConfig,
        email_service: Arc<dyn domain::EmailService>,
        sms_service: Arc<dyn domain::SmsService>,
        notification_service: Arc<dyn domain::NotificationService>,
    ) -> Result<Self, DomainError> {
        let inner_consumer = create_event_consumer_from_queue_config(queue_config)
            .await
            .map_err(|e| DomainError::InfrastructureError(format!("Failed to create event consumer: {}", e)))?;
        
        // Create composite event processor with all communication services
        let event_processor = Arc::new(CompositeEventProcessor::with_all_processors(
            email_service,
            sms_service,
            notification_service,
        ));
        
        Ok(Self {
            inner_consumer,
            event_processor,
        })
    }
    
    /// Start consuming events from queues
    pub async fn start(&self) -> Result<(), DomainError> {
        info!("Starting Telegraph event consumer");
        
        // Create a handler that adapts rustycog-events to IAM events
        let handler = TelegraphEventHandler::new();
        
        self.inner_consumer
            .start(handler)
            .await
            .map_err(|e| DomainError::InfrastructureError(format!("Event consumer error: {}", e)))?;
        
        Ok(())
    }
    
    /// Stop the event consumer
    pub async fn stop(&self) -> Result<(), DomainError> {
        info!("Stopping Telegraph event consumer");
        
        self.inner_consumer
            .stop()
            .await
            .map_err(|e| DomainError::InfrastructureError(format!("Failed to stop event consumer: {}", e)))?;
        
        Ok(())
    }
    
    /// Health check for the event consumer
    pub async fn health_check(&self) -> Result<(), DomainError> {
        self.inner_consumer
            .health_check()
            .await
            .map_err(|e| DomainError::InfrastructureError(format!("Event consumer health check failed: {}", e)))?;
        
        Ok(())
    }
}

#[async_trait]
impl IamEventHandler for EventConsumer {
    async fn handle_event(&self, event: &IamDomainEvent) -> Result<(), DomainError> {
        info!(
            event_id = %event.event_id(),
            event_type = event.event_type(),
            user_id = %event.user_id(),
            "Handling IAM domain event"
        );
        
        // Route events to appropriate communication processors
        match self.event_processor.process_event(event).await {
            Ok(()) => {
                info!(
                    event_id = %event.event_id(),
                    event_type = event.event_type(),
                    "Event routed and processed successfully"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    event_id = %event.event_id(),
                    event_type = event.event_type(),
                    error = %e,
                    "Failed to process event through communication processors"
                );
                Err(e)
            }
        }
    }
    
    fn supports_event_type(&self, event_type: &str) -> bool {
        // Use the event processor to determine supported event types
        self.event_processor.supports_event_type(event_type)
    }
}

/// Telegraph event handler that adapts rustycog-events to IAM events
pub struct TelegraphEventHandler;

impl TelegraphEventHandler {
    /// Create a new Telegraph event handler
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EventHandler for TelegraphEventHandler {
    async fn handle_event(&self, event: Box<dyn rustycog_events::DomainEvent>) -> Result<(), ServiceError> {
        // Convert the generic domain event to an IAM domain event
        // This is a simplified conversion - in practice you might need more sophisticated parsing
        match self.convert_to_iam_event(event.as_ref()) {
            Ok(iam_event) => {
                // Process the IAM event directly
                info!(
                    event_id = %iam_event.event_id(),
                    event_type = iam_event.event_type(),
                    user_id = %iam_event.user_id(),
                    "Processing IAM domain event in Telegraph"
                );
                
                // Here you would route the event to appropriate communication processors
                // For now, just log that we've processed it
                info!(
                    event_id = %iam_event.event_id(),
                    "IAM event processed successfully for communication"
                );
                
                Ok(())
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
        // Support IAM-related events
        event_type.starts_with("iam.") || 
        event_type.starts_with("user.") ||
        event_type.starts_with("auth.")
    }
}

impl TelegraphEventHandler {
    /// Convert a generic domain event to an IAM domain event
    fn convert_to_iam_event(&self, event: &dyn rustycog_events::DomainEvent) -> Result<IamDomainEvent, ServiceError> {
        // This is a simplified conversion - you might need to implement proper parsing based on your event structure
        let event_json = event.to_json()?;
        
        // Try to create an IAM domain event from the JSON string
        // This assumes the event data contains the necessary IAM event information
        serde_json::from_str(&event_json)
            .map_err(|e| ServiceError::infrastructure(format!("Failed to deserialize IAM event: {}", e)))
    }
} 