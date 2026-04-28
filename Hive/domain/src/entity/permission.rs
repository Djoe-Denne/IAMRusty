use chrono::{DateTime, Utc};
use rustycog_core::error::DomainError;
use serde::{Deserialize, Serialize};

/// Permission entity representing a specific permission level
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionLevel {
    Read,
    Write,
    Admin,
    Owner,
}

impl PermissionLevel {
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "admin" => Ok(Self::Admin),
            "owner" => Ok(Self::Owner),
            _ => Err(DomainError::InvalidInput {
                message: format!("Invalid permission level: {s}"),
            }),
        }
    }

    #[must_use]
    pub const fn to_str(&self) -> &str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Admin => "admin",
            Self::Owner => "owner",
        }
    }
}

impl From<rustycog_permission::Permission> for PermissionLevel {
    fn from(permission: rustycog_permission::Permission) -> Self {
        match permission {
            rustycog_permission::Permission::Read => Self::Read,
            rustycog_permission::Permission::Write => Self::Write,
            rustycog_permission::Permission::Admin => Self::Admin,
            rustycog_permission::Permission::Owner => Self::Owner,
        }
    }
}

impl From<PermissionLevel> for rustycog_permission::Permission {
    fn from(permission: PermissionLevel) -> Self {
        match permission {
            PermissionLevel::Read => Self::Read,
            PermissionLevel::Write => Self::Write,
            PermissionLevel::Admin => Self::Admin,
            PermissionLevel::Owner => Self::Owner,
        }
    }
}

/// Permission entity representing a specific permission level
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permission {
    pub level: PermissionLevel,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

impl Permission {
    /// Create a new permission
    #[must_use]
    pub const fn new(
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
