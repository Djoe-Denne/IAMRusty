//! OAuth state parameter handling for operation context

use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};
use thiserror::Error;
use uuid::Uuid;

const STATE_TTL_SECS: i64 = 600;

static STATE_SECRET: OnceLock<Vec<u8>> = OnceLock::new();
static USED_NONCES: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();

type HmacSha256 = Hmac<Sha256>;

/// Configure the HMAC secret used to sign OAuth state (not the JWT secret).
pub fn configure_oauth_state_secret(secret: impl Into<String>) {
    let bytes = secret.into().into_bytes();
    let _ = STATE_SECRET.set(bytes);
}

fn state_secret() -> &'static [u8] {
    STATE_SECRET
        .get()
        .map_or(b"iam-oauth-state-hmac-change-me", Vec::as_slice)
}

fn used_nonces() -> &'static Mutex<HashMap<String, i64>> {
    USED_NONCES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// OAuth operation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum OAuthOperation {
    /// Login operation (create new user or authenticate existing)
    #[serde(rename = "login")]
    Login,
    /// Link provider operation (link to existing authenticated user)
    #[serde(rename = "link")]
    Link { user_id: Uuid },
}

/// OAuth state parameter for encoding operation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthState {
    /// The operation being performed
    pub operation: OAuthOperation,
    /// Random nonce for security
    pub nonce: String,
    /// Unix expiry timestamp
    #[serde(default)]
    pub exp: i64,
}

/// State parameter encoding/decoding errors
#[derive(Debug, Error)]
pub enum StateError {
    /// Failed to serialize state
    #[error("Failed to serialize state: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Failed to encode/decode base64
    #[error("Failed to encode/decode base64: {0}")]
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

    /// State nonce was already used
    #[error("State already used")]
    Replay,
}

impl OAuthState {
    /// Create a new login state
    #[must_use]
    pub fn new_login() -> Self {
        Self {
            operation: OAuthOperation::Login,
            nonce: uuid::Uuid::new_v4().to_string(),
            exp: Utc::now().timestamp() + STATE_TTL_SECS,
        }
    }

    /// Create a new link provider state
    #[must_use]
    pub fn new_link(user_id: Uuid) -> Self {
        Self {
            operation: OAuthOperation::Link { user_id },
            nonce: uuid::Uuid::new_v4().to_string(),
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
    /// Returns [`StateError`] when the value is unsigned, tampered, expired, or replayed.
    pub fn inspect(encoded: &str) -> Result<Self, StateError> {
        let (payload, mac) = encoded.split_once('.').ok_or(StateError::InvalidFormat)?;
        let expected = sign(payload)?;
        if !constant_time_eq(mac.as_bytes(), expected.as_bytes()) {
            return Err(StateError::InvalidSignature);
        }
        let json_bytes = general_purpose::URL_SAFE_NO_PAD.decode(payload)?;
        let json = String::from_utf8(json_bytes).map_err(|_| StateError::InvalidFormat)?;
        let state: Self = serde_json::from_str(&json)?;
        if state.exp == 0 || state.exp < Utc::now().timestamp() {
            return Err(StateError::Expired);
        }
        Ok(state)
    }

    /// Decode a signed state string and consume its nonce.
    ///
    /// # Errors
    ///
    /// Returns [`StateError`] when inspection fails or the nonce was already consumed.
    pub fn decode(encoded: &str) -> Result<Self, StateError> {
        let state = Self::inspect(encoded)?;
        consume_nonce(&state.nonce, state.exp)?;
        Ok(state)
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
        }
    }
}

fn sign(payload: &str) -> Result<String, StateError> {
    let mut mac =
        HmacSha256::new_from_slice(state_secret()).map_err(|_| StateError::InvalidFormat)?;
    mac.update(payload.as_bytes());
    Ok(general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn consume_nonce(nonce: &str, exp: i64) -> Result<(), StateError> {
    let now = Utc::now().timestamp();
    let mut store = used_nonces().lock().unwrap_or_else(PoisonError::into_inner);
    store.retain(|_, until| *until > now);
    if store.contains_key(nonce) {
        return Err(StateError::Replay);
    }
    store.insert(nonce.to_string(), exp);
    drop(store);
    Ok(())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_state_roundtrip() {
        let state = OAuthState::new_login();
        let encoded = state.encode().unwrap();
        let decoded = OAuthState::decode(&encoded).unwrap();

        assert!(decoded.is_login());
        assert_eq!(decoded.operation, state.operation);
        assert_eq!(decoded.nonce, state.nonce);
    }

    #[test]
    fn test_link_state_roundtrip() {
        let user_id = Uuid::new_v4();
        let state = OAuthState::new_link(user_id);
        let encoded = state.encode().unwrap();
        let decoded = OAuthState::decode(&encoded).unwrap();

        assert!(!decoded.is_login());
        assert_eq!(decoded.get_link_user_id(), Some(user_id));
        assert_eq!(decoded.operation, state.operation);
        assert_eq!(decoded.nonce, state.nonce);
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
    fn replayed_state_is_rejected() {
        let state = OAuthState::new_login();
        let encoded = state.encode().unwrap();
        assert!(OAuthState::decode(&encoded).is_ok());
        assert!(matches!(
            OAuthState::decode(&encoded),
            Err(StateError::Replay)
        ));
    }
}
