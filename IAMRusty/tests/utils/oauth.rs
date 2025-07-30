use base64::{engine::general_purpose, Engine as _};
use serde_json::Value;
use std::collections::HashMap;
use url::Url;
use uuid::Uuid;

/// OAuth Test Utilities for state management and URL parsing
pub struct OAuthTestUtils;

impl OAuthTestUtils {
    /// Create a valid OAuth state for login operation
    pub fn create_login_state() -> String {
        let state_data = serde_json::json!({
            "operation": {
                "type": "login"
            },
            "nonce": Uuid::new_v4().to_string()
        });
        general_purpose::URL_SAFE_NO_PAD.encode(state_data.to_string())
    }

    /// Create a valid OAuth state for link operation
    pub fn create_link_state(user_id: Uuid) -> String {
        let state_data = serde_json::json!({
            "operation": {
                "type": "link",
                "user_id": user_id.to_string()
            },
            "nonce": Uuid::new_v4().to_string()
        });
        general_purpose::URL_SAFE_NO_PAD.encode(state_data.to_string())
    }

    /// Create an invalid OAuth state (for negative testing)
    pub fn create_invalid_state() -> String {
        "invalid_base64_state".to_string()
    }

    /// Decode and verify OAuth state parameter
    pub fn decode_state(state: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let decoded_bytes = general_purpose::URL_SAFE_NO_PAD.decode(state)?;
        let decoded_str = String::from_utf8(decoded_bytes)?;
        let state_json: Value = serde_json::from_str(&decoded_str)?;
        Ok(state_json)
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
            "State should contain {} operation type",
            expected_operation
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
            decoded_state["operation"]["user_id"].as_str().unwrap(),
            expected_user_id.to_string(),
            "Link operation should contain correct user_id"
        );
    }

    /// Assert redirect URL contains required OAuth parameters
    pub fn assert_oauth_redirect_params(location: &str, provider: &str) {
        let (_, params) =
            Self::parse_redirect_url(location).expect("Should be able to parse redirect URL");

        // Verify all required OAuth2 parameters are present
        let required_params = vec![
            "client_id",
            "redirect_uri",
            "scope",
            "response_type",
            "state",
        ];

        for param in required_params {
            assert!(
                params.contains_key(param),
                "Should have required OAuth2 parameter '{}' for provider '{}'",
                param,
                provider
            );
            assert!(
                !params.get(param).unwrap().is_empty(),
                "OAuth2 parameter '{}' should not be empty for provider '{}'",
                param,
                provider
            );
        }

        // Verify response_type is 'code'
        assert_eq!(
            params.get("response_type").unwrap(),
            "code",
            "response_type should be 'code' for authorization code flow"
        );

        // Verify redirect_uri contains correct callback path
        let redirect_uri = params.get("redirect_uri").unwrap();
        assert!(
            redirect_uri.contains(&format!("/oauth/{}/callback", provider)),
            "redirect_uri should point to correct callback endpoint for provider '{}'",
            provider
        );
    }

    /// Assert OAuth state is unique across multiple requests
    pub fn assert_state_uniqueness(states: &[String]) {
        let unique_states: std::collections::HashSet<_> = states.iter().collect();
        assert_eq!(
            unique_states.len(),
            states.len(),
            "All OAuth states should be unique"
        );
    }
}
