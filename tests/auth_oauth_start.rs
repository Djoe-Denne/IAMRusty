// Include common test utilities and fixtures
#[path = "common/mod.rs"] 
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::get_test_server;
use fixtures::{GitHubFixtures, GitLabFixtures};
use reqwest::Client;
use serde_json::Value;
use url::Url;
use base64::{Engine as _, engine::general_purpose};
use serial_test::serial;
use uuid;

/// Create a common HTTP client for tests that doesn't follow redirects automatically
fn create_test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client")
}

/// Helper function to decode and verify OAuth state parameter
fn decode_oauth_state(state: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let decoded_bytes = general_purpose::URL_SAFE_NO_PAD.decode(state)?;
    let decoded_str = String::from_utf8(decoded_bytes)?;
    let state_json: Value = serde_json::from_str(&decoded_str)?;
    Ok(state_json)
}

/// Helper function to verify redirect URL structure and extract query parameters
fn parse_redirect_url(location: &str) -> Result<(String, std::collections::HashMap<String, String>), Box<dyn std::error::Error>> {
    let url = Url::parse(location)?;
    let mut params = std::collections::HashMap::new();
    
    for (key, value) in url.query_pairs() {
        params.insert(key.to_string(), value.to_string());
    }
    
    Ok((url.origin().ascii_serialization() + url.path(), params))
}

// 🔐 Authentication & OAuth Flow Tests
// 🔁 /auth/{provider}/start

#[tokio::test]
#[serial]
async fn test_oauth_start_github_redirect_success() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup GitHub fixtures (scoped to this test)
    let _github_service = GitHubFixtures::service().await;
    
    // Make request to GitHub OAuth start endpoint
    let response = client
        .get(&format!("{}/api/auth/github/start", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    // ✅ Should return 303 redirect (instead of 307 as per requirements)
    assert_eq!(response.status(), 303, "Should return 303 redirect status");
    
    // ✅ Should have Location header
    let location = response
        .headers()
        .get("location")
        .expect("Should have Location header")
        .to_str()
        .expect("Location header should be valid string");
    
    // ✅ Should redirect to GitHub OAuth URL
    assert!(location.contains("github.com") || location.contains("localhost:3000"), 
           "Should redirect to GitHub OAuth provider (or mock)");
    
    // ✅ Parse redirect URL and verify query parameters
    let (base_path, params) = parse_redirect_url(location)
        .expect("Should be able to parse redirect URL");
    
    // ✅ Verify correct query params are present
    assert!(params.contains_key("client_id"), "Should have client_id parameter");
    assert!(params.contains_key("redirect_uri"), "Should have redirect_uri parameter");
    assert!(params.contains_key("scope"), "Should have scope parameter");
    assert!(params.contains_key("response_type"), "Should have response_type parameter");
    assert!(params.contains_key("state"), "Should have state parameter");
    
    // ✅ Verify response_type is 'code'
    assert_eq!(params.get("response_type").unwrap(), "code", "response_type should be 'code'");
    
    // ✅ Verify redirect_uri points back to our callback
    let redirect_uri = params.get("redirect_uri").unwrap();
    assert!(redirect_uri.contains("/api/auth/github/callback"), 
           "redirect_uri should point to our GitHub callback endpoint");
    
    // ✅ Verify state parameter is properly encoded and contains login operation
    let state = params.get("state").unwrap();
    let decoded_state = decode_oauth_state(state)
        .expect("Should be able to decode state parameter");
    
    assert_eq!(decoded_state["operation"]["type"], "login", 
              "State should contain login operation type");
    assert!(decoded_state["nonce"].is_string(), 
           "State should contain nonce for security");
}

#[tokio::test]
#[serial]
async fn test_oauth_start_gitlab_redirect_success() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup GitLab fixtures (scoped to this test)
    let _gitlab_service = GitLabFixtures::service().await;
    
    // Make request to GitLab OAuth start endpoint
    let response = client
        .get(&format!("{}/api/auth/gitlab/start", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    // ✅ Should return 303 redirect (instead of 307 as per requirements)
    assert_eq!(response.status(), 303, "Should return 303 redirect status");
    
    // ✅ Should have Location header
    let location = response
        .headers()
        .get("location")
        .expect("Should have Location header")
        .to_str()
        .expect("Location header should be valid string");
    
    // ✅ Should redirect to GitLab OAuth URL
    assert!(location.contains("gitlab.com") || location.contains("localhost:3000"), 
           "Should redirect to GitLab OAuth provider (or mock)");
    
    // ✅ Parse redirect URL and verify query parameters
    let (base_path, params) = parse_redirect_url(location)
        .expect("Should be able to parse redirect URL");
    
    // ✅ Verify correct query params are present
    assert!(params.contains_key("client_id"), "Should have client_id parameter");
    assert!(params.contains_key("redirect_uri"), "Should have redirect_uri parameter");
    assert!(params.contains_key("scope"), "Should have scope parameter");
    assert!(params.contains_key("response_type"), "Should have response_type parameter");
    assert!(params.contains_key("state"), "Should have state parameter");
    
    // ✅ Verify response_type is 'code'
    assert_eq!(params.get("response_type").unwrap(), "code", "response_type should be 'code'");
    
    // ✅ Verify redirect_uri points back to our callback
    let redirect_uri = params.get("redirect_uri").unwrap();
    assert!(redirect_uri.contains("/api/auth/gitlab/callback"), 
           "redirect_uri should point to our GitLab callback endpoint");
    
    // ✅ Verify state parameter is properly encoded and contains login operation
    let state = params.get("state").unwrap();
    let decoded_state = decode_oauth_state(state)
        .expect("Should be able to decode state parameter");
    
    assert_eq!(decoded_state["operation"]["type"], "login", 
              "State should contain login operation type");
    assert!(decoded_state["nonce"].is_string(), 
           "State should contain nonce for security");
}

#[tokio::test]
#[serial]
async fn test_oauth_start_unsupported_provider_returns_400() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Test unsupported providers
    let unsupported_providers = vec!["facebook", "google", "twitter", "unknown", ""];
    
    for provider in unsupported_providers {
        let response = client
            .get(&format!("{}/api/auth/{}/start", base_url, provider))
            .send()
            .await
            .expect("Failed to send request");
        
        // ❌ Should return 400 for unsupported providers
        assert_eq!(response.status(), 400, 
                  "Should return 400 Bad Request for unsupported provider: {}", provider);
        
        // Should return JSON error response
        let error_response: Value = response
            .json()
            .await
            .expect("Should return JSON error response");
        
        assert_eq!(error_response["operation"], "start", 
                  "Error response should indicate 'start' operation");
        assert_eq!(error_response["error"], "invalid_provider", 
                  "Error response should indicate 'invalid_provider'");
        assert!(error_response["message"].is_string(), 
               "Error response should have error message");
    }
}

#[tokio::test]
#[serial]
async fn test_oauth_start_case_insensitive_providers() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup fixtures
    let _github_service = GitHubFixtures::service().await;
    let _gitlab_service = GitLabFixtures::service().await;
    
    // Test case variations that should work
    let valid_cases = vec![
        ("github", "GitHub"),
        ("GITHUB", "GitHub"), 
        ("GitHub", "GitHub"),
        ("gitlab", "GitLab"),
        ("GITLAB", "GitLab"),
        ("GitLab", "GitLab"),
    ];
    
    for (provider_input, expected_provider) in valid_cases {
        let response = client
            .get(&format!("{}/api/auth/{}/start", base_url, provider_input))
            .send()
            .await
            .expect("Failed to send request");
        
        // ✅ Should successfully redirect regardless of case
        assert_eq!(response.status(), 303, 
                  "Should handle case-insensitive provider: {}", provider_input);
        
        let location = response
            .headers()
            .get("location")
            .expect("Should have Location header")
            .to_str()
            .expect("Location header should be valid string");
        
        // Verify the redirect goes to the correct provider
        if expected_provider == "GitHub" {
            assert!(location.contains("github") || location.contains("localhost:3000"));
        } else {
            assert!(location.contains("gitlab") || location.contains("localhost:3000"));
        }
    }
}

