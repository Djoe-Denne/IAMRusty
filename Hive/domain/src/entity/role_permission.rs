use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;

/// Role permission entity representing a named permission-resource combination
/// This acts like a permission group/template that can be assigned to users
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RolePermission {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permission_id: Uuid,
    pub resource_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl RolePermission {
    /// Create a new role permission
    pub fn new(
        name: String,
        description: Option<String>,
        permission_id: Uuid,
        resource_id: Uuid,
    ) -> Result<Self, DomainError> {
        Self::validate_name(&name)?;
        
        Ok(Self {
            id: Uuid::new_v4(),
            name,
            description,
            permission_id,
            resource_id,
            created_at: Utc::now(),
        })
    }
    
    /// Update role permission name
    pub fn update_name(&mut self, new_name: String) -> Result<(), DomainError> {
        Self::validate_name(&new_name)?;
        self.name = new_name;
        Ok(())
    }
    
    /// Update role permission description
    pub fn update_description(&mut self, new_description: Option<String>) {
        self.description = new_description;
    }
    
    /// Validate role permission name
    fn validate_name(name: &str) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::invalid_input("Role permission name cannot be empty"));
        }
        
        if name.len() > 100 {
            return Err(DomainError::invalid_input(
                "Role permission name cannot be longer than 100 characters",
            ));
        }
        
        Ok(())
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