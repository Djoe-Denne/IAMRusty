use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog_events::BaseEvent;

// =============================================================================
// External Integration Events
// =============================================================================

/// Event published when an external provider link is created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalLinkCreatedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub external_link_id: Uuid,
    pub provider_type: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// Implementations
// =============================================================================

impl ExternalLinkCreatedEvent {
    pub fn new(
        organization_id: &Uuid,
        organization_name: String,
        external_link_id: &Uuid,
        provider_type: String,
        created_by_user_id: &Uuid,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("external_link_created".to_string(), organization_id.clone()),
            organization_id: organization_id.clone(),
            organization_name,
            external_link_id: external_link_id.clone(),
            provider_type,
            created_by_user_id: created_by_user_id.clone(),
            created_at,
        }
    }
}
