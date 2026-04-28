use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog_events::BaseEvent;

use crate::Role;

/// Event published when an invitation is created
/// This triggers an email notification via Telegraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationCreatedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub invitation_id: Uuid,
    pub email: String,
    pub roles: Vec<Role>,
    pub invited_by_user_id: Uuid,
    pub invitation_token: String,
    pub expires_at: DateTime<Utc>,
}

/// Event published when an invitation is accepted
/// This triggers a confirmation email via Telegraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationAcceptedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub invitation_id: Uuid,
    pub user_id: Uuid,
    pub accepted_at: DateTime<Utc>,
}

/// Event published when an invitation expires
/// This triggers an expiry notification via Telegraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationExpiredEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub invitation_id: Uuid,
    pub email: String,
    pub expired_at: DateTime<Utc>,
}

pub struct InvitationCreatedEventData {
    pub organization_id: Uuid,
    pub organization_name: String,
    pub invitation_id: Uuid,
    pub email: String,
    pub roles: Vec<Role>,
    pub invited_by_user_id: Uuid,
    pub invitation_token: String,
    pub expires_at: DateTime<Utc>,
}

// =============================================================================
// Implementations
// =============================================================================

impl InvitationCreatedEvent {
    #[must_use]
    pub fn new(data: InvitationCreatedEventData) -> Self {
        Self {
            base: BaseEvent::new("invitation_created".to_string(), data.organization_id),
            organization_id: data.organization_id,
            organization_name: data.organization_name,
            invitation_id: data.invitation_id,
            email: data.email,
            roles: data.roles,
            invited_by_user_id: data.invited_by_user_id,
            invitation_token: data.invitation_token,
            expires_at: data.expires_at,
        }
    }
}

impl InvitationAcceptedEvent {
    #[must_use]
    pub fn new(
        organization_id: Uuid,
        organization_name: String,
        invitation_id: Uuid,
        user_id: Uuid,
        accepted_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("invitation_accepted".to_string(), organization_id),
            organization_id,
            organization_name,
            invitation_id,
            user_id,
            accepted_at,
        }
    }
}

impl InvitationExpiredEvent {
    #[must_use]
    pub fn new(
        organization_id: Uuid,
        organization_name: String,
        invitation_id: Uuid,
        email: String,
        expired_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("invitation_expired".to_string(), organization_id),
            organization_id,
            organization_name,
            invitation_id,
            email,
            expired_at,
        }
    }
}
