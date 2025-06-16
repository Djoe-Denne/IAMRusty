use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Registration flow type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegistrationFlow {
    /// Email/password registration flow
    #[serde(rename = "email_password")]
    EmailPassword,
    /// OAuth registration flow
    #[serde(rename = "oauth")]
    OAuth,
}

/// Claims for registration JWT tokens
/// These tokens are RSA-signed and contain minimal information for completing registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationTokenClaims {
    /// Subject type - always "registration" for these tokens
    pub sub: String,

    /// User ID that this token is for
    pub user_id: String,

    /// User's email address
    pub email: String,

    /// Registration flow type
    pub flow: RegistrationFlow,

    /// Provider information (only for OAuth flows)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_info: Option<ProviderInfo>,

    /// JWT expiration timestamp
    pub exp: i64,

    /// JWT issued at timestamp
    pub iat: i64,

    /// JWT token ID (for revocation)
    pub jti: String,
}

/// Provider information for OAuth registration tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider email
    pub email: String,
    /// Provider avatar URL  
    pub avatar: Option<String>,
    /// Suggested username based on provider data
    pub suggested_username: String,
}

impl RegistrationTokenClaims {
    /// Creates new registration token claims for email/password flow
    pub fn new(user_id: Uuid, email: String) -> Self {
        Self::new_with_flow(user_id, email, RegistrationFlow::EmailPassword)
    }

    /// Creates new registration token claims for OAuth flow
    pub fn new_oauth(user_id: Uuid, email: String) -> Self {
        Self::new_with_flow(user_id, email, RegistrationFlow::OAuth)
    }

    /// Creates new registration token claims with specified flow type
    pub fn new_with_flow(user_id: Uuid, email: String, flow: RegistrationFlow) -> Self {
        let now = Utc::now();
        let expires_in = Duration::hours(24); // 24 hours as recommended

        Self {
            sub: "registration".to_string(),
            user_id: user_id.to_string(),
            email,
            flow,
            exp: (now + expires_in).timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            provider_info: None,
        }
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    /// Get the user ID as UUID
    pub fn get_user_id(&self) -> Result<Uuid, uuid::Error> {
        Uuid::parse_str(&self.user_id)
    }

    /// Validate that this is a registration token
    pub fn is_registration_token(&self) -> bool {
        self.sub == "registration"
    }

    /// Check if this is an OAuth flow
    pub fn is_oauth_flow(&self) -> bool {
        self.flow == RegistrationFlow::OAuth
    }

    /// Check if this is an email/password flow
    pub fn is_email_password_flow(&self) -> bool {
        self.flow == RegistrationFlow::EmailPassword
    }
}
