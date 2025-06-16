//! Event publisher port for publishing domain events

use crate::entity::events::DomainEvent;
use crate::error::DomainError;
use async_trait::async_trait;

/// Port for publishing domain events to external systems
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a single domain event
    async fn publish(&self, event: DomainEvent) -> Result<(), DomainError>;

    /// Publish multiple domain events in a batch
    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<(), DomainError>;

    /// Health check for the event publishing system
    async fn health_check(&self) -> Result<(), DomainError>;
}
