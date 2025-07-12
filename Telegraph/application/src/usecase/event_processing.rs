//! Event processing use case for Telegraph application

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, error};

use telegraph_domain::{DomainError, EventProcessor};
use crate::command::ProcessEventCommand;

/// Use case for handling event processing operations
pub struct EventProcessingUseCase {
    event_processor: Arc<dyn EventProcessor>,
}

impl EventProcessingUseCase {
    /// Create a new event processing use case
    pub fn new(event_processor: Arc<dyn EventProcessor>) -> Self {
        Self {
            event_processor,
        }
    }
    
    /// Process an IAM domain event
    pub async fn process_event(&self, command: ProcessEventCommand) -> Result<(), DomainError> {
        info!(
            event_id = %command.event_id(),
            event_type = command.event_type(),
            user_id = %command.user_id(),
            queue_name = %command.queue_name,
            attempt = command.attempt,
            "Processing IAM domain event"
        );
        
        // Process the event through the domain event processor
        let result = self.event_processor.process_event(&command.event).await;
        
        match &result {
            Ok(()) => {
                info!(
                    event_id = %command.event_id(),
                    event_type = command.event_type(),
                    "Event processed successfully"
                );
            }
            Err(e) => {
                error!(
                    event_id = %command.event_id(),
                    event_type = command.event_type(),
                    error = %e,
                    attempt = command.attempt,
                    "Event processing failed"
                );
            }
        }
        
        result
    }
}

/// Trait for event processing use case
#[async_trait]
pub trait EventProcessingUseCaseTrait: Send + Sync {
    /// Process an event
    async fn process_event(&self, command: ProcessEventCommand) -> Result<(), DomainError>;
}

#[async_trait]
impl EventProcessingUseCaseTrait for EventProcessingUseCase {
    async fn process_event(&self, command: ProcessEventCommand) -> Result<(), DomainError> {
        self.process_event(command).await
    }
} 