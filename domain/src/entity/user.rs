use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Represents a user in the system
/// 
/// Users can link multiple OAuth providers (GitHub, GitLab, etc.) using email as the linking mechanism.
/// Provider-specific information is stored in the provider_tokens table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    /// Unique identifier for the user
    pub id: Uuid,
    
    /// Username (can be updated when linking multiple providers)
    pub username: String,
    
    /// Email address - the primary linking mechanism for multiple providers
    /// This field is required and must be unique across users
    pub email: String,
    
    /// URL to the user's avatar
    pub avatar_url: Option<String>,
    
    /// When the user was created
    pub created_at: DateTime<Utc>,
    
    /// When the user was last updated
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Creates a new user with the given email
    /// 
    /// Email is used as the primary identifier to link multiple OAuth providers
    pub fn new(
        username: String,
        email: String,
        avatar_url: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            username,
            email,
            avatar_url,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Updates user information, typically when linking a new provider
    pub fn update_profile(
        &mut self,
        username: Option<String>,
        avatar_url: Option<String>,
    ) {
        if let Some(username) = username {
            self.username = username;
        }
        if let Some(avatar_url) = avatar_url {
            self.avatar_url = Some(avatar_url);
        }
        self.updated_at = Utc::now();
    }
} 