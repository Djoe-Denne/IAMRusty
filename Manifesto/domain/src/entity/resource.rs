use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Resource entity representing a specific resource in the system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    pub name: String,
    pub created_at: Option<DateTime<Utc>>,
}

impl Resource {
    /// Create a new resource
    pub fn new(name: String, created_at: Option<DateTime<Utc>>) -> Self {
        Self { name, created_at }
    }
}

impl From<String> for Resource {
    fn from(name: String) -> Self {
        Self::new(name, None)
    }
}

impl From<&str> for Resource {
    fn from(name: &str) -> Self {
        Self::new(name.to_string(), None)
    }
}

