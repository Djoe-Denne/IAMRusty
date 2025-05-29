use async_trait::async_trait;
use crate::entity::events::DomainEvent;
use crate::error::DomainError;

/// Port for publishing domain events to external systems
/// 
/// This port defines the contract for event publishing functionality
/// without coupling the domain to specific message broker implementations.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a domain event
    /// 
    /// # Arguments
    /// * `event` - The domain event to publish
    /// 
    /// # Returns
    /// * `Ok(())` if the event was successfully published
    /// * `Err(DomainError)` if publishing failed
    async fn publish(&self, event: DomainEvent) -> Result<(), DomainError>;

    /// Publish multiple domain events in a batch
    /// 
    /// This method should handle events atomically where possible.
    /// If the underlying system doesn't support transactions,
    /// it should publish events individually and report any failures.
    /// 
    /// # Arguments
    /// * `events` - Vector of domain events to publish
    /// 
    /// # Returns
    /// * `Ok(())` if all events were successfully published
    /// * `Err(DomainError)` if any event failed to publish
    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<(), DomainError> {
        // Default implementation publishes events individually
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }

    /// Check if the event publisher is healthy and ready to publish events
    /// 
    /// This method can be used for health checks and circuit breaker patterns.
    /// 
    /// # Returns
    /// * `Ok(())` if the publisher is healthy
    /// * `Err(DomainError)` if the publisher is not ready
    async fn health_check(&self) -> Result<(), DomainError>;
} 