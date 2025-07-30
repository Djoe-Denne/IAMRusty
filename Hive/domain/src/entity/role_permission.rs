use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::DomainError, entity::{permission::Permission, resource::Resource}};

/// Role permission entity representing a named permission-resource combination
/// This acts like a permission group/template that can be assigned to users
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RolePermission { 
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permission: Permission,
    pub resource: Resource,
    pub created_at: DateTime<Utc>,
}

impl RolePermission {
    /// Create a new role permission
    pub fn new(
        id: Uuid,
        name: String,
        description: Option<String>,
        permission: &Permission,
        resource: &Resource,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            description,
            permission: permission.clone(),
            resource: resource.clone(),
            created_at,
        }
    }

    /// Update role permission name
    pub fn update_name(&mut self, new_name: String) {
        self.name = new_name;
    }

    /// Update role permission description
    pub fn update_description(&mut self, new_description: Option<String>) {
        self.description = new_description;
    }
}

/// Helper struct to represent permission-resource combinations for easier handling
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionResourceCombo {
    pub permission_level: String,
    pub resource_type: String,
}

impl PermissionResourceCombo {
    pub fn new(permission_level: String, resource_type: String) -> Self {
        Self {
            permission_level,
            resource_type,
        }
    }
}
