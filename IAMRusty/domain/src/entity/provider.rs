use serde::{Deserialize, Serialize};

/// Supported `OAuth2` Providers
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Provider {
    /// GitHub `OAuth2` provider
    GitHub,
    /// GitLab `OAuth2` provider
    GitLab,
}

impl Provider {
    /// Converts a string to a Provider enum
    #[must_use]
    pub fn from_str(provider: &str) -> Option<Self> {
        match provider.to_lowercase().as_str() {
            "github" => Some(Self::GitHub),
            "gitlab" => Some(Self::GitLab),
            _ => None,
        }
    }

    /// Converts a Provider enum to a string
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::GitHub => "github",
            Self::GitLab => "gitlab",
        }
    }
}

/// Represents `OAuth2` tokens for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTokens {
    /// The access token
    pub access_token: String,

    /// Refresh token, if provided
    pub refresh_token: Option<String>,

    /// Expiration time in seconds from issuance
    pub expires_in: Option<u64>,
}

/// User profile data retrieved from a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUserProfile {
    /// Provider-specific user ID
    pub id: String,

    /// Username from the provider
    pub username: String,

    /// Email address from the provider
    pub email: Option<String>,

    /// URL to the user's avatar
    pub avatar_url: Option<String>,
}
