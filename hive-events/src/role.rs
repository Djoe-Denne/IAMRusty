//! Hive Domain Events
//! 
//! This crate contains all domain events for the Hive organization management service.
//! Events are used for inter-service communication, particularly with the Telegraph
//! notification service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog_events::{DomainEvent, BaseEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum HiveDomainEvent {
    #[serde(rename = "organization_created")]
    OrganizationCreated(OrganizationCreatedEvent),
    #[serde(rename = "organization_updated")]
    OrganizationUpdated(OrganizationUpdatedEvent),
    #[serde(rename = "organization_deleted")]
    OrganizationDeleted(OrganizationDeletedEvent),
    #[serde(rename = "member_invited")]
    MemberInvited(MemberInvitedEvent),
    #[serde(rename = "member_joined")]
    MemberJoined(MemberJoinedEvent),
    #[serde(rename = "member_removed")]
    MemberRemoved(MemberRemovedEvent),
    #[serde(rename = "invitation_created")]
    InvitationCreated(InvitationCreatedEvent),
    #[serde(rename = "invitation_accepted")]
    InvitationAccepted(InvitationAcceptedEvent),
    #[serde(rename = "invitation_expired")]
    InvitationExpired(InvitationExpiredEvent),
    #[serde(rename = "external_link_created")]
    ExternalLinkCreated(ExternalLinkCreatedEvent),
    #[serde(rename = "sync_job_started")]
    SyncJobStarted(SyncJobStartedEvent),
    #[serde(rename = "sync_job_completed")]
    SyncJobCompleted(SyncJobCompletedEvent),
}

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
// Invitation Events
// =============================================================================

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
    pub role_name: String,
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
// Sync Events
// =============================================================================

/// Event published when a sync job starts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobStartedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub external_link_id: Uuid,
    pub sync_job_id: Uuid,
    pub job_type: String,
    pub started_at: DateTime<Utc>,
}

/// Event published when a sync job completes (success or failure)
/// Failed jobs trigger error notifications via Telegraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub external_link_id: Uuid,
    pub sync_job_id: Uuid,
    pub job_type: String,
    pub status: String, // "completed", "failed"
    pub items_processed: i32,
    pub items_created: i32,
    pub items_updated: i32,
    pub items_failed: i32,
    pub error_message: Option<String>,
    pub completed_at: DateTime<Utc>,
}

// =============================================================================
// Organization Events implementations
// =============================================================================

impl OrganizationCreatedEvent {
    pub fn new(organization_id: Uuid, organization_name: String, organization_slug: String, owner_user_id: Uuid, created_at: DateTime<Utc>) -> Self {
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
    pub fn new(organization_id: Uuid, organization_name: String, updated_fields: Vec<String>, updated_by_user_id: Uuid, updated_at: DateTime<Utc>) -> Self {
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
    pub fn new(organization_id: Uuid, organization_name: String, deleted_by_user_id: Uuid, deleted_at: DateTime<Utc>) -> Self {
        Self {
            base: BaseEvent::new("organization_deleted".to_string(), organization_id),
            organization_id,
            organization_name,
            deleted_by_user_id,
            deleted_at,
        }
    }
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
