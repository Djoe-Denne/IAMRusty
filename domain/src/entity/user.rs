use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Represents a user in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    /// Unique identifier for the user
    pub id: Uuid,
    
    /// Provider-specific identifier (e.g., "github_12345")
    pub provider_user_id: String,
    
    /// Username from the provider
    pub username: String,
    
    /// Email address from the provider
    pub email: Option<String>,
    
    /// URL to the user's avatar
    pub avatar_url: Option<String>,
    
    /// When the user was created
    pub created_at: DateTime<Utc>,
    
    /// When the user was last updated
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Creates a new user with the given provider and provider user ID
    pub fn new(
        provider_user_id: String,
        username: String,
        email: Option<String>,
        avatar_url: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            provider_user_id,
            username,
            email,
            avatar_url,
            created_at: now,
            updated_at: now,
        }
    }
} 