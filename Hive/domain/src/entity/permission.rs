use chrono::{DateTime, Utc};
use rustycog_core::error::DomainError;
use serde::{Deserialize, Serialize};

/// Permission entity representing a specific permission level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PermissionLevel {
    Read,
    Write,
    Admin,
    Owner,
}

impl PermissionLevel {
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s {
            "read" => Ok(PermissionLevel::Read),
            "write" => Ok(PermissionLevel::Write),
            "admin" => Ok(PermissionLevel::Admin),
            "owner" => Ok(PermissionLevel::Owner),
            _ => Err(DomainError::InvalidInput {
                message: format!("Invalid permission level: {}", s),
            }),
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            PermissionLevel::Read => "read",
            PermissionLevel::Write => "write",
            PermissionLevel::Admin => "admin",
            PermissionLevel::Owner => "owner",
        }
    }
}

impl From<rustycog_permission::Permission> for PermissionLevel {
    fn from(permission: rustycog_permission::Permission) -> Self {
        match permission {
            rustycog_permission::Permission::Read => PermissionLevel::Read,
            rustycog_permission::Permission::Write => PermissionLevel::Write,
            rustycog_permission::Permission::Admin => PermissionLevel::Admin,
            rustycog_permission::Permission::Owner => PermissionLevel::Owner,
        }
    }
}

impl From<PermissionLevel> for rustycog_permission::Permission {
    fn from(permission: PermissionLevel) -> Self {
        match permission {
            PermissionLevel::Read => rustycog_permission::Permission::Read,
            PermissionLevel::Write => rustycog_permission::Permission::Write,
            PermissionLevel::Admin => rustycog_permission::Permission::Admin,
            PermissionLevel::Owner => rustycog_permission::Permission::Owner,
        }
    }
}

/// Permission entity representing a specific permission level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Permission {
    pub level: PermissionLevel,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

impl Permission {
    /// Create a new permission
    pub fn new(
        level: PermissionLevel,
        description: Option<String>,
        created_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            level,
            description,
            created_at,
        }
    }
}
