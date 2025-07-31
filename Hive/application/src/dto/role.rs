use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use hive_domain::entity::{Permission, PermissionLevel, Resource, RolePermission};

// =============================================================================
// Role Request DTOs
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MemberRolePermission {
    Read,
    Write,
    Delete,
    Admin,
}

impl From<MemberRolePermission> for &str {

    fn from(permission: MemberRolePermission) -> Self {
        match permission {
            MemberRolePermission::Read => "read",
            MemberRolePermission::Write => "write",
            MemberRolePermission::Delete => "delete",
            MemberRolePermission::Admin => "admin",
        }
    }
}

impl From<String> for MemberRolePermission {
    fn from(permission: String) -> Self {
        match permission.to_lowercase().as_str() {
            "read" => MemberRolePermission::Read,
            "write" => MemberRolePermission::Write,
            "delete" => MemberRolePermission::Delete,
            "admin" => MemberRolePermission::Admin,
            _ => panic!("Invalid member role permission"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MemberRole {
    pub resource: String,
    pub permissions: MemberRolePermission,
}

/// DTO for creating a new role
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateMemberRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub roles: Vec<MemberRole>,
}

/// DTO for updating a role
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateMemberRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub roles: Option<Vec<MemberRole>>,
}

impl From<&MemberRole> for RolePermission {
    fn from(member_role: &MemberRole) -> Self {
        RolePermission::new(None, None, None, &member_role.permissions.into(), &member_role.resource.clone().into(), None)
    }
}

impl From<RolePermission> for MemberRole {
    fn from(role_permission: RolePermission) -> Self {
        MemberRole {
            resource: role_permission.resource.name,
            permissions: role_permission.permission.level.to_string().into(),
        }
    }
}

impl From<MemberRolePermission> for Permission {
    fn from(permission: MemberRolePermission) -> Self {
        Permission::new(PermissionLevel::from_str(permission.into()).unwrap(), None, Utc::now())
    }
}