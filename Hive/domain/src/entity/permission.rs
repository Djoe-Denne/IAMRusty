use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rustycog_permission::PermissionLevel;

use crate::error::DomainError;

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

impl From<String> for PermissionLevel {
    fn from(s: String) -> Self {
        Self::from_str(&s).unwrap()
    }
}
