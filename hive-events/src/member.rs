//! Hive Domain Events
//!
//! This crate contains all domain events for the Hive organization management service.
//! Events are used for inter-service communication, particularly with the Telegraph
//! notification service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog_events::BaseEvent;

use crate::Role;

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
    pub roles: Vec<Role>,
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
    pub roles: Vec<Role>,
    pub joined_at: DateTime<Utc>,
}

/// Event published when a member's roles are updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberRolesUpdatedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub user_id: Uuid,
    pub roles: Vec<Role>,
    pub updated_at: DateTime<Utc>,
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
    pub fn new(
        organization_id: Uuid,
        organization_name: String,
        invitation_id: Uuid,
        email: String,
        roles: Vec<Role>,
        invited_by_user_id: Uuid,
        invitation_token: String,
        expires_at: DateTime<Utc>,
        message: Option<String>,
    ) -> Self {
        Self {
            base: BaseEvent::new("member_invited".to_string(), organization_id),
            organization_id,
            organization_name,
            invitation_id,
            email,
            roles,
            invited_by_user_id,
            invitation_token,
            expires_at,
            message,
        }
    }
}

impl MemberJoinedEvent {
    pub fn new(
        organization_id: Uuid,
        organization_name: String,
        user_id: Uuid,
        roles: Vec<Role>,
        joined_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("member_joined".to_string(), organization_id),
            organization_id,
            organization_name,
            user_id,
            roles,
            joined_at,
        }
    }
}

impl MemberRolesUpdatedEvent {

    pub fn new(
        organization_id: Uuid,
        organization_name: String,
        user_id: Uuid,
        roles: Vec<Role>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("member_roles_updated".to_string(), organization_id),
            organization_id,
            organization_name,
            user_id,
            roles,
            updated_at,
        }
    }
}

impl MemberRemovedEvent {
    pub fn new(
        organization_id: Uuid,
        organization_name: String,
        user_id: Uuid,
        user_email: String,
        removed_by_user_id: Uuid,
        removed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("member_removed".to_string(), organization_id.clone()),
            organization_id: organization_id.clone(),
            organization_name,
            user_id,
            user_email,
            removed_by_user_id,
            removed_at,
        }
    }
}
