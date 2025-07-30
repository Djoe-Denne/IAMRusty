use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;

/// Resource entity representing a specific resource in the system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Resource {
    /// Create a new resource
    pub fn new(
        name: String,
        description: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            name,
            description,
            created_at,
        })
    }
}
