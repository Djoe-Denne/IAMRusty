//! Process event command for Telegraph application

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use iam_events::{IamDomainEvent, DomainEvent};

/// Command to process an IAM domain event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEventCommand {
    /// The IAM domain event to process
    pub event: IamDomainEvent,
    /// Queue name that received the event
    pub queue_name: String,
    /// Processing attempt number
    #[serde(default = "default_attempt")]
    pub attempt: u32,
    /// Additional context metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_attempt() -> u32 {
    1
}

impl ProcessEventCommand {
    /// Create a new process event command
    pub fn new(event: IamDomainEvent, queue_name: String) -> Self {
        Self {
            event,
            queue_name,
            attempt: 1,
            metadata: HashMap::new(),
        }
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
    
    /// Get the event ID
    pub fn event_id(&self) -> Uuid {
        self.event.event_id()
    }
    
    /// Get the event type
    pub fn event_type(&self) -> &str {
        self.event.event_type()
    }
    
    /// Get the user ID associated with the event
    pub fn user_id(&self) -> Uuid {
        self.event.user_id()
    }
} 