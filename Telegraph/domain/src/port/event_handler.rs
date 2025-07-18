//! Event handler port interfaces for Telegraph service

use async_trait::async_trait;
use uuid::Uuid;
use std::sync::Arc;

use crate::error::DomainError;
use rustycog_events::DomainEvent;

/// Port for handling IAM domain events
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle an IAM domain event and convert it to communication messages
    async fn handle_event(&self, event: &EventContext) -> Result<(), DomainError>;
    
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
    async fn process_event(&self, event: &EventContext) -> Result<(), DomainError>;
    
    /// Get handlers for a specific event type
    fn get_handlers_for_event(&self, event_type: &str) -> Vec<&dyn EventHandler>;
}

/// Context information for event processing
#[derive(Debug)]
pub struct EventContext {
    /// Event ID for tracking
    pub event_id: Uuid,
    /// Event type
    pub event_type: String,
    /// Recipient information
    pub recipient: EventRecipient,
    /// Content information
    pub event: Arc<dyn DomainEvent>,
    /// Processing attempt number
    pub attempt: u32,
    /// Additional context metadata
    pub metadata: std::collections::HashMap<String, String>,
} 


/// Recipient information for send message command
#[derive(Debug)]
pub struct EventRecipient {
    /// User ID (if known)
    pub user_id: Option<Uuid>,
    /// Email address (for email messages)
    pub email: Option<String>,
 
}
