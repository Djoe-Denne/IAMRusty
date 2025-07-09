//! Event processing domain service for Telegraph

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::error::DomainError;
use crate::port::{IamEventHandler, EventProcessor};
use iam_events::{IamDomainEvent, DomainEvent};

/// Event processing service that coordinates event handling
pub struct EventProcessingServiceImpl {
    handlers: HashMap<String, Vec<Arc<dyn IamEventHandler>>>,
}

impl EventProcessingServiceImpl {
    /// Create a new event processing service
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }
}

#[async_trait]
impl EventProcessor for EventProcessingServiceImpl {
    async fn process_event(&self, event: &IamDomainEvent) -> Result<(), DomainError> {
        let event_type = event.event_type();
        
        info!(
            event_id = %event.event_id(),
            event_type = event_type,
            user_id = %event.user_id(),
            "Processing IAM event"
        );
        
        let handlers = self.get_handlers_for_event(event_type);
        
        if handlers.is_empty() {
            warn!(
                event_type = event_type,
                "No handlers found for event type"
            );
            return Ok(()); // Not an error - just no handlers
        }
        
        let mut errors = Vec::new();
        let mut processed_count = 0;
        
        for handler in handlers {
            match handler.handle_event(event).await {
                Ok(()) => {
                    processed_count += 1;
                    info!(
                        event_type = event_type,
                        handler_priority = handler.priority(),
                        "Event processed successfully by handler"
                    );
                }
                Err(e) => {
                    error!(
                        event_type = event_type,
                        handler_priority = handler.priority(),
                        error = %e,
                        "Handler failed to process event"
                    );
                    errors.push(e);
                }
            }
        }
        
        if processed_count == 0 && !errors.is_empty() {
            // All handlers failed
            let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            return Err(DomainError::event_processing_error(
                format!("All handlers failed: {}", error_messages.join(", "))
            ));
        }
        
        info!(
            event_type = event_type,
            processed_count = processed_count,
            failed_count = errors.len(),
            "Event processing completed"
        );
        
        Ok(())
    }
    
    async fn register_handler(&self, _handler: Box<dyn IamEventHandler>) -> Result<(), DomainError> {
        // Note: This would require mutable access in a real implementation
        // For now, this is just a placeholder
        Err(DomainError::internal_error("Handler registration not implemented in this example".to_string()))
    }
    
    fn get_handlers_for_event(&self, event_type: &str) -> Vec<&dyn IamEventHandler> {
        self.handlers
            .get(event_type)
            .map(|handlers| handlers.iter().map(|h| h.as_ref()).collect())
            .unwrap_or_default()
    }
}

impl Default for EventProcessingServiceImpl {
    fn default() -> Self {
        Self::new()
    }
} 