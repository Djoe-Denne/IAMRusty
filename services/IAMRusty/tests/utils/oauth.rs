use iam_http_server::OAuthState;
use serde_json::Value;

/// OAuth test utilities for invalid state creation and signed state inspection.
pub struct OAuthTestUtils;

impl OAuthTestUtils {
    /// Create an invalid OAuth state (for negative testing)
    pub fn create_invalid_state() -> String {
        "invalid_base64_state".to_string()
    }

    /// Decode and verify OAuth state parameter
    pub fn decode_state(state: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let decoded = OAuthState::inspect(state)?;
        Ok(serde_json::to_value(decoded)?)
    }
}
