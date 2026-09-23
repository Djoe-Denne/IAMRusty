// Include common test utilities and fixtures

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;
mod utils;

use common::setup_test_server;
use fixtures::IdpConnectFixtures;
use iam_infra::auth::HttpIdpConnector;
use idp_connect_contract::{FederatedOAuthClient, FederatedOAuthError};
use serde_json::Value;
use serial_test::serial;
use url::Url;
use utils::oauth::OAuthTestUtils;

/// Helper function to decode and verify a signed OAuth state parameter
fn decode_oauth_state(state: &str) -> Result<Value, Box<dyn std::error::Error>> {
    OAuthTestUtils::decode_state(state)
}

/// Helper function to verify redirect URL structure and extract query parameters
fn parse_redirect_url(
    location: &str,
) -> Result<(String, std::collections::HashMap<String, String>), Box<dyn std::error::Error>> {
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
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Setup GitHub fixtures (scoped to this test)
    let idp = IdpConnectFixtures::service().await;
    idp.mock_github_happy_arthur().await;

    // Make request to GitHub OAuth login endpoint (updated endpoint)
    let response = client
        .get(format!("{base_url}/api/auth/github/login"))
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

    assert!(
        location.starts_with("http://localhost:3000/login/oauth/authorize"),
        "Should redirect to the WireMock connector authorize URL, not github.com"
    );

    // ✅ Parse redirect URL and verify query parameters
    let (_base_path, params) =
        parse_redirect_url(location).expect("Should be able to parse redirect URL");

    // ✅ Verify correct query params are present
    assert!(
        params.contains_key("client_id"),
        "Should have client_id parameter"
    );
    assert!(
        params.contains_key("redirect_uri"),
        "Should have redirect_uri parameter"
    );
    assert!(params.contains_key("scope"), "Should have scope parameter");
    assert!(
        params.contains_key("response_type"),
        "Should have response_type parameter"
    );
    assert!(params.contains_key("state"), "Should have state parameter");

    // ✅ Verify response_type is 'code'
    assert_eq!(
        params.get("response_type").unwrap(),
        "code",
        "response_type should be 'code'"
    );

    // ✅ Verify redirect_uri points back to our callback
    let redirect_uri = params.get("redirect_uri").unwrap();
    assert!(
        redirect_uri.contains("github"),
        "redirect_uri should point to our GitHub callback endpoint"
    );
    assert!(
        redirect_uri.ends_with("/callback"),
        "login redirect_uri must use the callback path"
    );
    assert!(
        !redirect_uri.contains("relink-callback"),
        "login redirect_uri must not use relink-callback"
    );

    // ✅ Verify state parameter is properly encoded and contains login operation
    let state = params.get("state").unwrap();
    let decoded_state =
        decode_oauth_state(state).expect("Should be able to decode state parameter");

    assert_eq!(
        decoded_state["operation"]["type"], "login",
        "State should contain login operation type"
    );
    assert!(
        decoded_state["nonce"].is_string(),
        "State should contain nonce for security"
    );
}

#[tokio::test]
#[serial]
async fn test_oauth_start_gitlab_redirect_success() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Setup GitLab fixtures (scoped to this test)
    let idp = IdpConnectFixtures::service().await;
    idp.mock_gitlab_happy_alice().await;

    // Make request to GitLab OAuth login endpoint (updated endpoint)
    let response = client
        .get(format!("{base_url}/api/auth/gitlab/login"))
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

    assert!(
        location.starts_with("http://localhost:3000/oauth/authorize"),
        "Should redirect to the WireMock connector authorize URL, not gitlab.com"
    );

    // ✅ Parse redirect URL and verify query parameters
    let (_base_path, params) =
        parse_redirect_url(location).expect("Should be able to parse redirect URL");

    // ✅ Verify correct query params are present
    assert!(
        params.contains_key("client_id"),
        "Should have client_id parameter"
    );
    assert!(
        params.contains_key("redirect_uri"),
        "Should have redirect_uri parameter"
    );
    assert!(params.contains_key("scope"), "Should have scope parameter");
    assert!(
        params.contains_key("response_type"),
        "Should have response_type parameter"
    );
    assert!(params.contains_key("state"), "Should have state parameter");

    // ✅ Verify response_type is 'code'
    assert_eq!(
        params.get("response_type").unwrap(),
        "code",
        "response_type should be 'code'"
    );

    // ✅ Verify redirect_uri points back to our callback
    let redirect_uri = params.get("redirect_uri").unwrap();
    assert!(
        redirect_uri.contains("gitlab"),
        "redirect_uri should point to our GitLab callback endpoint"
    );
    assert!(
        redirect_uri.ends_with("/callback"),
        "login redirect_uri must use the callback path"
    );
    assert!(
        !redirect_uri.contains("relink-callback"),
        "login redirect_uri must not use relink-callback"
    );

    // ✅ Verify state parameter is properly encoded and contains login operation
    let state = params.get("state").unwrap();
    let decoded_state =
        decode_oauth_state(state).expect("Should be able to decode state parameter");

    assert_eq!(
        decoded_state["operation"]["type"], "login",
        "State should contain login operation type"
    );
    assert!(
        decoded_state["nonce"].is_string(),
        "State should contain nonce for security"
    );
}

