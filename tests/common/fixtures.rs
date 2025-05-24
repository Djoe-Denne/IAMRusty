use serde_json::json;
use uuid::Uuid;
use wiremock::{
    matchers::{method, path_regex},
    Mock, MockServer, ResponseTemplate,
};

/// OAuth state fixtures for testing
pub struct OAuthStateFixtures;

impl OAuthStateFixtures {
    /// Create a valid login state
    pub fn valid_login_state() -> String {
        let state = http_server::oauth_state::OAuthState::new_login();
        state.encode().expect("Failed to encode login state")
    }
    
    /// Create a valid link state for a user
    pub fn valid_link_state(user_id: Uuid) -> String {
        let state = http_server::oauth_state::OAuthState::new_link(user_id);
        state.encode().expect("Failed to encode link state")
    }
    
    /// Create an invalid state (malformed base64)
    pub fn invalid_state() -> String {
        "invalid!@#$%^&*()".to_string()
    }
    
    /// Create an expired or tampered state
    pub fn tampered_state() -> String {
        "dGFtcGVyZWRzdGF0ZQ==".to_string() // base64 for "tamperedstate"
    }
}

/// User fixtures for testing
pub struct UserFixtures;

impl UserFixtures {
    /// Create a test user ID
    pub fn test_user_id() -> Uuid {
        Uuid::parse_str("01234567-89ab-cdef-0123-456789abcdef").unwrap()
    }
    
    /// Create a test JWT token (mock)
    pub fn test_jwt_token() -> String {
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIwMTIzNDU2Ny04OWFiLWNkZWYtMDEyMy00NTY3ODlhYmNkZWYiLCJpYXQiOjE3MDAwMDAwMDB9.test".to_string()
    }
}

/// Mock OAuth provider responses
pub struct MockOAuthProvider {
    pub server: MockServer,
}

impl MockOAuthProvider {
    /// Create a new mock OAuth provider server
    pub async fn new() -> Self {
        let server = MockServer::start().await;
        Self { server }
    }
    
    /// Setup successful GitHub authorization response
    pub async fn setup_github_success(&self) -> String {
        let auth_code = "test_auth_code_123";
        
        // Mock the token exchange endpoint
        Mock::given(method("POST"))
            .and(path_regex("/login/oauth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "gho_test_access_token",
                "token_type": "bearer",
                "scope": "user:email"
            })))
            .mount(&self.server)
            .await;
        
        // Mock the user info endpoint
        Mock::given(method("GET"))
            .and(path_regex("/user"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 12345,
                "login": "testuser",
                "email": "test@example.com",
                "avatar_url": "https://github.com/images/avatar.png",
                "name": "Test User"
            })))
            .mount(&self.server)
            .await;
        
        // Mock the user emails endpoint
        Mock::given(method("GET"))
            .and(path_regex("/user/emails"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "email": "test@example.com",
                    "primary": true,
                    "verified": true,
                    "visibility": "public"
                }
            ])))
            .mount(&self.server)
            .await;
        
        auth_code.to_string()
    }
    
    /// Setup successful GitLab authorization response
    pub async fn setup_gitlab_success(&self) -> String {
        let auth_code = "gitlab_auth_code_456";
        
        // Mock the token exchange endpoint
        Mock::given(method("POST"))
            .and(path_regex("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "glpat_test_access_token",
                "token_type": "Bearer",
                "scope": "read_user"
            })))
            .mount(&self.server)
            .await;
        
        // Mock the user info endpoint
        Mock::given(method("GET"))
            .and(path_regex("/api/v4/user"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 67890,
                "username": "gitlabuser",
                "email": "gitlab@example.com",
                "avatar_url": "https://gitlab.com/avatar.png",
                "name": "GitLab User"
            })))
            .mount(&self.server)
            .await;
        
        auth_code.to_string()
    }
    
    /// Setup OAuth error response
    pub async fn setup_oauth_error(&self, error: &str, description: &str) {
        Mock::given(method("POST"))
            .and(path_regex("/oauth/token|/login/oauth/access_token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": error,
                "error_description": description
            })))
            .mount(&self.server)
            .await;
    }
    
    /// Get the base URL for this mock server
    pub fn base_url(&self) -> String {
        self.server.uri()
    }
}

/// Test request builders
pub struct TestRequestBuilder;

impl TestRequestBuilder {
    /// Create authorization header with Bearer token
    pub fn auth_header(token: &str) -> (String, String) {
        ("Authorization".to_string(), format!("Bearer {}", token))
    }
    
    /// Create OAuth callback query parameters
    pub fn oauth_callback_query(code: &str, state: Option<&str>) -> Vec<(String, String)> {
        let mut params = vec![("code".to_string(), code.to_string())];
        if let Some(s) = state {
            params.push(("state".to_string(), s.to_string()));
        }
        params
    }
    
    /// Create OAuth error callback query parameters
    pub fn oauth_error_query(error: &str, description: Option<&str>) -> Vec<(String, String)> {
        let mut params = vec![("error".to_string(), error.to_string())];
        if let Some(desc) = description {
            params.push(("error_description".to_string(), desc.to_string()));
        }
        params
    }
}

/// Response assertion helpers
pub struct ResponseAssertions;

impl ResponseAssertions {
    /// Assert redirect response contains required query parameters
    pub fn assert_redirect_has_params(location: &str, expected_params: &[&str]) {
        let url = url::Url::parse(location).expect("Invalid redirect URL");
        let query_pairs: std::collections::HashMap<String, String> = url.query_pairs().into_owned().collect();
        
        for param in expected_params {
            assert!(
                query_pairs.contains_key(*param),
                "Missing query parameter '{}' in redirect URL: {}",
                param,
                location
            );
        }
    }
    
    /// Assert state parameter is valid and decodable
    pub fn assert_valid_state(state: &str) {
        http_server::oauth_state::OAuthState::decode(state)
            .expect("State parameter should be valid and decodable");
    }
    
    /// Assert OAuth error response format
    pub fn assert_oauth_error_response(body: &serde_json::Value, expected_error: &str) {
        assert_eq!(
            body["error"].as_str(),
            Some(expected_error),
            "Error field should match expected error"
        );
        assert!(
            body["message"].as_str().is_some(),
            "Error response should contain a message"
        );
    }
    
    /// Assert OAuth success response format
    pub fn assert_oauth_success_response(body: &serde_json::Value) {
        assert!(
            body["operation"].as_str().is_some(),
            "Success response should contain operation type"
        );
        assert!(
            body["user"].is_object(),
            "Success response should contain user data"
        );
    }
} 