#[tokio::test]
#[serial]
async fn test_oauth_start_state_security_and_uniqueness() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup GitHub fixtures (scoped to this test)
    let _github_service = GitHubFixtures::service().await;
    
    // Make multiple requests to verify state uniqueness
    let mut states = std::collections::HashSet::new();
    
    for i in 0..5 {
        let response = client
            .get(&format!("{}/api/auth/github/start", base_url))
            .send()
            .await
            .expect("Failed to send request");
        
        assert_eq!(response.status(), 303);
        
        let location = response
            .headers()
            .get("location")
            .expect("Should have Location header")
            .to_str()
            .expect("Location header should be valid string");
        
        let (_, params) = parse_redirect_url(location)
            .expect("Should be able to parse redirect URL");
        
        let state = params.get("state").unwrap();
        
        // ✅ Each state should be unique
        assert!(!states.contains(state), 
               "State parameter should be unique across requests (iteration {})", i);
        states.insert(state.clone());
        
        // ✅ State should be properly base64 encoded
        let decoded_state = decode_oauth_state(state)
            .expect("State should be valid base64 encoded JSON");
        
        // ✅ State should contain required security fields
        assert_eq!(decoded_state["operation"]["type"], "login");
        assert!(decoded_state["nonce"].is_string());
        
        // ✅ Nonce should be a valid UUID format
        let nonce = decoded_state["nonce"].as_str().unwrap();
        assert!(uuid::Uuid::parse_str(nonce).is_ok(), 
               "Nonce should be a valid UUID");
    }
    
    // ✅ Verify we collected 5 unique states
    assert_eq!(states.len(), 5, "Should generate 5 unique state parameters");

}

