use apparatus_events::{ApparatusDomainEvent, ComponentStatusChangedEvent};
use manifesto_domain::DomainError;
use rustycog_events::{ConcreteEventPublisher, DomainEvent, EventPublisher};
use std::sync::Arc;
use tracing::{debug, error};

/// Event adapter for publishing apparatus domain events
pub struct ApparatusEventAdapter {
    event_publisher: Arc<ConcreteEventPublisher>,
}

impl ApparatusEventAdapter {
    pub fn new(event_publisher: Arc<ConcreteEventPublisher>) -> Self {
        Self { event_publisher }
    }

    /// Publish a component status changed event
    pub async fn publish_component_status_changed(
        &self,
        event: ComponentStatusChangedEvent,
    ) -> Result<(), DomainError> {
        debug!(
            "Publishing component status changed event for project {} component {}",
            event.project_id, event.component_type
        );

        let domain_event: Box<dyn DomainEvent> =
            Box::new(ApparatusDomainEvent::ComponentStatusChanged(event));

        self.event_publisher
            .publish(&domain_event)
            .await
            .map_err(|e| {
                error!("Failed to publish component status changed event: {}", e);
                DomainError::internal_error(&format!("Failed to publish event: {}", e))
            })?;

        debug!("Successfully published component status changed event");
        Ok(())
    }

    /// Publish a generic apparatus domain event
    pub async fn publish_event(&self, event: ApparatusDomainEvent) -> Result<(), DomainError> {
        debug!("Publishing apparatus domain event: {}", event.event_type());

        let domain_event: Box<dyn DomainEvent> = Box::new(event);

        self.event_publisher
            .publish(&domain_event)
            .await
            .map_err(|e| {
                error!("Failed to publish event: {}", e);
                DomainError::internal_error(&format!("Failed to publish event: {}", e))
            })?;

        debug!("Successfully published event");
        Ok(())
    }
}
