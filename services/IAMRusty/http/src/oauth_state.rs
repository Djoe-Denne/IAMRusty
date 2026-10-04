//! OAuth state parameter handling for operation context

use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use hmac::{Hmac, Mac};
use iam_configuration::security::OAuthStateSecret;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::OnceLock;
use thiserror::Error;
use uuid::Uuid;

const STATE_TTL_SECS: i64 = 600;

static STATE_SECRET: OnceLock<Vec<u8>> = OnceLock::new();

type HmacSha256 = Hmac<Sha256>;

/// Configure the HMAC secret used to sign OAuth state (not the JWT secret).
///
/// # Errors
/// Returns an error if another app has already installed a different key.
pub fn configure_oauth_state_secret(secret: OAuthStateSecret) -> Result<(), &'static str> {
    let bytes = secret.as_bytes().to_vec();
    match STATE_SECRET.set(bytes) {
        Ok(()) => Ok(()),
        Err(bytes) if STATE_SECRET.get() == Some(&bytes) => Ok(()),
        Err(_) => Err("OAuth state key already configured differently"),
    }
}

fn state_secret() -> Result<&'static [u8], StateError> {
    STATE_SECRET
        .get()
        .map(Vec::as_slice)
        .ok_or(StateError::Unconfigured)
}

/// OAuth operation type
pub use iam_domain::entity::oauth_transaction::OAuthOperation;

/// OAuth state parameter for encoding operation context
#[derive(Clone, Serialize, Deserialize)]
pub struct OAuthState {
    /// The operation being performed
    pub operation: OAuthOperation,
    /// Random nonce for security
    pub nonce: String,
    /// Canonical IdP slug bound at start (CSRF cross-provider check)
    pub provider: String,
    /// Unix expiry timestamp
    #[serde(default)]
    pub exp: i64,
}

impl std::fmt::Debug for OAuthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OAuthState([redacted])")
    }
}

/// State parameter encoding/decoding errors
#[derive(Error)]
pub enum StateError {
    /// No boot-validated key has been installed.
    #[error("OAuth state secret not configured")]
    Unconfigured,
    /// Failed to serialize state
    #[error("Failed to serialize state")]
    SerializationError(#[from] serde_json::Error),

    /// Failed to encode/decode base64
    #[error("Failed to encode/decode base64")]
    Base64Error(#[from] base64::DecodeError),

    /// Invalid state format
    #[error("Invalid state format")]
    InvalidFormat,

    /// State signature is invalid
    #[error("Invalid state signature")]
    InvalidSignature,

    /// State has expired
    #[error("State expired")]
    Expired,
}

impl std::fmt::Debug for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StateError([redacted])")
    }
}

impl OAuthState {
    /// Create a new login state for `provider` (canonical slug).
    #[must_use]
    pub fn new_login(provider: impl Into<String>) -> Self {
        Self {
            operation: OAuthOperation::Login,
            nonce: uuid::Uuid::new_v4().to_string(),
            provider: provider.into(),
            exp: Utc::now().timestamp() + STATE_TTL_SECS,
        }
    }

    /// Create a new link provider state for `provider` (canonical slug).
    #[must_use]
    pub fn new_link(user_id: Uuid, provider: impl Into<String>) -> Self {
        Self {
            operation: OAuthOperation::Link { user_id },
            nonce: uuid::Uuid::new_v4().to_string(),
            provider: provider.into(),
            exp: Utc::now().timestamp() + STATE_TTL_SECS,
        }
    }

    /// Create a separate relink intention, bound to the guarded platform subject.
    #[must_use]
    pub fn new_relink(user_id: Uuid, provider: impl Into<String>) -> Self {
        Self {
            operation: OAuthOperation::Relink { user_id },
            nonce: Uuid::new_v4().to_string(),
            provider: provider.into(),
            exp: Utc::now().timestamp() + STATE_TTL_SECS,
        }
    }

    /// Encode the state to a signed base64 string for use in OAuth flow
    ///
    /// # Errors
    ///
    /// Returns [`StateError`] when JSON serialization or HMAC signing fails.
    pub fn encode(&self) -> Result<String, StateError> {
        let json = serde_json::to_string(self)?;
        let payload = general_purpose::URL_SAFE_NO_PAD.encode(json.as_bytes());
        let mac = sign(&payload)?;
        Ok(format!("{payload}.{mac}"))
    }

