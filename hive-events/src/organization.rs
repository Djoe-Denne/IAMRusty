use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog_events::{BaseEvent, DomainEvent};

// =============================================================================
// Organization Events
// =============================================================================

/// Event published when a new organization is created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationCreatedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub organization_slug: String,
    pub owner_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Event published when an organization is updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationUpdatedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub updated_fields: Vec<String>,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

/// Event published when an organization is deleted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationDeletedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub deleted_by_user_id: Uuid,
    pub deleted_at: DateTime<Utc>,
}

// =============================================================================
// Organization Events implementations
// =============================================================================

impl OrganizationCreatedEvent {
    pub fn new(
        organization_id: Uuid,
        organization_name: String,
        organization_slug: String,
        owner_user_id: Uuid,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("organization_created".to_string(), organization_id),
            organization_id,
            organization_name,
            organization_slug,
            owner_user_id,
            created_at,
        }
    }
}

impl OrganizationUpdatedEvent {
    pub fn new(
        organization_id: Uuid,
        organization_name: String,
        updated_fields: Vec<String>,
        updated_by_user_id: Uuid,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("organization_updated".to_string(), organization_id),
            organization_id,
            organization_name,
            updated_fields,
            updated_by_user_id,
            updated_at,
        }
    }
}

impl OrganizationDeletedEvent {
    pub fn new(
        organization_id: Uuid,
        organization_name: String,
        deleted_by_user_id: Uuid,
        deleted_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("organization_deleted".to_string(), organization_id),
            organization_id,
            organization_name,
            deleted_by_user_id,
            deleted_at,
        }
    }
}
