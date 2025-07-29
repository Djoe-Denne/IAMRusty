use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;

/// Permission constants for organization roles
pub mod permissions {
    // Organization permissions
    pub const ADMIN_ORG: &str = "admin:org";
    pub const READ_ORG: &str = "read:org";
    pub const WRITE_ORG: &str = "write:org";
    pub const DELETE_ORG: &str = "delete:org";
    
    // Member permissions
    pub const ADMIN_MEMBERS: &str = "admin:members";
    pub const READ_MEMBERS: &str = "read:members";
    pub const WRITE_MEMBERS: &str = "write:members";
    
    // Role permissions
    pub const ADMIN_ROLES: &str = "admin:roles";
    pub const READ_ROLES: &str = "read:roles";
    pub const WRITE_ROLES: &str = "write:roles";
    
    // External permissions
    pub const ADMIN_EXTERNAL: &str = "admin:external";
    
    // Issue permissions (for custom roles)
    pub const READ_ISSUES: &str = "read:issues";
    pub const WRITE_ISSUES: &str = "write:issues";
    pub const DELETE_ISSUES: &str = "delete:issues";
    
    /// Get all valid permissions
    pub fn all_permissions() -> Vec<&'static str> {
        vec![
            ADMIN_ORG,
            READ_ORG,
            WRITE_ORG,
            DELETE_ORG,
            ADMIN_MEMBERS,
            READ_MEMBERS,
            WRITE_MEMBERS,
            ADMIN_ROLES,
            READ_ROLES,
            WRITE_ROLES,
            ADMIN_EXTERNAL,
            READ_ISSUES,
            WRITE_ISSUES,
            DELETE_ISSUES,
        ]
    }
    
    /// Check if a permission is valid
    pub fn is_valid_permission(permission: &str) -> bool {
        all_permissions().contains(&permission)
    }
}

/// Organization role entity defining permissions within an organization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationRole {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_system_default: bool,
    pub created_at: DateTime<Utc>,
}

/// Common system role types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SystemRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl OrganizationRole {
    /// Create a new custom role
    pub fn new(
        organization_id: Uuid,
        name: String,
        description: Option<String>,
        permissions: Vec<String>,
    ) -> Result<Self, DomainError> {
        Self::validate_name(&name)?;
        Self::validate_permissions(&permissions)?;

        Ok(Self {
            id: Uuid::new_v4(),
            organization_id,
            name,
            description,
            permissions,
            is_system_default: false,
            created_at: Utc::now(),
        })
    }

    /// Create a system default role
    pub fn new_system_role(
        organization_id: Uuid,
        system_role: SystemRole,
    ) -> Self {
        let (name, permission_list) = match system_role {
            SystemRole::Owner => (
                "Owner".to_string(),
                vec![
                    permissions::ADMIN_ORG,
                    permissions::READ_ORG,
                    permissions::WRITE_ORG,
                    permissions::DELETE_ORG,
                    permissions::ADMIN_MEMBERS,
                    permissions::READ_MEMBERS,
                    permissions::WRITE_MEMBERS,
                    permissions::ADMIN_ROLES,
                    permissions::READ_ROLES,
                    permissions::WRITE_ROLES,
                    permissions::ADMIN_EXTERNAL,
                ],
            ),
            SystemRole::Admin => (
                "Admin".to_string(),
                vec![
                    permissions::READ_ORG,
                    permissions::WRITE_ORG,
                    permissions::ADMIN_MEMBERS,
                    permissions::READ_MEMBERS,
                    permissions::WRITE_MEMBERS,
                    permissions::ADMIN_ROLES,
                    permissions::READ_ROLES,
                    permissions::WRITE_ROLES,
                    permissions::ADMIN_EXTERNAL,
                ],
            ),
            SystemRole::Member => (
                "Member".to_string(),
                vec![
                    permissions::READ_ORG,
                    permissions::READ_MEMBERS,
                    permissions::READ_ROLES,
                ],
            ),
            SystemRole::Viewer => (
                "Viewer".to_string(),
                vec![
                    permissions::READ_ORG,
                ],
            ),
        };

        Self {
            id: Uuid::new_v4(),
            organization_id,
            name,
            description: Some(format!("System default {} role", system_role.as_str())),
            permissions: permission_list.into_iter().map(|p| p.to_string()).collect(),
            is_system_default: true,
            created_at: Utc::now(),
        }
    }

    /// Update role name (only for custom roles)
    pub fn update_name(&mut self, new_name: String) -> Result<(), DomainError> {
        if self.is_system_default {
            return Err(DomainError::business_rule_violation(
                "Cannot update name of system default role",
            ));
        }

        Self::validate_name(&new_name)?;
        self.name = new_name;
        Ok(())
    }

    /// Update role description
    pub fn update_description(&mut self, new_description: Option<String>) -> Result<(), DomainError> {
        if self.is_system_default {
            return Err(DomainError::business_rule_violation(
                "Cannot update description of system default role",
            ));
        }

        self.description = new_description;
        Ok(())
    }

    /// Update role permissions (only for custom roles)
    pub fn update_permissions(&mut self, new_permissions: Vec<String>) -> Result<(), DomainError> {
        if self.is_system_default {
            return Err(DomainError::business_rule_violation(
                "Cannot update permissions of system default role",
            ));
        }

        Self::validate_permissions(&new_permissions)?;
        self.permissions = new_permissions;
        Ok(())
    }

    /// Check if role has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    /// Check if role has admin permissions for the organization
    pub fn is_admin(&self) -> bool {
        self.has_permission(permissions::ADMIN_ORG) || self.has_permission(permissions::ADMIN_MEMBERS)
    }

    /// Validate role name
    fn validate_name(name: &str) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::invalid_input("Role name cannot be empty"));
        }

        if name.len() > 100 {
            return Err(DomainError::invalid_input(
                "Role name cannot be longer than 100 characters",
            ));
        }

        Ok(())
    }

    /// Validate permissions
    fn validate_permissions(permissions: &[String]) -> Result<(), DomainError> {
        if permissions.is_empty() {
            return Err(DomainError::invalid_input(
                "Role must have at least one permission",
            ));
        }

        // Validate permission format (should be action:resource)
        for permission in permissions {
            if !permission.contains(':') {
                return Err(DomainError::invalid_input(
                    "Permission must be in format 'action:resource'",
                ));
            }
            
            // Validate against known permissions
            if !permissions::is_valid_permission(permission) {
                return Err(DomainError::invalid_input(
                    &format!("Unknown permission: {}", permission),
                ));
            }
        }

        Ok(())
    }
}

impl SystemRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SystemRole::Owner => "Owner",
            SystemRole::Admin => "Admin",
            SystemRole::Member => "Member",
            SystemRole::Viewer => "Viewer",
        }
    }
}
