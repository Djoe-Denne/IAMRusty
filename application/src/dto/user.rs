use domain::entity::user::User;
use serde::{Deserialize, Serialize};

/// User profile DTO for HTTP responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfileDto {
    /// User UUID
    pub id: String,
    
    /// Username
    pub username: String,
    
    /// Email address (now required)
    pub email: String,
    
    /// Avatar URL
    pub avatar: Option<String>,
}

impl From<User> for UserProfileDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id.to_string(),
            username: user.username,
            email: user.email,
            avatar: user.avatar_url,
        }
    }
} 