#[tokio::test]
#[serial]
async fn test_oauth_start_unsupported_provider_returns_422() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test unsupported providers (letters-only slugs absent from the boot registry)
    let unsupported_providers = vec!["facebook", "google", "twitter", "unknown", "bitbucket"];

    for provider in unsupported_providers {
        let response = client
            .get(format!("{base_url}/api/auth/{provider}/login"))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(
            response.status(),
            422,
            "Should return 422 Unprocessable Entity for unregistered provider: {provider}"
        );

        let error_response: Value = response
            .json()
            .await
            .expect("Should return JSON error response");

        assert_eq!(
            error_response["error"]["error_code"], "connector_not_configured",
            "Unregistered slug {provider} should be connector_not_configured"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_oauth_start_illegal_syntax_returns_400() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    let overlong = "a".repeat(51);
    for provider in ["hugging-face", "github2", overlong.as_str()] {
        let response = client
            .get(format!("{base_url}/api/auth/{provider}/login"))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(
            response.status(),
            400,
            "Should return 400 for illegal provider syntax: {provider}"
        );

        let error_response: Value = response
            .json()
            .await
            .expect("Should return JSON error response");

        assert_eq!(
            error_response["error"]["error_code"], "invalid_provider",
            "Illegal slug {provider} should be invalid_provider"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_oauth_start_case_insensitive_providers() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Setup fixtures
    let idp = IdpConnectFixtures::service().await;
    idp.mock_github_happy_arthur().await;
    idp.mock_gitlab_happy_alice().await;

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
            .get(format!("{base_url}/api/auth/{provider_input}/login"))
            .send()
            .await
            .expect("Failed to send request");

        // ✅ Should successfully redirect regardless of case
        assert_eq!(
            response.status(),
            303,
            "Should handle case-insensitive provider: {provider_input}"
        );

        let location = response
            .headers()
            .get("location")
            .expect("Should have Location header")
            .to_str()
            .expect("Location header should be valid string");

        if expected_provider == "GitHub" {
            assert!(
                location.starts_with("http://localhost:3000/login/oauth/authorize"),
                "GitHub start must use the connector mock, not github.com"
            );
        } else {
            assert!(
                location.starts_with("http://localhost:3000/oauth/authorize"),
                "GitLab start must use the connector mock, not gitlab.com"
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_oauth_start_state_security_and_uniqueness() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Setup GitHub fixtures (scoped to this test)
    let idp = IdpConnectFixtures::service().await;
    idp.mock_github_happy_arthur().await;

    // Make multiple requests to verify state uniqueness
    let mut states = std::collections::HashSet::new();

    for i in 0..5 {
        let response = client
            .get(format!("{base_url}/api/auth/github/login"))
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

        let (_, params) =
            parse_redirect_url(location).expect("Should be able to parse redirect URL");

        let state = params.get("state").unwrap();

        // ✅ Each state should be unique
        assert!(
            !states.contains(state),
            "State parameter should be unique across requests (iteration {i})"
        );
        states.insert(state.clone());

        // ✅ State should be properly base64 encoded
        let decoded_state =
            decode_oauth_state(state).expect("State should be valid base64 encoded JSON");

        // ✅ State should contain required security fields
        assert_eq!(decoded_state["operation"]["type"], "login");
        assert!(decoded_state["nonce"].is_string());

        // ✅ Nonce should be a valid UUID format
        let nonce = decoded_state["nonce"].as_str().unwrap();
        assert!(
            uuid::Uuid::parse_str(nonce).is_ok(),
            "Nonce should be a valid UUID"
        );
    }

    // ✅ Verify we collected 5 unique states
    assert_eq!(states.len(), 5, "Should generate 5 unique state parameters");
}

#[tokio::test]
#[serial]
async fn test_oauth_start_with_auth_header_link_operation() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Setup GitHub fixtures (scoped to this test)
    let idp = IdpConnectFixtures::service().await;
    idp.mock_github_happy_arthur().await;

    // First, we need a valid JWT token (in a real scenario, this would come from a login)
    // For this test, we'll use a mock JWT token that would be validated by the system
    // Note: This test assumes the system can validate tokens - if not, it will return 401
    let mock_jwt_token = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjNlNDU2Ny1lODliLTEyZDMtYTQ1Ni00MjY2MTQxNzQwMDAiLCJleHAiOjk5OTk5OTk5OTl9.test";

    // Make request to the link endpoint with Authorization header for provider linking
    let response = client
        .get(format!("{base_url}/api/auth/github/link"))
        .header("Authorization", mock_jwt_token)
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Since we're using a mock/invalid token, should return 401 (Unauthorized)
    // The link endpoint now properly requires authentication
    assert_eq!(
        response.status(),
        401,
        "Should return 401 Unauthorized for invalid token on link endpoint"
    );

    // The response might be empty or contain error details
    // Let's be flexible about the response format since this is an auth middleware response
    let response_text = response.text().await.unwrap_or_default();

    // Just verify we got an unauthorized response - the exact format may vary
    // based on the auth middleware implementation
    println!("Response for invalid token: {response_text}");
}

#[tokio::test]
#[serial]
async fn test_oauth_start_invalid_auth_header_formats() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test various invalid Authorization header formats
    let invalid_headers = vec![
        "Invalid token format",
        "Basic dXNlcjpwYXNz", // Basic auth instead of Bearer
        "Bearer",             // Missing token
        "bearer token",       // Wrong case
        "",                   // Empty header
    ];

    for invalid_header in invalid_headers {
        let response = client
            .get(format!("{base_url}/api/auth/github/link"))
            .header("Authorization", invalid_header)
            .send()
            .await
            .expect("Failed to send request");

        // ✅ Should return 401 for invalid Authorization header format
        // The link endpoint now properly requires authentication via middleware
        assert_eq!(
            response.status(),
            401,
            "Should return 401 Unauthorized for invalid Authorization header: '{invalid_header}'"
        );

        // The response might be empty or contain error details
        // Let's be flexible about the response format since this is an auth middleware response
        let response_text = response.text().await.unwrap_or_default();

        // Just verify we got an unauthorized response - the exact format may vary
        println!("Response for invalid header '{invalid_header}': {response_text}");
    }
}

#[tokio::test]
#[serial]
async fn test_oauth_start_query_parameter_structure() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Setup fixtures
    let idp = IdpConnectFixtures::service().await;
    idp.mock_github_happy_arthur().await;
    idp.mock_gitlab_happy_alice().await;

    let providers = vec!["github", "gitlab"];

    for provider in providers {
        let response = client
            .get(format!("{base_url}/api/auth/{provider}/login"))
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

        let (_, params) =
            parse_redirect_url(location).expect("Should be able to parse redirect URL");

        // ✅ Verify all required OAuth2 parameters are present
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
                "Should have required OAuth2 parameter '{param}' for provider '{provider}'"
            );
            assert!(
                !params.get(param).unwrap().is_empty(),
                "OAuth2 parameter '{param}' should not be empty for provider '{provider}'"
            );
        }

        // ✅ Verify parameter values meet OAuth2 standards
        assert_eq!(
            params.get("response_type").unwrap(),
            "code",
            "response_type should be 'code' for authorization code flow"
        );

        // ✅ Verify scope contains expected values (depends on provider)
        let scope = params.get("scope").unwrap();
        if provider == "github" {
            assert!(
                scope.contains("user") || scope.contains("read:user"),
                "GitHub scope should include user permissions"
            );
        } else if provider == "gitlab" {
            assert!(
                scope.contains("read_user") || scope.contains("openid"),
                "GitLab scope should include user permissions"
            );
        }

        // ✅ Verify redirect_uri is properly URL encoded and contains correct callback path
        let redirect_uri = params.get("redirect_uri").unwrap();
        assert!(
            redirect_uri.contains(provider),
            "redirect_uri should point to correct callback endpoint for provider '{provider}'"
        );
        assert!(
            redirect_uri.ends_with("/callback"),
            "login redirect_uri must use the callback path for provider '{provider}'"
        );
        assert!(
            !redirect_uri.contains("relink-callback"),
            "login redirect_uri must not use relink-callback for provider '{provider}'"
        );
    }
}

#[tokio::test]
#[serial]
async fn idp_connect_rejects_invalid_hmac_with_401() {
    let idp = IdpConnectFixtures::service().await;
    idp.mock_github_happy_arthur().await;

    let connector = HttpIdpConnector::new(
        format!("{}/github-connect", idp.base_url()),
        "wrong-hmac-secret!!",
    )
    .expect("wrong secret still meets minimum length");

    let err = connector
        .exchange_code(
            "test_auth_code",
            "http://127.0.0.1:8081/iam/api/auth/github/callback",
        )
        .await
        .expect_err("mismatched HMAC must fail closed");

    assert_eq!(err, FederatedOAuthError::ExchangeCode);
}

#[tokio::test]
#[serial]
async fn oauth_start_connector_401_is_not_redirect() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let idp = IdpConnectFixtures::service().await;
    idp.mock_s2s_unauthorized("github", "/v1/authorize").await;

    let response = client
        .get(format!("{base_url}/api/auth/github/login"))
        .send()
        .await
        .expect("Failed to send request");

    assert_ne!(response.status(), 200);
    assert_ne!(response.status(), 303);
}
