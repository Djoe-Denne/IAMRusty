//! Process event command for Telegraph application

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use iam_events::{IamDomainEvent, DomainEvent};
use rustycog_command::{Command, CommandError, CommandHandler, CommandErrorMapper};
use crate::usecase::EventProcessingUseCaseTrait;
use telegraph_domain::DomainError;

/// Command to process an IAM domain event
#[derive(Debug, Clone)]
pub struct ProcessEventCommand {
    /// Command ID for tracking
    pub id: Uuid,
    /// Message recipient information extracted from the event
    pub recipient: SendMessageRecipient,
    /// Message content (dummy for now)
    pub event: Arc<dyn DomainEvent>,
    /// Processing attempt number
    pub attempt: u32,
    /// Additional context metadata
    pub metadata: HashMap<String, String>,
    /// Original event ID
    pub original_event_id: Uuid,
    /// Event type
    pub event_type: String,
}


/// Recipient information for send message command
#[derive(Debug, Clone)]
pub struct SendMessageRecipient {
    /// User ID (if known)
    pub user_id: Option<Uuid>,
    /// Email address (for email messages)
    pub email: Option<String>,
 
}

fn default_attempt() -> u32 {
    1
}

impl ProcessEventCommand {
    /// Create a new process event command from an IAM domain event
    pub fn new(event: Arc<dyn DomainEvent>) -> Self {
        let recipient = Self::extract_recipient_from_event(&event);
        
        Self {
            id: Uuid::new_v4(),
            recipient,
            event_type: event.event_type().to_string(),
            original_event_id: event.event_id(),
            event,
            attempt: 1,
            metadata: HashMap::new(),
        }
    }
    
    /// Extract recipient information from IAM domain event
    fn extract_recipient_from_event(event: &Arc<dyn DomainEvent>) -> SendMessageRecipient {
        // Use aggregation_id as user_id (BaseEvent attribute)
        let user_id = Some(event.aggregate_id());
        
        // Convert event to JSON and look for email field
        let email = match event.to_json() {
            Ok(json_str) => {
                match serde_json::from_str::<serde_json::Value>(&json_str) {
                    Ok(json_value) => Self::extract_email_from_json(&json_value),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        };
        
        SendMessageRecipient {
            user_id,
            email
        }
    }
    
    /// Extract email from JSON recursively
    fn extract_email_from_json(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::Object(obj) => {
                // Try to find email field directly
                if let Some(serde_json::Value::String(email)) = obj.get("email") {
                    return Some(email.clone());
                }
                
                // Search recursively in nested objects
                for val in obj.values() {
                    if let Some(email) = Self::extract_email_from_json(val) {
                        return Some(email);
                    }
                }
            }
            _ => {}
        }
        None
    }
    
    /// Set the attempt number
    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt;
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// Get the original event ID
    pub fn event_id(&self) -> Uuid {
        self.original_event_id
    }
    
    /// Get the event type
    pub fn event_type(&self) -> &str {
        &self.event_type
    }
    
    /// Get the user ID associated with the event
    pub fn user_id(&self) -> Option<Uuid> {
        self.recipient.user_id
    }
}

#[async_trait]
impl Command for ProcessEventCommand {
    type Result = ();

    fn command_type(&self) -> &'static str {
        "process_event"
    }

    fn command_id(&self) -> Uuid {
        self.id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // Validate that we have a valid original event ID
        if self.original_event_id == Uuid::nil() {
            return Err(CommandError::validation("invalid_event_id", "Original event ID cannot be nil"));
        }
        
        if self.event_type.is_empty() {
            return Err(CommandError::validation("empty_event_type", "Event type cannot be empty"));
        }
        
        if self.attempt == 0 {
            return Err(CommandError::validation("invalid_attempt", "Attempt number must be greater than 0"));
        }
        
        // Validate that we have at least some recipient information
        if self.recipient.user_id.is_none() && self.recipient.email.is_none() {
            return Err(CommandError::validation("invalid_recipient", "Recipient must have either user_id or email"));
        }
        
        Ok(())
    }
}

/// Command handler for processing IAM domain events
pub struct ProcessEventCommandHandler<E>
where
    E: EventProcessingUseCaseTrait + ?Sized,
{
    event_processing_usecase: Arc<E>,
}

impl<E> ProcessEventCommandHandler<E>
where
    E: EventProcessingUseCaseTrait + ?Sized,
{
    /// Create a new process event command handler
    pub fn new(event_processing_usecase: Arc<E>) -> Self {
        Self {
            event_processing_usecase,
        }
    }
}

#[async_trait]
impl<E> CommandHandler<ProcessEventCommand> for ProcessEventCommandHandler<E>
where
    E: EventProcessingUseCaseTrait + ?Sized,
{
    async fn handle(&self, command: ProcessEventCommand) -> Result<(), CommandError> {
        self.event_processing_usecase
            .process_event(command)
            .await
            .map_err(|e| CommandError::infrastructure("event_processing_failed", e.to_string()))
    }
}

/// Error mapper for process event commands
pub struct ProcessEventErrorMapper;

impl CommandErrorMapper for ProcessEventErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(domain_error) = error.downcast_ref::<DomainError>() {
            match domain_error {
                DomainError::InvalidMessage(msg) => 
                    CommandError::validation("invalid_message", msg.clone()),
                DomainError::InvalidRecipient(msg) => 
                    CommandError::validation("invalid_recipient", msg.clone()),
                DomainError::InvalidEmail(msg) => 
                    CommandError::validation("invalid_email", msg.clone()),
                DomainError::ConfigurationError(msg) => 
                    CommandError::business("configuration_error", msg.clone()),
                DomainError::TemplateNotFound(msg) => 
                    CommandError::business("template_not_found", msg.clone()),
                DomainError::EventProcessingError(msg) => 
                    CommandError::business("event_processing_error", msg.clone()),
                DomainError::InfrastructureError(msg) => 
                    CommandError::infrastructure("infrastructure_error", msg.clone()),
                DomainError::ServiceUnavailable(msg) => 
                    CommandError::infrastructure("service_unavailable", msg.clone()),
                _ => CommandError::infrastructure("unknown_domain_error", domain_error.to_string()),
            }
        } else {
            CommandError::infrastructure("unknown_error", error.to_string())
        }
    }
} 