#[tokio::test]
#[serial]
async fn test_oauth_start_with_auth_header_link_operation() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup GitHub fixtures (scoped to this test)
    let _github_service = GitHubFixtures::service().await;
    
    // First, we need a valid JWT token (in a real scenario, this would come from a login)
    // For this test, we'll use a mock JWT token that would be validated by the system
    // Note: This test assumes the system can validate tokens - if not, it will return 401
    let mock_jwt_token = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjNlNDU2Ny1lODliLTEyZDMtYTQ1Ni00MjY2MTQxNzQwMDAiLCJleHAiOjk5OTk5OTk5OTl9.test";
    
    // Make request with Authorization header for provider linking
    let response = client
        .get(&format!("{}/api/auth/github/start", base_url))
        .header("Authorization", mock_jwt_token)
        .send()
        .await
        .expect("Failed to send request");
    
    // The response will depend on whether the JWT validation succeeds
    if response.status() == 303 {
        // ✅ If token is valid, should redirect with link operation state
        let location = response
            .headers()
            .get("location")
            .expect("Should have Location header")
            .to_str()
            .expect("Location header should be valid string");
        
        let (_, params) = parse_redirect_url(location)
            .expect("Should be able to parse redirect URL");
        
        let state = params.get("state").unwrap();
        let decoded_state = decode_oauth_state(state)
            .expect("Should be able to decode state parameter");
        
        // ✅ State should contain link operation with user_id
        assert_eq!(decoded_state["operation"]["type"], "link", 
                  "State should contain link operation type when Authorization header is present");
        assert!(decoded_state["operation"]["user_id"].is_string(), 
               "Link operation should contain user_id");
        assert!(decoded_state["nonce"].is_string(), 
               "State should contain nonce for security");
    } else if response.status() == 401 {
        // ✅ If token is invalid, should return 401
        let error_response: Value = response
            .json()
            .await
            .expect("Should return JSON error response");
        
        assert_eq!(error_response["operation"], "start");
        assert_eq!(error_response["error"], "invalid_token");
    } else {
        panic!("Unexpected response status: {}", response.status());
    }
}

#[tokio::test]
#[serial]
async fn test_oauth_start_invalid_auth_header_formats() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Test various invalid Authorization header formats
    let invalid_headers = vec![
        "Invalid token format",
        "Basic dXNlcjpwYXNz", // Basic auth instead of Bearer
        "Bearer", // Missing token
        "bearer token", // Wrong case
        "", // Empty header
    ];
    
    for invalid_header in invalid_headers {
        let response = client
            .get(&format!("{}/api/auth/github/start", base_url))
            .header("Authorization", invalid_header)
            .send()
            .await
            .expect("Failed to send request");
        
        // ❌ Should return 400 for invalid Authorization header format
        assert_eq!(response.status(), 400, 
                  "Should return 400 for invalid Authorization header: '{}'", invalid_header);
        
        let error_response: Value = response
            .json()
            .await
            .expect("Should return JSON error response");
        
        assert_eq!(error_response["operation"], "start");
        assert_eq!(error_response["error"], "invalid_authorization_header");
    }

}

#[tokio::test]
#[serial]
async fn test_oauth_start_query_parameter_structure() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup fixtures
    let _github_service = GitHubFixtures::service().await;
    let _gitlab_service = GitLabFixtures::service().await;
    
    let providers = vec!["github", "gitlab"];
    
    for provider in providers {
        let response = client
            .get(&format!("{}/api/auth/{}/start", base_url, provider))
            .send()
            .await
            .expect("Failed to send request");
        
        assert_eq!(response.status(), 303);
        
        let location = response
            .headers()
            .get("location")
            .expect("Should have Location header")
            .to_str()
            .expect("Location header should be valid string");
        
        let (_, params) = parse_redirect_url(location)
            .expect("Should be able to parse redirect URL");
        
        // ✅ Verify all required OAuth2 parameters are present
        let required_params = vec!["client_id", "redirect_uri", "scope", "response_type", "state"];
        for param in required_params {
            assert!(params.contains_key(param), 
                   "Should have required OAuth2 parameter '{}' for provider '{}'", param, provider);
            assert!(!params.get(param).unwrap().is_empty(), 
                   "OAuth2 parameter '{}' should not be empty for provider '{}'", param, provider);
        }
        
        // ✅ Verify parameter values meet OAuth2 standards
        assert_eq!(params.get("response_type").unwrap(), "code", 
                  "response_type should be 'code' for authorization code flow");
        
        // ✅ Verify scope contains expected values (depends on provider)
        let scope = params.get("scope").unwrap();
        if provider == "github" {
            assert!(scope.contains("user") || scope.contains("read:user"), 
                   "GitHub scope should include user permissions");
        } else if provider == "gitlab" {
            assert!(scope.contains("read_user") || scope.contains("openid"), 
                   "GitLab scope should include user permissions");
        }
        
        // ✅ Verify redirect_uri is properly URL encoded and contains correct callback path
        let redirect_uri = params.get("redirect_uri").unwrap();
        assert!(redirect_uri.contains(&format!("/api/auth/{}/callback", provider)), 
               "redirect_uri should point to correct callback endpoint for provider '{}'", provider);
    }

}
