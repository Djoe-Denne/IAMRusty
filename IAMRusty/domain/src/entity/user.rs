use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Represents a user in the system
/// 
/// Users can link multiple OAuth providers and have multiple email addresses.
/// Email addresses are stored separately in the UserEmail entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    /// Unique identifier for the user
    pub id: Uuid,
    
    /// Username (can be updated when linking multiple providers)
    pub username: String,
    
    /// URL to the user's avatar
    pub avatar_url: Option<String>,
    
    /// Password hash (for email/password authentication)
    pub password_hash: Option<String>,
    
    /// When the user was created
    pub created_at: DateTime<Utc>,
    
    /// When the user was last updated
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Creates a new user
    pub fn new(
        username: String,
        avatar_url: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            username,
            avatar_url,
            password_hash: None,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Creates a new user with password authentication
    pub fn new_with_password(
        username: String,
        password_hash: String,
        avatar_url: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            username,
            avatar_url,
            password_hash: Some(password_hash),
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
    
    /// Updates the user's password hash
    pub fn update_password(&mut self, password_hash: String) {
        self.password_hash = Some(password_hash);
        self.updated_at = Utc::now();
    }
    
    /// Checks if the user has password authentication enabled
    pub fn has_password(&self) -> bool {
        self.password_hash.is_some()
    }
} 