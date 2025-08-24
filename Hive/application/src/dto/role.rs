use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use hive_domain::entity::{Permission, PermissionLevel, RolePermission};

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
impl From<MemberRolePermission> for String {

    fn from(permission: MemberRolePermission) -> Self {
        match permission {
            MemberRolePermission::Read => String::from("read"),
            MemberRolePermission::Write => String::from("write"),
            MemberRolePermission::Delete => String::from("delete"),
            MemberRolePermission::Admin => String::from("admin"),
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

impl From<MemberRolePermission> for PermissionLevel {
    fn from(permission: MemberRolePermission) -> Self {
        PermissionLevel::from_str(permission.into()).unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MemberRole {
    pub organization_id: Uuid,
    pub resource: String,
    pub permissions: MemberRolePermission,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MemberRoleListResponse {
    pub roles: Vec<MemberRole>,
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
        let permission = Permission::new(member_role.permissions.into(), None, None);
        RolePermission::new(None, None, member_role.organization_id, &permission, &member_role.resource.clone().into(), Some(Utc::now()))
    }
}

impl From<RolePermission> for MemberRole {
    fn from(role_permission: RolePermission) -> Self {
        MemberRole {
            organization_id: role_permission.organization_id,
            resource: role_permission.resource.name,
            permissions: role_permission.permission.level.to_str().to_string().into(),
        }
    }
}
