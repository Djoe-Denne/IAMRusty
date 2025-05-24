use domain::entity::user::User;
use serde::{Deserialize, Serialize};

/// User profile DTO for HTTP responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfileDto {
    /// User UUID
    pub id: String,
    
    /// Username
    pub username: String,
    
    /// Email address (populated separately from UserEmail entity)
    pub email: String,
    
    /// Avatar URL
    pub avatar: Option<String>,
}

impl UserProfileDto {
    /// Create a UserProfileDto from a User and email
    pub fn from_user_and_email(user: User, email: String) -> Self {
        Self {
            id: user.id.to_string(),
            username: user.username,
            email,
            avatar: user.avatar_url,
        }
    }
}

impl From<User> for UserProfileDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id.to_string(),
            username: user.username,
            email: "".to_string(), // Placeholder - legacy auth service compatibility
            avatar: user.avatar_url,
        }
    }
} 