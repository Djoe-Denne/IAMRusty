//! Component domain events for Manifesto service

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog_events::BaseEvent;

// =============================================================================
// Component Events
// =============================================================================

/// Event published when a component is added to a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentAddedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub component_id: Uuid,
    pub component_type: String,
    pub added_by: Uuid,
    pub added_at: DateTime<Utc>,
}

impl ComponentAddedEvent {
    pub fn new(
        project_id: Uuid,
        component_id: Uuid,
        component_type: String,
        added_by: Uuid,
        added_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("component_added".to_string(), project_id),
            project_id,
            component_id,
            component_type,
            added_by,
            added_at,
        }
    }
}

/// Event published when a component's status changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatusChangedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub component_id: Uuid,
    pub component_type: String,
    pub old_status: String,
    pub new_status: String,
    pub changed_by: Uuid,
    pub changed_at: DateTime<Utc>,
}

impl ComponentStatusChangedEvent {
    pub fn new(
        project_id: Uuid,
        component_id: Uuid,
        component_type: String,
        old_status: String,
        new_status: String,
        changed_by: Uuid,
        changed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("component_status_changed".to_string(), project_id),
            project_id,
            component_id,
            component_type,
            old_status,
            new_status,
            changed_by,
            changed_at,
        }
    }
}

/// Event published when a component is removed from a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRemovedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub component_id: Uuid,
    pub component_type: String,
    pub removed_by: Uuid,
    pub removed_at: DateTime<Utc>,
}

impl ComponentRemovedEvent {
    pub fn new(
        project_id: Uuid,
        component_id: Uuid,
        component_type: String,
        removed_by: Uuid,
        removed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("component_removed".to_string(), project_id),
            project_id,
            component_id,
            component_type,
            removed_by,
            removed_at,
        }
    }
}
