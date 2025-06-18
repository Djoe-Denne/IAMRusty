// Include common test utilities and fixtures
mod common;
mod fixtures;
mod utils;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use chrono::{Duration, Utc};
use utils::jwt::{
    create_expired_jwt_token_with_encoder, create_invalid_jwt_token_with_encoder,
    create_valid_jwt_token_with_encoder,
};
use common::setup_test_server;
use fixtures::DbFixtures;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::Client;
use serial_test::serial;
use tokio;

/// Create a common HTTP client for tests
fn create_test_client() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
}

/// JWT claims structure for testing
#[derive(Debug, Serialize, Deserialize)]
struct TestClaims {
    sub: String, // Subject (user ID)
    exp: usize,  // Expiration time (as UTC timestamp)
    iat: usize,  // Issued at (as UTC timestamp)
    jti: String, // JWT ID (unique identifier for the token)
}

/// Create a valid JWT token for testing (deprecated - use create_valid_jwt_token_with_encoder)
#[deprecated(note = "Use create_valid_jwt_token_with_encoder from jwt_test_utils instead")]
fn create_valid_jwt_token(user_id: Uuid, secret: &str) -> String {
    let now = Utc::now();
    let exp = now + Duration::hours(1);

    let claims = TestClaims {
        sub: user_id.to_string(),
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
        jti: Uuid::new_v4().to_string(),
    };

    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());

    encode(&header, &claims, &encoding_key).expect("Failed to encode JWT token")
}

/// Create an expired JWT token for testing (deprecated - use create_expired_jwt_token_with_encoder)
#[deprecated(note = "Use create_expired_jwt_token_with_encoder from jwt_test_utils instead")]
fn create_expired_jwt_token(user_id: Uuid, secret: &str) -> String {
    let now = Utc::now();
    let exp = now - Duration::hours(1); // Expired 1 hour ago

    let claims = TestClaims {
        sub: user_id.to_string(),
        exp: exp.timestamp() as usize,
        iat: (now - Duration::hours(2)).timestamp() as usize,
        jti: Uuid::new_v4().to_string(),
    };

    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());

    encode(&header, &claims, &encoding_key).expect("Failed to encode JWT token")
}

/// Create a JWT token with invalid signature for testing (deprecated - use create_invalid_signature_jwt_token_with_encoder)
#[deprecated(
    note = "Use create_invalid_signature_jwt_token_with_encoder from jwt_test_utils instead"
)]
fn create_invalid_signature_jwt_token(user_id: Uuid) -> String {
    let now = Utc::now();
    let exp = now + Duration::hours(1);

    let claims = TestClaims {
        sub: user_id.to_string(),
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
        jti: Uuid::new_v4().to_string(),
    };

    let header = Header::new(Algorithm::HS256);
    let wrong_secret = "wrong_secret_for_invalid_signature";
    let encoding_key = EncodingKey::from_secret(wrong_secret.as_bytes());

    encode(&header, &claims, &encoding_key).expect("Failed to encode JWT token")
}

// 🔐 Internal Provider Token Endpoint Tests
// 🔁 POST /internal/{provider}/token

