//! Event abstractions for domain events and event publishing

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use rustycog_core::error::ServiceError;

/// Core trait for domain events
pub trait DomainEvent: Send + Sync + std::fmt::Debug {
    /// Get the event type identifier
    fn event_type(&self) -> & str;
    
    /// Get the event ID
    fn event_id(&self) -> Uuid;
    
    /// Get the aggregate ID that this event relates to
    fn aggregate_id(&self) -> Uuid;
    
    /// Get the timestamp when this event occurred
    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc>;
    
    /// Get the event version for schema evolution
    fn version(&self) -> u32;
    
    /// Serialize the event to JSON
    fn to_json(&self) -> Result<String, ServiceError>;
    
    /// Get event metadata
    fn metadata(&self) -> HashMap<String, String>;
}

/// Event publisher trait for publishing domain events
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a single event
    async fn publish(&self, event: Box<dyn DomainEvent>) -> Result<(), ServiceError>;
    
    /// Publish multiple events in a batch
    async fn publish_batch(&self, events: Vec<Box<dyn DomainEvent>>) -> Result<(), ServiceError>;
    
    /// Health check for the event publisher
    async fn health_check(&self) -> Result<(), ServiceError>;
}

/// Event subscriber trait for consuming domain events
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Subscribe to events of a specific type
    async fn subscribe(&self, event_type: &str) -> Result<(), ServiceError>;
    
    /// Unsubscribe from events of a specific type
    async fn unsubscribe(&self, event_type: &str) -> Result<(), ServiceError>;
    
    /// Start consuming events
    async fn start_consuming(&self) -> Result<(), ServiceError>;
    
    /// Stop consuming events
    async fn stop_consuming(&self) -> Result<(), ServiceError>;
}

/// Event handler trait for processing domain events
#[async_trait]
pub trait EventHandler<E: DomainEvent>: Send + Sync {
    /// Handle a domain event
    async fn handle(&self, event: E) -> Result<(), ServiceError>;
}

/// Base implementation for domain events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEvent {
    /// Event ID
    pub event_id: Uuid,
    /// Aggregate ID
    pub aggregate_id: Uuid,
    /// Event type
    pub event_type: String,
    /// When the event occurred
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    /// Event version
    pub version: u32,
    /// Event metadata
    pub metadata: HashMap<String, String>,
}

impl BaseEvent {
    /// Create a new base event
    pub fn new(event_type: String, aggregate_id: Uuid) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            aggregate_id,
            event_type,
            occurred_at: chrono::Utc::now(),
            version: 1,
            metadata: HashMap::new(),
        }
    }
    
    /// Add metadata to the event
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// Set the event version
    pub fn with_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }
} 