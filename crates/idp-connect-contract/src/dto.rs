//! JSON DTOs and `/v1/*` path constants shared by IAM and `IdP` Connect.

use serde::{Deserialize, Serialize};

/// Relative authorize path (no service prefix).
pub const AUTHORIZE_PATH: &str = "/v1/authorize";
/// Relative token path (no service prefix).
pub const TOKEN_PATH: &str = "/v1/token";
/// Relative profile path (no service prefix).
pub const PROFILE_PATH: &str = "/v1/profile";

/// `OAuth2` tokens returned by a federated authenticator.
#[derive(Clone, Serialize, Deserialize)]
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
#[derive(Clone, Serialize, Deserialize)]
pub struct AuthorizeRequest {
    /// Callback URI registered for this OAuth start.
    pub redirect_uri: String,
    /// Opaque CSRF/state value chosen by IAM (not interpreted here).
    pub state: String,
    /// S256 challenge, paired with `code_challenge_method`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_challenge: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_challenge_method: Option<String>,
}

/// JSON body returned by `POST /v1/authorize`.
#[derive(Clone, Serialize, Deserialize)]
pub struct AuthorizeResponse {
    /// Vendor authorization URL to redirect the browser to.
    pub authorization_url: String,
    /// Space-delimited OAuth scope string advertised by the connector.
    pub scope: String,
}

/// JSON body for `POST /v1/token`.
#[derive(Clone, Serialize, Deserialize)]
pub struct TokenRequest {
    /// Authorization code from the vendor callback.
    pub code: String,
    /// Same redirect URI used at authorize time.
    pub redirect_uri: String,
    /// Secret verifier forwarded only after IAM's transaction writer has consumed state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_verifier: Option<String>,
}

/// JSON body for `POST /v1/profile`.
#[derive(Clone, Serialize, Deserialize)]
pub struct ProfileRequest {
    /// Access token previously obtained from `/v1/token`.
    pub access_token: String,
}

impl AuthorizeRequest {
    /// Validate the wire pair without reflecting state or challenge in errors.
    ///
    /// # Errors
    /// Rejects an incomplete pair, non-S256 method or malformed challenge.
    pub fn pkce_challenge(&self) -> Result<Option<&str>, crate::error::FederatedOAuthError> {
        match (
            self.code_challenge.as_deref(),
            self.code_challenge_method.as_deref(),
        ) {
            (None, None) => Ok(None),
            (Some(challenge), Some("S256")) if valid_s256_challenge(challenge) => {
                Ok(Some(challenge))
            }
            _ => Err(crate::error::FederatedOAuthError::Authorize),
        }
    }
}

/// SHA-256 base64url without padding is exactly 43 URL-safe characters.
#[must_use]
pub fn valid_s256_challenge(challenge: &str) -> bool {
    challenge.len() == 43
        && challenge
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// RFC 7636 unreserved verifier alphabet, length 43..=128.
#[must_use]
pub fn valid_pkce_verifier(verifier: &str) -> bool {
    (43..=128).contains(&verifier.len())
        && verifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'))
}

macro_rules! redacted_debug {
    ($($name:ident),+ $(,)?) => { $(
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($name), "([redacted])"))
            }
        }
    )+ };
}
redacted_debug!(
    ProviderTokens,
    AuthorizeRequest,
    AuthorizeResponse,
    TokenRequest,
    ProfileRequest
);

#[cfg(test)]
mod pkce_tests {
    use super::*;

    #[test]
    fn authorize_requires_complete_s256_pair() {
        let mut req = AuthorizeRequest {
            redirect_uri: "https://app/callback".into(),
            state: "opaque".into(),
            code_challenge: None,
            code_challenge_method: None,
        };
        assert_eq!(req.pkce_challenge().unwrap(), None);
        req.code_challenge = Some("a".repeat(43));
        assert!(req.pkce_challenge().is_err());
        req.code_challenge_method = Some("plain".into());
        assert!(req.pkce_challenge().is_err());
        req.code_challenge_method = Some("S256".into());
        assert!(req.pkce_challenge().unwrap().is_some());
        req.code_challenge = None;
        assert!(req.pkce_challenge().is_err());
    }

    #[test]
    fn verifier_and_sensitive_debug_are_bounded_and_redacted() {
        assert!(valid_pkce_verifier(&"a".repeat(43)));
        assert!(!valid_pkce_verifier(&"a".repeat(42)));
        assert!(!valid_pkce_verifier(&"a".repeat(129)));
        assert!(!valid_pkce_verifier(&format!("{}+", "a".repeat(43))));
        let req = TokenRequest {
            code: "SENTINEL-CODE".into(),
            redirect_uri: "https://app/cb".into(),
            code_verifier: Some("SENTINEL-VERIFIER".into()),
        };
        assert!(!format!("{req:?}").contains("SENTINEL"));
    }
}
