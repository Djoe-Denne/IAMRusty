//! Event processing use case for Telegraph application

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{error, info};

use crate::command::ProcessEventCommand;
use telegraph_domain::{DomainError, EventContext, EventProcessor, EventRecipient};

/// Use case for handling event processing operations
pub struct EventProcessingUseCase {
    event_processor: Arc<dyn EventProcessor>,
}

impl EventProcessingUseCase {
    /// Create a new event processing use case
    pub fn new(event_processor: Arc<dyn EventProcessor>) -> Self {
        Self { event_processor }
    }

    /// Process an IAM domain event
    pub async fn process_event(&self, command: ProcessEventCommand) -> Result<(), DomainError> {
        // Extract values for logging before converting the command
        let event_id = command.event_id();
        let event_type = command.event_type().to_string();
        let attempt = command.attempt;

        // Process the event through the domain event processor
        let result = self.event_processor.process_event(&command.into()).await;

        match &result {
            Ok(()) => {
                info!(
                    event_id = %event_id,
                    event_type = %event_type,
                    "Event processed successfully"
                );
            }
            Err(e) => {
                error!(
                    event_id = %event_id,
                    event_type = %event_type,
                    error = %e,
                    attempt = attempt,
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

/// Mapper for Event command to Event context
impl From<ProcessEventCommand> for EventContext {
    fn from(command: ProcessEventCommand) -> Self {
        Self {
            event_id: command.event_id(),
            event_type: command.event_type().to_string(),
            recipient: EventRecipient {
                user_id: command.recipient.user_id,
                email: command.recipient.email,
            },
            event: command.event,
            attempt: command.attempt,
            metadata: command.metadata,
        }
    }
}
