use iam_http_server::OAuthState;
use serde_json::Value;
use std::collections::HashMap;
use url::Url;
use uuid::Uuid;

/// OAuth Test Utilities for state management and URL parsing
pub struct OAuthTestUtils;

impl OAuthTestUtils {
    /// Create a valid OAuth state for login operation
    pub fn create_login_state() -> String {
        OAuthState::new_login()
            .encode()
            .expect("signed login state")
    }

    /// Create a valid OAuth state for link operation
    pub fn create_link_state(user_id: Uuid) -> String {
        OAuthState::new_link(user_id)
            .encode()
            .expect("signed link state")
    }

    /// Create an invalid OAuth state (for negative testing)
    pub fn create_invalid_state() -> String {
        "invalid_base64_state".to_string()
    }

    /// Decode and verify OAuth state parameter
    pub fn decode_state(state: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let decoded = OAuthState::inspect(state)?;
        Ok(serde_json::to_value(decoded)?)
    }

    /// Parse redirect URL and extract query parameters
    pub fn parse_redirect_url(
        location: &str,
    ) -> Result<(String, HashMap<String, String>), Box<dyn std::error::Error>> {
        let url = Url::parse(location)?;
        let mut params = HashMap::new();

        for (key, value) in url.query_pairs() {
            params.insert(key.to_string(), value.to_string());
        }

        Ok((url.origin().ascii_serialization() + url.path(), params))
    }

    /// Assert OAuth state has valid structure and operation type
    pub fn assert_state_operation(state: &str, expected_operation: &str) {
        let decoded_state =
            Self::decode_state(state).expect("Should be able to decode OAuth state");

        assert_eq!(
            decoded_state["operation"]["type"], expected_operation,
            "State should contain {expected_operation} operation type"
        );
        assert!(
            decoded_state["nonce"].is_string(),
            "State should contain nonce for security"
        );
    }

    /// Assert OAuth state has link operation with user ID
    pub fn assert_link_state_with_user_id(state: &str, expected_user_id: Uuid) {
        let decoded_state =
            Self::decode_state(state).expect("Should be able to decode OAuth state");

        assert_eq!(
            decoded_state["operation"]["type"], "link",
            "State should contain link operation type"
        );
        assert_eq!(
            decoded_state["operation"]["user_id"],
            expected_user_id.to_string(),
            "State should contain the expected user ID"
        );
    }
}
