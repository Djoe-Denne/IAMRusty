use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::{permission::Permission, resource::Resource};

/// Role permission entity representing a named permission-resource combination
/// This acts like a permission group/template scoped to a project
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RolePermission {
    pub id: Option<Uuid>,
    pub name: Option<String>,
    pub project_id: Uuid,
    pub permission: Permission,
    pub resource: Resource,
    pub created_at: Option<DateTime<Utc>>,
}

impl RolePermission {
    /// Create a new role permission
    pub fn new(
        id: Option<Uuid>,
        name: Option<String>,
        project_id: Uuid,
        permission: Permission,
        resource: Resource,
        created_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            name,
            project_id,
            permission,
            resource,
            created_at,
        }
    }

    /// Update role permission name
    pub fn update_name(&mut self, new_name: String) {
        self.name = Some(new_name);
    }
}

/// Helper struct to represent permission-resource combinations for easier handling
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionResourceCombo {
    pub permission_level: String,
    pub resource_name: String,
}

impl PermissionResourceCombo {
    pub fn new(permission_level: String, resource_name: String) -> Self {
        Self {
            permission_level,
            resource_name,
        }
    }
}
