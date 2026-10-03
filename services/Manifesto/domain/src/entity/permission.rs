use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::value_objects::PermissionLevel;

/// Permission entity representing a specific permission level
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permission {
    pub level: PermissionLevel,
    pub created_at: Option<DateTime<Utc>>,
}

impl Permission {
    /// Create a new permission
    #[must_use]
    pub const fn new(level: PermissionLevel, created_at: Option<DateTime<Utc>>) -> Self {
        Self { level, created_at }
    }

    /// Create from permission level
    #[must_use]
    pub const fn from_level(level: PermissionLevel) -> Self {
        Self {
            level,
            created_at: None,
        }
    }
}

impl From<PermissionLevel> for Permission {
    fn from(level: PermissionLevel) -> Self {
        Self::from_level(level)
    }
}
