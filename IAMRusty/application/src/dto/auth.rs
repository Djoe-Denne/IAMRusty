use domain::entity::provider::ProviderTokens;
use serde::{Deserialize, Serialize};

use super::UserProfileDto;

/// Authentication response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponseDto {
    /// JWT token
    pub token: String,
    
    /// User profile
    pub user: UserProfileDto,
}

/// Provider token response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTokenResponseDto {
    /// Access token
    pub access_token: String,
    
    /// Token expiration in seconds
    pub expires_in: Option<u64>,
}

impl From<ProviderTokens> for ProviderTokenResponseDto {
    fn from(tokens: ProviderTokens) -> Self {
        Self {
            access_token: tokens.access_token,
            expires_in: tokens.expires_in,
        }
    }
} 