#[tokio::test]
#[serial]
async fn test_internal_provider_token_github_success_returns_access_token() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user in database
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    // Create GitHub provider token for the user
    let _provider_token = DbFixtures::provider_token()
        .arthur_github(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create provider token");

    // Create valid JWT token for authentication using the new encoder-based method
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Make request to internal provider token endpoint
    let response = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 OK with provider token
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for valid request"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    // ✅ Should contain access token and expiration
    assert!(
        response_json["access_token"].is_string(),
        "Response should contain access token"
    );
    assert!(
        response_json["expires_in"].is_number() || response_json["expires_in"].is_null(),
        "Response should contain expires_in field"
    );

    let access_token = response_json["access_token"]
        .as_str()
        .expect("access_token should be a string");
    assert!(!access_token.is_empty(), "Access token should not be empty");
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_gitlab_success_returns_access_token() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user in database
    let user = DbFixtures::user()
        .alice()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    // Create GitLab provider token for the user
    let _provider_token = DbFixtures::provider_token()
        .alice_gitlab(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create provider token");

    // Create valid JWT token for authentication using the new encoder-based method
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Make request to internal provider token endpoint
    let response = client
        .post(&format!("{}/internal/gitlab/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 OK with provider token
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for valid GitLab request"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    // ✅ Should contain access token and expiration
    assert!(
        response_json["access_token"].is_string(),
        "Response should contain access token"
    );
    assert!(
        response_json["expires_in"].is_number() || response_json["expires_in"].is_null(),
        "Response should contain expires_in field"
    );
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_401_when_no_authorization_header() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Make request without Authorization header
    let response = client
        .post(&format!("{}/internal/github/token", base_url))
        .send()
        .await
        .expect("Failed to send request");

    // ❌ Should return 401 Unauthorized for missing header
    assert_eq!(
        response.status(),
        401,
        "Should return 401 for missing Authorization header"
    );
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_401_when_token_is_expired() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create expired JWT token using the new encoder-based method
    let user_id = Uuid::new_v4();
    let expired_token = create_expired_jwt_token_with_encoder(user_id, &_fixture.config())
        .expect("Failed to create expired JWT token");

    // Make request with expired token
    let response = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", expired_token))
        .send()
        .await
        .expect("Failed to send request");

    // ❌ Should return 401 Unauthorized for expired token
    assert_eq!(
        response.status(),
        401,
        "Should return 401 for expired token"
    );
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_401_when_token_has_invalid_signature() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create JWT token with invalid signature using the new encoder-based method
    let user_id = Uuid::new_v4();
    let invalid_token = create_invalid_jwt_token_with_encoder(user_id, &_fixture.config())
        .expect("Failed to create invalid signature JWT token");

    // Make request with invalid signature token
    let response = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", invalid_token))
        .send()
        .await
        .expect("Failed to send request");

    // ❌ Should return 401 Unauthorized for invalid signature
    assert_eq!(
        response.status(),
        401,
        "Should return 401 for invalid signature"
    );
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_422_when_provider_is_unsupported() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create valid JWT token using the new encoder-based method
    let user_id = Uuid::new_v4();
    let jwt_token = create_valid_jwt_token_with_encoder(user_id, &_fixture.config())
        .expect("Failed to create JWT token");

    // Test unsupported providers
    let unsupported_providers = vec!["facebook", "twitter", "linkedin", "invalid"];

    for provider in unsupported_providers {
        let response = client
            .post(&format!("{}/internal/{}/token", base_url, provider))
            .header("Authorization", format!("Bearer {}", jwt_token))
            .send()
            .await
            .expect("Failed to send request");

        // ❌ Should return 422 Unprocessable Entity for unsupported provider
        assert_eq!(
            response.status(),
            422,
            "Should return 422 for unsupported provider: '{}'",
            provider
        );
    }
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_404_when_no_token_for_provider() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user in database but NO provider token
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    // Create valid JWT token for authentication using the new encoder-based method
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Make request for GitHub token when user has no GitHub token
    let response = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ❌ Should return 404 Not Found when no token available
    assert_eq!(
        response.status(),
        404,
        "Should return 404 when no token available for provider"
    );

    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(
        response_json["error"].is_object(),
        "Should return error object"
    );
    assert_eq!(
        response_json["error"]["error_code"], "no_token_for_provider",
        "Should return no_token_for_provider error code"
    );
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_401_when_user_not_found() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create valid JWT token for a user that doesn't exist in database using the new encoder-based method
    let non_existent_user_id = Uuid::new_v4();
    let jwt_token = create_valid_jwt_token_with_encoder(non_existent_user_id, &_fixture.config())
        .expect("Failed to create JWT token");

    // Make request with token for non-existent user
    let response = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ❌ Should return 401 Unauthorized to avoid revealing user existence (security)
    assert_eq!(
        response.status(),
        401,
        "Should return 401 when user not found to avoid information leakage"
    );
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_401_when_malformed_token() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let malformed_tokens = vec![
        "invalid.jwt.token",
        "not.a.jwt",
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.invalid_payload.invalid_signature",
        "",
        "malformed_token_without_dots",
    ];

    for malformed_token in malformed_tokens {
        let response = client
            .post(&format!("{}/internal/github/token", base_url))
            .header("Authorization", format!("Bearer {}", malformed_token))
            .send()
            .await
            .expect("Failed to send request");

        // ❌ Should return 401 for malformed tokens
        assert_eq!(
            response.status(),
            401,
            "Should return 401 for malformed token: '{}'",
            malformed_token
        );
    }
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_case_insensitive_providers() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with GitHub token
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let _provider_token = DbFixtures::provider_token()
        .arthur_github(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create provider token");

    // Create valid JWT token for authentication using the new encoder-based method
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Test different case variations of GitHub
    let provider_variations = vec!["github", "GitHub", "GITHUB"];

    for provider in provider_variations {
        let response = client
            .post(&format!("{}/internal/{}/token", base_url, provider))
            .header("Authorization", format!("Bearer {}", jwt_token))
            .send()
            .await
            .expect("Failed to send request");

        // ✅ Should work for all case variations
        assert_eq!(
            response.status(),
            200,
            "Should return 200 for provider case variation: '{}'",
            provider
        );
    }
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_different_users_different_tokens() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create two users with different GitHub tokens
    let user1 = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user1");

    let user2 = DbFixtures::user()
        .bob()
        .commit(db.clone())
        .await
        .expect("Failed to create user2");

    let _token1 = DbFixtures::provider_token()
        .arthur_github(user1.id())
        .access_token("github_token_arthur_123")
        .commit(db.clone())
        .await
        .expect("Failed to create token1");

    let _token2 = DbFixtures::provider_token()
        .bob_github(user2.id())
        .access_token("github_token_bob_456")
        .commit(db.clone())
        .await
        .expect("Failed to create token2");

    // Test user 1 using the new encoder-based method
    let jwt_token1 = create_valid_jwt_token_with_encoder(user1.id(), &_fixture.config())
        .expect("Failed to create JWT token for user1");
    let response1 = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token1))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response1.status(), 200);
    let response1_json: Value = response1.json().await.expect("Should return JSON");
    let token1 = response1_json["access_token"]
        .as_str()
        .expect("Should have access_token");

    // Test user 2 using the new encoder-based method
    let jwt_token2 = create_valid_jwt_token_with_encoder(user2.id(), &_fixture.config())
        .expect("Failed to create JWT token for user2");
    let response2 = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token2))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response2.status(), 200);
    let response2_json: Value = response2.json().await.expect("Should return JSON");
    let token2 = response2_json["access_token"]
        .as_str()
        .expect("Should have access_token");

    // ✅ Should return different tokens for different users
    assert_ne!(
        token1, token2,
        "Different users should have different provider tokens"
    );
    assert_eq!(
        token1, "github_token_arthur_123",
        "User 1 should get Arthur's token"
    );
    assert_eq!(
        token2, "github_token_bob_456",
        "User 2 should get Bob's token"
    );
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_user_with_multiple_providers() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with both GitHub and GitLab tokens
    let user = DbFixtures::user()
        .charlie()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let _github_token = DbFixtures::provider_token()
        .charlie_github(user.id())
        .access_token("charlie_github_token_789")
        .commit(db.clone())
        .await
        .expect("Failed to create GitHub token");

    let _gitlab_token = DbFixtures::provider_token()
        .charlie_gitlab(user.id())
        .access_token("charlie_gitlab_token_xyz")
        .commit(db.clone())
        .await
        .expect("Failed to create GitLab token");

    // Create valid JWT token for authentication using the new encoder-based method
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Test GitHub token retrieval
    let github_response = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send GitHub request");

    assert_eq!(github_response.status(), 200);
    let github_json: Value = github_response.json().await.expect("Should return JSON");
    assert_eq!(github_json["access_token"], "charlie_github_token_789");

    // Test GitLab token retrieval
    let gitlab_response = client
        .post(&format!("{}/internal/gitlab/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send GitLab request");

    assert_eq!(gitlab_response.status(), 200);
    let gitlab_json: Value = gitlab_response.json().await.expect("Should return JSON");
    assert_eq!(gitlab_json["access_token"], "charlie_gitlab_token_xyz");
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_concurrent_requests_same_user() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with GitHub token
    let user = DbFixtures::user()
        .diana()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let _provider_token = DbFixtures::provider_token()
        .diana_github(user.id())
        .access_token("diana_github_concurrent_token")
        .commit(db.clone())
        .await
        .expect("Failed to create provider token");

    // Create valid JWT token for authentication using the new encoder-based method
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Make multiple concurrent requests
    let concurrent_requests = 5;
    let mut handles = Vec::new();

    for i in 0..concurrent_requests {
        let client_clone = client.clone();
        let base_url_clone = base_url.clone();
        let jwt_token_clone = jwt_token.clone();

        let handle = tokio::spawn(async move {
            let response = client_clone
                .post(&format!("{}/internal/github/token", base_url_clone))
                .header("Authorization", format!("Bearer {}", jwt_token_clone))
                .send()
                .await
                .expect(&format!("Failed to send concurrent request {}", i));

            (i, response)
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let (request_id, response) = handle.await.expect("Concurrent request failed");
        assert_eq!(
            response.status(),
            200,
            "Concurrent request {} should return 200",
            request_id
        );

        let response_json: Value = response.json().await.expect(&format!(
            "Concurrent request {} should return JSON",
            request_id
        ));
        assert_eq!(
            response_json["access_token"], "diana_github_concurrent_token",
            "Concurrent request {} should return correct token",
            request_id
        );
    }
}
