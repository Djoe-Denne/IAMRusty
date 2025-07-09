//! Event handler port interfaces for Telegraph service

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::DomainError;
use iam_events::IamDomainEvent;

/// Port for handling IAM domain events
#[async_trait]
pub trait IamEventHandler: Send + Sync {
    /// Handle an IAM domain event and convert it to communication messages
    async fn handle_event(&self, event: &IamDomainEvent) -> Result<(), DomainError>;
    
    /// Check if this handler supports a specific event type
    fn supports_event_type(&self, event_type: &str) -> bool;
    
    /// Get the priority of this handler (higher numbers = higher priority)
    fn priority(&self) -> u32 {
        100 // Default priority
    }
}

/// Port for event processing coordination
#[async_trait]
pub trait EventProcessor: Send + Sync {
    /// Process an event with appropriate handlers
    async fn process_event(&self, event: &IamDomainEvent) -> Result<(), DomainError>;
    
    /// Register an event handler
    async fn register_handler(&self, handler: Box<dyn IamEventHandler>) -> Result<(), DomainError>;
    
    /// Get handlers for a specific event type
    fn get_handlers_for_event(&self, event_type: &str) -> Vec<&dyn IamEventHandler>;
}

/// Context information for event processing
#[derive(Debug, Clone)]
pub struct EventContext {
    /// Event ID for tracking
    pub event_id: Uuid,
    /// Event type
    pub event_type: String,
    /// User ID associated with the event
    pub user_id: Uuid,
    /// Queue name that received the event
    pub queue_name: String,
    /// Processing attempt number
    pub attempt: u32,
    /// Additional context metadata
    pub metadata: std::collections::HashMap<String, String>,
} 