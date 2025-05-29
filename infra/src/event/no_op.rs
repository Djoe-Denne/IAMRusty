use async_trait::async_trait;
use domain::entity::events::DomainEvent;
use domain::error::DomainError;
use domain::port::event_publisher::EventPublisher;

/// No-op event publisher for testing and development
/// 
/// This publisher doesn't actually publish events anywhere,
/// but provides a valid implementation for development environments
/// where event publishing is not needed.
pub struct NoOpEventPublisher;

impl NoOpEventPublisher {
    /// Create a new NoOpEventPublisher
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoOpEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventPublisher for NoOpEventPublisher {
    async fn publish(&self, event: DomainEvent) -> Result<(), DomainError> {
        // Log the event but don't actually publish it
        tracing::debug!(
            event_id = %event.event_id(),
            event_type = %event.event_type(),
            user_id = %event.user_id(),
            "Event would be published (no-op mode)"
        );
        Ok(())
    }

    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<(), DomainError> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }

    async fn health_check(&self) -> Result<(), DomainError> {
        // Always healthy since no-op
        Ok(())
    }
} 