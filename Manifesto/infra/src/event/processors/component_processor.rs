use std::sync::Arc;

use apparatus_events::ComponentStatusChangedEvent;
use manifesto_domain::{service::ComponentService, value_objects::ComponentStatus};
use rustycog_core::error::ServiceError;
use tracing::{info, warn};

/// Processor for component status changed events
pub struct ComponentStatusProcessor {
    component_service: Arc<dyn ComponentService>,
}

impl ComponentStatusProcessor {
    pub fn new(component_service: Arc<dyn ComponentService>) -> Self {
        Self { component_service }
    }

    fn parse_status(raw_status: &str, field_name: &str) -> Result<ComponentStatus, ServiceError> {
        ComponentStatus::from_str(raw_status).map_err(|error| {
            ServiceError::validation(format!(
                "Invalid {field_name} '{raw_status}' in component status event: {error}"
            ))
        })
    }

    /// Process a component status changed event
    pub async fn process(&self, event: ComponentStatusChangedEvent) -> Result<(), ServiceError> {
        let expected_old_status = Self::parse_status(&event.old_status, "old_status")?;
        let target_status = Self::parse_status(&event.new_status, "new_status")?;
        let mut component = self
            .component_service
            .get_component_by_type(&event.project_id, &event.component_type)
            .await
            .map_err(ServiceError::from)?;

        info!(
            event_id = %event.base.event_id,
            project_id = %event.project_id,
            component_type = %event.component_type,
            current_status = %component.status,
            expected_old_status = %expected_old_status,
            target_status = %target_status,
            "Processing apparatus component status change"
        );

        // Duplicate deliveries are expected with queue-based consumers.
        if component.status == target_status {
            info!(
                event_id = %event.base.event_id,
                project_id = %event.project_id,
                component_type = %event.component_type,
                target_status = %target_status,
                "Skipping apparatus component status event because target status is already applied"
            );
            return Ok(());
        }

        // If the component has already moved elsewhere, ignore the stale event instead of
        // rewinding state or poisoning the queue with a permanently non-applicable message.
        if component.status != expected_old_status {
            warn!(
                event_id = %event.base.event_id,
                project_id = %event.project_id,
                component_type = %event.component_type,
                current_status = %component.status,
                expected_old_status = %expected_old_status,
                target_status = %target_status,
                "Ignoring stale apparatus component status event because current state no longer matches the event precondition"
            );
            return Ok(());
        }

        let set_configured_at =
            target_status == ComponentStatus::Configured && component.configured_at.is_none();
        let set_activated_at =
            target_status == ComponentStatus::Active && component.activated_at.is_none();
        let set_disabled_at =
            target_status == ComponentStatus::Disabled && component.disabled_at.is_none();

        component
            .transition_status(target_status)
            .map_err(ServiceError::from)?;

        if set_configured_at {
            component.configured_at = Some(event.changed_at);
        }
        if set_activated_at {
            component.activated_at = Some(event.changed_at);
        }
        if set_disabled_at {
            component.disabled_at = Some(event.changed_at);
        }

        let updated_component = self
            .component_service
            .update_component(component)
            .await
            .map_err(ServiceError::from)?;

        info!(
            event_id = %event.base.event_id,
            project_id = %event.project_id,
            component_type = %event.component_type,
            applied_status = %updated_component.status,
            "Applied apparatus component status event successfully"
        );

        Ok(())
    }
}

