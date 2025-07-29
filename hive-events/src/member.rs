//! Hive Domain Events
//! 
//! This crate contains all domain events for the Hive organization management service.
//! Events are used for inter-service communication, particularly with the Telegraph
//! notification service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog_events::BaseEvent;

// =============================================================================
// Member Events
// =============================================================================

/// Event published when a member is invited to an organization
/// This triggers an email notification via Telegraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberInvitedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub invitation_id: Uuid,
    pub email: String,
    pub role_name: String,
    pub invited_by_user_id: Uuid,
    pub invitation_token: String,
    pub expires_at: DateTime<Utc>,
    pub message: Option<String>,
}

/// Event published when a member joins an organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberJoinedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub user_id: Uuid,
    pub role_name: String,
    pub joined_at: DateTime<Utc>,
}

/// Event published when a member is removed from an organization
/// This triggers an email notification via Telegraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberRemovedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub user_id: Uuid,
    pub user_email: String,
    pub removed_by_user_id: Uuid,
    pub removed_at: DateTime<Utc>,
}

// =============================================================================
// Member Events implementations
// =============================================================================

impl MemberInvitedEvent {
    pub fn new(organization_id: Uuid, organization_name: String, invitation_id: Uuid, email: String, role_name: String, invited_by_user_id: Uuid, invitation_token: String, expires_at: DateTime<Utc>, message: Option<String>) -> Self {
        Self {
            base: BaseEvent::new("member_invited".to_string(), organization_id),
            organization_id,
            organization_name,
            invitation_id,
            email,
            role_name,
            invited_by_user_id,
            invitation_token,
            expires_at,
            message,
        }
    }
}

impl MemberJoinedEvent {
    pub fn new(organization_id: Uuid, organization_name: String, user_id: Uuid, role_name: String, joined_at: DateTime<Utc>) -> Self {
        Self {
            base: BaseEvent::new("member_joined".to_string(), organization_id),
            organization_id,
            organization_name,
            user_id,
            role_name,
            joined_at,
        }
    }
}