    /// Decode a signed state string back to OAuth state
    ///
    /// # Errors
    ///
    /// Returns [`StateError`] when the value is unsigned, malformed, tampered or expired.
    /// Replay protection belongs exclusively to the transaction writer.
    pub fn inspect(encoded: &str) -> Result<Self, StateError> {
        if encoded.len() > 2000 {
            return Err(StateError::InvalidFormat);
        }
        let (payload, mac) = encoded.split_once('.').ok_or(StateError::InvalidFormat)?;
        let signature = general_purpose::URL_SAFE_NO_PAD
            .decode(mac)
            .map_err(|_| StateError::InvalidSignature)?;
        let mut verifier =
            HmacSha256::new_from_slice(state_secret()?).map_err(|_| StateError::InvalidFormat)?;
        verifier.update(payload.as_bytes());
        verifier
            .verify_slice(&signature)
            .map_err(|_| StateError::InvalidSignature)?;
        let json_bytes = general_purpose::URL_SAFE_NO_PAD.decode(payload)?;
        let json = String::from_utf8(json_bytes).map_err(|_| StateError::InvalidFormat)?;
        let state: Self = serde_json::from_str(&json)?;
        if state.exp <= Utc::now().timestamp() {
            return Err(StateError::Expired);
        }
        Ok(state)
    }

    /// Inspect a signed state. Only the persistent transaction writer authorizes a callback.
    ///
    /// # Errors
    ///
    /// Returns [`StateError`] when inspection fails. This is not an anti-replay authority.
    pub fn decode(encoded: &str) -> Result<Self, StateError> {
        Self::inspect(encoded)
    }

    /// Check if this is a login operation
    #[must_use]
    pub const fn is_login(&self) -> bool {
        matches!(self.operation, OAuthOperation::Login)
    }

    /// Check if this is a link operation and return the user ID
    #[must_use]
    pub const fn get_link_user_id(&self) -> Option<Uuid> {
        match &self.operation {
            OAuthOperation::Link { user_id } => Some(*user_id),
            OAuthOperation::Login => None,
            OAuthOperation::Relink { .. } => None,
        }
    }
}

fn sign(payload: &str) -> Result<String, StateError> {
    let mut mac =
        HmacSha256::new_from_slice(state_secret()?).map_err(|_| StateError::InvalidFormat)?;
    mac.update(payload.as_bytes());
    Ok(general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configure_test_key() {
        let config = iam_configuration::security::SecurityConfig {
            mode: iam_configuration::security::SecurityMode::IsolatedTest,
            ..iam_configuration::security::SecurityConfig::default()
        };
        configure_oauth_state_secret(
            config
                .validate_oauth_state_secret("iam-oauth-state-hmac-test")
                .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn installing_a_different_key_never_silently_reuses_the_first() {
        configure_test_key();
        let config = iam_configuration::security::SecurityConfig {
            mode: iam_configuration::security::SecurityMode::IsolatedTest,
            ..iam_configuration::security::SecurityConfig::default()
        };
        assert!(configure_oauth_state_secret(
            config
                .validate_oauth_state_secret("different-isolated-test-key")
                .unwrap()
        )
        .is_err());
    }

    #[test]
    fn test_login_state_roundtrip() {
        configure_test_key();
        let state = OAuthState::new_login("github");
        let encoded = state.encode().unwrap();
        let decoded = OAuthState::decode(&encoded).unwrap();

        assert!(decoded.is_login());
        assert_eq!(decoded.operation, state.operation);
        assert_eq!(decoded.nonce, state.nonce);
        assert_eq!(decoded.provider, "github");
    }

    #[test]
    fn test_link_state_roundtrip() {
        configure_test_key();
        let user_id = Uuid::new_v4();
        let state = OAuthState::new_link(user_id, "gitlab");
        let encoded = state.encode().unwrap();
        let decoded = OAuthState::decode(&encoded).unwrap();

        assert!(!decoded.is_login());
        assert_eq!(decoded.get_link_user_id(), Some(user_id));
        assert_eq!(decoded.operation, state.operation);
        assert_eq!(decoded.nonce, state.nonce);
        assert_eq!(decoded.provider, "gitlab");
    }

    #[test]
    fn unsigned_state_is_rejected() {
        let unsigned = general_purpose::URL_SAFE_NO_PAD.encode(
            serde_json::json!({
                "operation": { "type": "login" },
                "nonce": Uuid::new_v4().to_string(),
                "exp": Utc::now().timestamp() + 60
            })
            .to_string(),
        );
        assert!(OAuthState::decode(&unsigned).is_err());
    }

    #[test]
    fn inspection_is_not_a_process_local_replay_authority() {
        configure_test_key();
        let state = OAuthState::new_login("github");
        let encoded = state.encode().unwrap();
        assert!(OAuthState::decode(&encoded).is_ok());
        assert!(OAuthState::inspect(&encoded).is_ok());
        assert!(OAuthState::decode(&encoded).is_ok());
    }

    #[test]
    fn decode_rejects_missing_provider_field() {
        configure_test_key();
        let json = serde_json::json!({
            "operation": { "type": "login" },
            "nonce": Uuid::new_v4().to_string(),
            "exp": Utc::now().timestamp() + 60
        })
        .to_string();
        let payload = general_purpose::URL_SAFE_NO_PAD.encode(json.as_bytes());
        let mac = sign(&payload).unwrap();
        let encoded = format!("{payload}.{mac}");
        assert!(OAuthState::decode(&encoded).is_err());
    }
}
