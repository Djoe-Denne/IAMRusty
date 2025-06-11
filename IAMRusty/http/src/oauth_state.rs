//! OAuth state parameter handling for operation context

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose};
use thiserror::Error;

/// OAuth operation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
}

impl OAuthState {
    /// Create a new login state
    pub fn new_login() -> Self {
        Self {
            operation: OAuthOperation::Login,
            nonce: uuid::Uuid::new_v4().to_string(),
        }
    }
    
    /// Create a new link provider state
    pub fn new_link(user_id: Uuid) -> Self {
        Self {
            operation: OAuthOperation::Link { user_id },
            nonce: uuid::Uuid::new_v4().to_string(),
        }
    }
    
    /// Encode the state to a base64 string for use in OAuth flow
    pub fn encode(&self) -> Result<String, StateError> {
        let json = serde_json::to_string(self)?;
        Ok(general_purpose::URL_SAFE_NO_PAD.encode(json))
    }
    
    /// Decode a base64 string back to OAuth state
    pub fn decode(encoded: &str) -> Result<Self, StateError> {
        let json_bytes = general_purpose::URL_SAFE_NO_PAD.decode(encoded)?;
        let json = String::from_utf8(json_bytes)
            .map_err(|_| StateError::InvalidFormat)?;
        let state: OAuthState = serde_json::from_str(&json)?;
        Ok(state)
    }
    
    /// Check if this is a login operation
    pub fn is_login(&self) -> bool {
        matches!(self.operation, OAuthOperation::Login)
    }
    
    /// Check if this is a link operation and return the user ID
    pub fn get_link_user_id(&self) -> Option<Uuid> {
        match &self.operation {
            OAuthOperation::Link { user_id } => Some(*user_id),
            _ => None,
        }
    }
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
} 