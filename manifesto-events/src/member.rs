//! Member and permission domain events for Manifesto service

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog::events::BaseEvent;

use crate::authz::{exact_user_tuple, resolve_authz_target, AuthzTuple};

const EVENT_SCHEMA_V2: u32 = 2;

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
    #[serde(default)]
    pub object_type: Option<String>,
    #[serde(default)]
    pub object_id: Option<Uuid>,
    pub added_by: Uuid,
    pub added_at: DateTime<Utc>,
}

impl MemberAddedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        initial_permission: String,
        initial_resource: String,
        added_by: Uuid,
        added_at: DateTime<Utc>,
    ) -> Self {
        let target = resolve_authz_target(&initial_resource, project_id);
        Self {
            base: BaseEvent::new("member_added".to_string(), project_id)
                .with_version(EVENT_SCHEMA_V2),
            project_id,
            member_id,
            user_id,
            initial_permission,
            initial_resource,
            object_type: target.as_ref().map(|t| t.object_type.clone()),
            object_id: target.as_ref().map(|t| t.object_id),
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
    /// Previous grants. Empty on v1 payloads — translators must not wipe.
    #[serde(default)]
    pub previous: Vec<ResourcePermission>,
    pub updated_by: Uuid,
    pub updated_at: DateTime<Utc>,
}

/// A resource-permission pair for event payloads
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourcePermission {
    pub resource: String,
    pub permission: String,
    #[serde(default)]
    pub object_type: Option<String>,
    #[serde(default)]
    pub object_id: Option<Uuid>,
}

impl ResourcePermission {
    #[must_use]
    pub fn new(resource: String, permission: String, project_id: Uuid) -> Self {
        let target = resolve_authz_target(&resource, project_id);
        Self {
            resource,
            permission,
            object_type: target.as_ref().map(|t| t.object_type.clone()),
            object_id: target.as_ref().map(|t| t.object_id),
        }
    }

    #[must_use]
    pub fn to_tuple(&self, project_id: Uuid, user_id: Uuid) -> Option<AuthzTuple> {
        if let (Some(object_type), Some(object_id)) = (&self.object_type, self.object_id) {
            return crate::authz::fga_relation(object_type, &self.permission).map(|relation| {
                AuthzTuple::new(object_type.clone(), object_id, relation, user_id)
            });
        }
        exact_user_tuple(&self.resource, &self.permission, project_id, user_id)
    }
}

impl MemberPermissionsUpdatedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        permissions: Vec<ResourcePermission>,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self::with_previous(
            project_id,
            member_id,
            user_id,
            Vec::new(),
            permissions,
            updated_by,
            updated_at,
        )
    }

    #[must_use]
    pub fn with_previous(
        project_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        previous: Vec<ResourcePermission>,
        permissions: Vec<ResourcePermission>,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("member_permissions_updated".to_string(), project_id)
                .with_version(EVENT_SCHEMA_V2),
            project_id,
            member_id,
            user_id,
            permissions,
            previous,
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
    /// Exact tuples to delete. Empty on v1 — translators must not wipe extras.
    #[serde(default)]
    pub tuples: Vec<AuthzTuple>,
    #[serde(default)]
    pub component_ids: Vec<Uuid>,
}

impl MemberRemovedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        removed_by: Uuid,
        removed_at: DateTime<Utc>,
    ) -> Self {
        Self::with_tuples(
            project_id,
            member_id,
            user_id,
            removed_by,
            removed_at,
            Vec::new(),
            Vec::new(),
        )
    }

    #[must_use]
    pub fn with_tuples(
        project_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        removed_by: Uuid,
        removed_at: DateTime<Utc>,
        tuples: Vec<AuthzTuple>,
        component_ids: Vec<Uuid>,
    ) -> Self {
        Self {
            base: BaseEvent::new("member_removed".to_string(), project_id)
                .with_version(EVENT_SCHEMA_V2),
            project_id,
            member_id,
            user_id,
            removed_by,
            removed_at,
            tuples,
            component_ids,
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
    #[serde(default)]
    pub object_type: Option<String>,
    #[serde(default)]
    pub object_id: Option<Uuid>,
    pub granted_by: Uuid,
    pub granted_at: DateTime<Utc>,
}

impl PermissionGrantedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        resource: String,
        permission: String,
        granted_by: Uuid,
        granted_at: DateTime<Utc>,
    ) -> Self {
        let target = resolve_authz_target(&resource, project_id);
        Self {
            base: BaseEvent::new("permission_granted".to_string(), project_id)
                .with_version(EVENT_SCHEMA_V2),
            project_id,
            member_id,
            user_id,
            resource,
            permission,
            object_type: target.as_ref().map(|t| t.object_type.clone()),
            object_id: target.as_ref().map(|t| t.object_id),
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
    #[serde(default)]
    pub permission: Option<String>,
    #[serde(default)]
    pub object_type: Option<String>,
    #[serde(default)]
    pub object_id: Option<Uuid>,
    pub revoked_by: Uuid,
    pub revoked_at: DateTime<Utc>,
}

impl PermissionRevokedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        resource: String,
        permission: String,
        revoked_by: Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Self {
        let target = resolve_authz_target(&resource, project_id);
        Self {
            base: BaseEvent::new("permission_revoked".to_string(), project_id)
                .with_version(EVENT_SCHEMA_V2),
            project_id,
            member_id,
            user_id,
            resource,
            permission: Some(permission),
            object_type: target.as_ref().map(|t| t.object_type.clone()),
            object_id: target.as_ref().map(|t| t.object_id),
            revoked_by,
            revoked_at,
        }
    }
}

/// Event published when project ownership is transferred to another member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectOwnershipTransferredEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub from_user_id: Uuid,
    pub to_user_id: Uuid,
    pub transferred_by: Uuid,
    pub transferred_at: DateTime<Utc>,
    #[serde(default)]
    pub owner_type: Option<String>,
    #[serde(default)]
    pub previous_owner_id: Option<Uuid>,
    #[serde(default)]
    pub new_owner_id: Option<Uuid>,
}

impl ProjectOwnershipTransferredEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        from_user_id: Uuid,
        to_user_id: Uuid,
        transferred_by: Uuid,
        transferred_at: DateTime<Utc>,
        owner_type: Option<String>,
        previous_owner_id: Option<Uuid>,
        new_owner_id: Option<Uuid>,
    ) -> Self {
        Self {
            base: BaseEvent::new("project_ownership_transferred".to_string(), project_id)
                .with_version(EVENT_SCHEMA_V2),
            project_id,
            from_user_id,
            to_user_id,
            transferred_by,
            transferred_at,
            owner_type,
            previous_owner_id,
            new_owner_id,
        }
    }
}
