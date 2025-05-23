use domain::entity::user::User;
use serde::{Deserialize, Serialize};

/// User profile DTO for HTTP responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfileDto {
    /// User ID in provider:id format (e.g., "github_12345")
    pub id: String,
    
    /// Username
    pub username: String,
    
    /// Email address
    pub email: Option<String>,
    
    /// Avatar URL
    pub avatar: Option<String>,
}

impl From<User> for UserProfileDto {
    fn from(user: User) -> Self {
        Self {
            id: user.provider_user_id,
            username: user.username,
            email: user.email,
            avatar: user.avatar_url,
        }
    }
} 