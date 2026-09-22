//! JSON DTOs and `/v1/*` path constants shared by IAM and `IdP` Connect.

use serde::{Deserialize, Serialize};

/// Relative authorize path (no service prefix).
pub const AUTHORIZE_PATH: &str = "/v1/authorize";
/// Relative token path (no service prefix).
pub const TOKEN_PATH: &str = "/v1/token";
/// Relative profile path (no service prefix).
pub const PROFILE_PATH: &str = "/v1/profile";

/// `OAuth2` tokens returned by a federated authenticator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTokens {
    /// The access token
    pub access_token: String,

    /// Refresh token, if provided
    pub refresh_token: Option<String>,

    /// Expiration time in seconds from issuance
    pub expires_in: Option<u64>,
}

/// User profile data retrieved from a federated authenticator.
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

    /// Whether the provider asserts this email is verified
    #[serde(default)]
    pub email_verified: bool,
}

/// JSON body for `POST /v1/authorize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizeRequest {
    /// Callback URI registered for this OAuth start.
    pub redirect_uri: String,
    /// Opaque CSRF/state value chosen by IAM (not interpreted here).
    pub state: String,
}

/// JSON body returned by `POST /v1/authorize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizeResponse {
    /// Vendor authorization URL to redirect the browser to.
    pub authorization_url: String,
    /// Space-delimited OAuth scope string advertised by the connector.
    pub scope: String,
}

/// JSON body for `POST /v1/token`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRequest {
    /// Authorization code from the vendor callback.
    pub code: String,
    /// Same redirect URI used at authorize time.
    pub redirect_uri: String,
}

/// JSON body for `POST /v1/profile`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileRequest {
    /// Access token previously obtained from `/v1/token`.
    pub access_token: String,
}
