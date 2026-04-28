use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog_events::BaseEvent;

// =============================================================================
// Component Events
// =============================================================================

/// Event published when a component status changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatusChangedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub component_type: String,
    pub old_status: String,
    pub new_status: String,
    pub changed_at: DateTime<Utc>,
}

impl ComponentStatusChangedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        component_type: String,
        old_status: String,
        new_status: String,
        changed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("component_status_changed".to_string(), project_id),
            project_id,
            component_type,
            old_status,
            new_status,
            changed_at,
        }
    }
}
