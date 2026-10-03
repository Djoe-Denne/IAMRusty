use iam_domain::entity::provider::ProviderTokens;
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

/// Request for completing user registration with username
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRegistrationRequest {
    /// RSA-signed JWT registration token
    pub registration_token: String,
    /// Chosen username
    pub username: String,
}

/// Response for completed registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRegistrationResponse {
    /// User profile
    pub user: UserDto,
    /// Access token
    pub access_token: String,
    /// Token expiration in seconds
    pub expires_in: u64,
    /// Refresh token
    pub refresh_token: String,
}

/// User DTO for responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDto {
    /// User ID
    pub id: String,
    /// Username
    pub username: String,
    /// Email address
    pub email: String,
    /// Avatar URL
    pub avatar: Option<String>,
}

/// Request for checking username availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckUsernameRequest {
    /// Username to check
    pub username: String,
}

/// Response for username availability check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckUsernameResponse {
    /// Whether the username is available
    pub available: bool,
    /// Suggested alternatives if username is taken
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<String>>,
}

/// Updated signup response with registration token for incomplete registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignupResponse {
    /// Response type
    #[serde(flatten)]
    pub variant: SignupResponseVariant,
}

/// Signup response variants for different scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum SignupResponseVariant {
    /// Existing user - password auth added (200)
    #[serde(rename = "existing_user")]
    ExistingUser {
        user: UserDto,
        access_token: String,
        expires_in: u64,
        refresh_token: String,
        message: String,
    },
    /// New user created - username required (201)
    #[serde(rename = "registration_required")]
    RegistrationRequired {
        user: IncompleteUserDto,
        registration_token: String,
        requires_username: bool,
        message: String,
    },
}

/// User DTO for incomplete registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompleteUserDto {
    /// User ID
    pub id: String,
    /// Email address
    pub email: String,
}

/// OAuth callback response variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum OAuthCallbackResponse {
    /// Existing user login (200)
    #[serde(rename = "login")]
    Login {
        user: UserDto,
        access_token: String,
        expires_in: u64,
        refresh_token: String,
    },
    /// Provider linking (200)
    #[serde(rename = "link")]
    Link {
        message: String,
        user: UserDto,
        emails: Vec<String>,
        new_email_added: bool,
    },
    /// New user - username selection required (202)
    #[serde(rename = "registration_required")]
    RegistrationRequired {
        registration_token: String,
        provider_info: ProviderInfoDto,
        requires_username: bool,
    },
}

/// Provider information for OAuth registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfoDto {
    /// Email from provider
    pub email: String,
    /// Avatar URL from provider
    pub avatar: Option<String>,
    /// Suggested username based on provider data
    pub suggested_username: String,
}

/// Login response with support for incomplete registration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum LoginResponse {
    /// Successful login (200)
    #[serde(rename = "success")]
    Success {
        user: UserDto,
        access_token: String,
        expires_in: u64,
        refresh_token: String,
    },
    /// Account exists but registration incomplete (423)
    #[serde(rename = "registration_incomplete")]
    RegistrationIncomplete {
        error: String,
        message: String,
        registration_token: String,
    },
}
