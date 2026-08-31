use std::str::FromStr;

use chrono::Utc;
use rustycog::core::error::DomainError;
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
            MemberRolePermission::Read => Self::from("read"),
            MemberRolePermission::Write => Self::from("write"),
            MemberRolePermission::Delete => Self::from("delete"),
            MemberRolePermission::Admin => Self::from("admin"),
        }
    }
}

impl FromStr for MemberRolePermission {
    type Err = DomainError;

    fn from_str(permission: &str) -> Result<Self, Self::Err> {
        match permission.to_lowercase().as_str() {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "delete" => Ok(Self::Delete),
            "admin" => Ok(Self::Admin),
            _ => Err(DomainError::invalid_input(&format!(
                "Invalid member role permission: {permission}"
            ))),
        }
    }
}

impl TryFrom<String> for MemberRolePermission {
    type Error = DomainError;

    fn try_from(permission: String) -> Result<Self, Self::Error> {
        permission.parse()
    }
}

impl TryFrom<MemberRolePermission> for PermissionLevel {
    type Error = DomainError;

    fn try_from(permission: MemberRolePermission) -> Result<Self, Self::Error> {
        match permission {
            MemberRolePermission::Read => Ok(Self::Read),
            MemberRolePermission::Write => Ok(Self::Write),
            MemberRolePermission::Admin => Ok(Self::Admin),
            MemberRolePermission::Delete => Err(DomainError::invalid_input(
                "delete is not a Hive permission level",
            )),
        }
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

impl TryFrom<&MemberRole> for RolePermission {
    type Error = DomainError;

    fn try_from(member_role: &MemberRole) -> Result<Self, Self::Error> {
        let level = PermissionLevel::try_from(member_role.permissions)?;
        let permission = Permission::new(level, None, None);
        Ok(Self::new(
            None,
            None,
            member_role.organization_id,
            &permission,
            &member_role.resource.clone().into(),
            Some(Utc::now()),
        ))
    }
}

impl From<RolePermission> for MemberRole {
    fn from(role_permission: RolePermission) -> Self {
        let permissions = match role_permission.permission.level {
            PermissionLevel::Read => MemberRolePermission::Read,
            PermissionLevel::Write => MemberRolePermission::Write,
            PermissionLevel::Admin | PermissionLevel::Owner => MemberRolePermission::Admin,
        };
        Self {
            organization_id: role_permission.organization_id,
            resource: role_permission.resource.name,
            permissions,
        }
    }
}
