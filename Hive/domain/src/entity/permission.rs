use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;

/// Permission levels available in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionLevel {
    Read,
    Write,
    Admin,
    Owner,
}

impl PermissionLevel {
    /// Get all available permission levels
    pub fn all() -> Vec<PermissionLevel> {
        vec![
            PermissionLevel::Read,
            PermissionLevel::Write,
            PermissionLevel::Admin,
            PermissionLevel::Owner,
        ]
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionLevel::Read => "read",
            PermissionLevel::Write => "write",
            PermissionLevel::Admin => "admin",
            PermissionLevel::Owner => "owner",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "read" => Ok(PermissionLevel::Read),
            "write" => Ok(PermissionLevel::Write),
            "admin" => Ok(PermissionLevel::Admin),
            "owner" => Ok(PermissionLevel::Owner),
            _ => Err(DomainError::invalid_input(&format!(
                "Invalid permission level: {}",
                s
            ))),
        }
    }
}

/// Permission entity representing a specific permission level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Permission {
    pub level: PermissionLevel,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Permission {
    /// Create a new permission
    pub fn new(level: PermissionLevel, description: Option<String>, created_at: DateTime<Utc>) -> Self {
        Self {
            level,
            description,
            created_at,
        }
    }
}

impl std::fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

