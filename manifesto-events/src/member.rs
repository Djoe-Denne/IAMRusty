//! Member and permission domain events for Manifesto service

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog_events::BaseEvent;

// =============================================================================
// Member Events
// =============================================================================

/// Event published when a member is added to a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberAddedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub member_id: Uuid,
    pub user_id: Uuid,
    pub initial_permission: String,
    pub initial_resource: String,
    pub added_by: Uuid,
    pub added_at: DateTime<Utc>,
}

impl MemberAddedEvent {
    pub fn new(
        project_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        initial_permission: String,
        initial_resource: String,
        added_by: Uuid,
        added_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("member_added".to_string(), project_id),
            project_id,
            member_id,
            user_id,
            initial_permission,
            initial_resource,
            added_by,
            added_at,
        }
    }
}

/// Event published when a member's permissions are updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberPermissionsUpdatedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub member_id: Uuid,
    pub user_id: Uuid,
    pub permissions: Vec<ResourcePermission>,
    pub updated_by: Uuid,
    pub updated_at: DateTime<Utc>,
}

/// A resource-permission pair for event payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePermission {
    pub resource: String,
    pub permission: String,
}

impl MemberPermissionsUpdatedEvent {
    pub fn new(
        project_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        permissions: Vec<ResourcePermission>,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("member_permissions_updated".to_string(), project_id),
            project_id,
            member_id,
            user_id,
            permissions,
            updated_by,
            updated_at,
        }
    }
}

/// Event published when a member is removed from a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberRemovedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub member_id: Uuid,
    pub user_id: Uuid,
    pub removed_by: Uuid,
    pub removed_at: DateTime<Utc>,
}

impl MemberRemovedEvent {
    pub fn new(
        project_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        removed_by: Uuid,
        removed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("member_removed".to_string(), project_id),
            project_id,
            member_id,
            user_id,
            removed_by,
            removed_at,
        }
    }
}

/// Event published when a permission is granted to a member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGrantedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub member_id: Uuid,
    pub user_id: Uuid,
    pub resource: String,
    pub permission: String,
    pub granted_by: Uuid,
    pub granted_at: DateTime<Utc>,
}

impl PermissionGrantedEvent {
    pub fn new(
        project_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        resource: String,
        permission: String,
        granted_by: Uuid,
        granted_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("permission_granted".to_string(), project_id),
            project_id,
            member_id,
            user_id,
            resource,
            permission,
            granted_by,
            granted_at,
        }
    }
}

/// Event published when a permission is revoked from a member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRevokedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub member_id: Uuid,
    pub user_id: Uuid,
    pub resource: String,
    pub revoked_by: Uuid,
    pub revoked_at: DateTime<Utc>,
}

impl PermissionRevokedEvent {
    pub fn new(
        project_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        resource: String,
        revoked_by: Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("permission_revoked".to_string(), project_id),
            project_id,
            member_id,
            user_id,
            resource,
            revoked_by,
            revoked_at,
        }
    }
}


