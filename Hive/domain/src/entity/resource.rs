use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Resource entity representing a specific resource in the system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    pub name: String,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

impl Resource {
    /// Create a new resource
    pub fn new(
        name: String,
        description: Option<String>,
        created_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            name,
            description,
            created_at,
        }
    }
}

impl From<String> for Resource {
    fn from(name: String) -> Self {
        Self::new(name, None, None)
    }
}
