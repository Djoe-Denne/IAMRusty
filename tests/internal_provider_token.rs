// Include common test utilities and fixtures
#[path = "common/mod.rs"] 
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::{get_test_server, TestFixture};
use fixtures::DbFixtures;
use reqwest::Client;
use serde_json::Value;
use serial_test::serial;
use uuid::Uuid;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use chrono::{Utc, Duration};
use serde::{Serialize, Deserialize};

/// Create a common HTTP client for tests
fn create_test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client")
}

/// JWT claims structure for testing
#[derive(Debug, Serialize, Deserialize)]
struct TestClaims {
    sub: String,        // Subject (user ID)
    exp: usize,         // Expiration time (as UTC timestamp)
    iat: usize,         // Issued at (as UTC timestamp)
    jti: String,        // JWT ID (unique identifier for the token)
}

/// Create a valid JWT token for testing
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
    
    encode(&header, &claims, &encoding_key)
        .expect("Failed to encode JWT token")
}

/// Create an expired JWT token for testing
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
    
    encode(&header, &claims, &encoding_key)
        .expect("Failed to encode JWT token")
}

/// Create a JWT token with invalid signature for testing
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
    
    encode(&header, &claims, &encoding_key)
        .expect("Failed to encode JWT token")
}

// 🔐 Internal Provider Token Endpoint Tests
// 🔁 POST /internal/{provider}/token

#[tokio::test]
#[serial]
async fn test_internal_provider_token_github_success_returns_access_token() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
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
    
    // Create valid JWT token for authentication
    let secret = test_fixture.config().jwt.get_secret_string().expect("Failed to get secret string");
    let jwt_token = create_valid_jwt_token(user.id(), &secret);
    
    // Make request to internal provider token endpoint
    let response = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ✅ Should return 200 OK with provider token
    assert_eq!(response.status(), 200, "Should return 200 OK for valid request");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON response");
    
    // ✅ Should contain access token and expiration
    assert!(response_json["access_token"].is_string(), 
           "Response should contain access token");
    assert!(response_json["expires_in"].is_number() || response_json["expires_in"].is_null(), 
           "Response should contain expires_in field");
    
    let access_token = response_json["access_token"].as_str()
        .expect("access_token should be a string");
    assert!(!access_token.is_empty(), "Access token should not be empty");
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_gitlab_success_returns_access_token() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
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
    
    // Create valid JWT token for authentication
    let secret = test_fixture.config().jwt.get_secret_string().expect("Failed to get secret string");
    let jwt_token = create_valid_jwt_token(user.id(), &secret);
    
    // Make request to internal provider token endpoint
    let response = client
        .post(&format!("{}/internal/gitlab/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ✅ Should return 200 OK with provider token
    assert_eq!(response.status(), 200, "Should return 200 OK for valid GitLab request");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON response");
    
    // ✅ Should contain access token and expiration
    assert!(response_json["access_token"].is_string(), 
           "Response should contain access token");
    assert!(response_json["expires_in"].is_number() || response_json["expires_in"].is_null(), 
           "Response should contain expires_in field");
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_401_when_no_authorization_header() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Make request without Authorization header
    let response = client
        .post(&format!("{}/internal/github/token", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 401 Unauthorized for missing header
    assert_eq!(response.status(), 401, "Should return 401 for missing Authorization header");
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_401_when_token_is_expired() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create expired JWT token
    let user_id = Uuid::new_v4();
    let secret = test_fixture.config().jwt.get_secret_string().expect("Failed to get secret string");
    let expired_token = create_expired_jwt_token(user_id, &secret);
    
    // Make request with expired token
    let response = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", expired_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 401 Unauthorized for expired token
    assert_eq!(response.status(), 401, "Should return 401 for expired token");
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_401_when_token_has_invalid_signature() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create token with invalid signature
    let user_id = Uuid::new_v4();
    let invalid_token = create_invalid_signature_jwt_token(user_id);
    
    // Make request with invalid signature token
    let response = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", invalid_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 401 Unauthorized for invalid signature
    assert_eq!(response.status(), 401, "Should return 401 for invalid signature");
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_422_when_provider_is_unsupported() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create user in database
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    // Create valid JWT token for authentication
    let secret = test_fixture.config().jwt.get_secret_string().expect("Failed to get secret string");
    let jwt_token = create_valid_jwt_token(user.id(), &secret);
    
    // Make request with unsupported provider
    let response = client
        .post(&format!("{}/internal/unsupported/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 422 Unprocessable Entity for validation error (invalid provider name)
    assert_eq!(response.status(), 422, "Should return 422 for validation error on unsupported provider");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON error response");
    
    // ✅ Should contain validation error for provider_name field
    assert!(response_json["provider_name"].is_array(), 
           "Should return validation errors for provider_name field");
    
    let provider_name_errors = response_json["provider_name"].as_array()
        .expect("provider_name should be an array");
    assert!(!provider_name_errors.is_empty(), "Should have at least one validation error");
    
    let first_error = &provider_name_errors[0];
    assert_eq!(first_error["code"], "invalid_provider", 
              "Should return invalid_provider error code");
    assert_eq!(first_error["message"], "Invalid provider name", 
              "Should return appropriate error message");
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_404_when_no_token_for_provider() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create user in database but NO provider token
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    // Create valid JWT token for authentication
    let secret = test_fixture.config().jwt.get_secret_string().expect("Failed to get secret string");
    let jwt_token = create_valid_jwt_token(user.id(), &secret);
    
    // Make request for GitHub token when user has no GitHub token
    let response = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 404 Not Found when no token available
    assert_eq!(response.status(), 404, "Should return 404 when no token available for provider");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON error response");
    
    assert!(response_json["error"].is_object(), 
           "Should return error object");
    assert_eq!(response_json["error"]["error_code"], "no_token_available", 
              "Should return no_token_available error code");
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_401_when_user_not_found() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create valid JWT token for a user that doesn't exist in database
    let non_existent_user_id = Uuid::new_v4();
    let secret = test_fixture.config().jwt.get_secret_string().expect("Failed to get secret string");
    let jwt_token = create_valid_jwt_token(non_existent_user_id, &secret);
    
    // Make request with token for non-existent user
    let response = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 401 Unauthorized to avoid revealing user existence (security)
    assert_eq!(response.status(), 401, "Should return 401 when user not found to avoid information leakage");
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_returns_401_when_malformed_token() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
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
        assert_eq!(response.status(), 401, 
                  "Should return 401 for malformed token: '{}'", malformed_token);
    }
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_case_insensitive_providers() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
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
    
    // Create valid JWT token for authentication
    let secret = test_fixture.config().jwt.get_secret_string().expect("Failed to get secret string");
    let jwt_token = create_valid_jwt_token(user.id(), &secret);
    
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
        assert_eq!(response.status(), 200, 
                  "Should return 200 for provider case variation: '{}'", provider);
    }
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_different_users_different_tokens() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
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
    
    let secret = test_fixture.config().jwt.get_secret_string().expect("Failed to get secret string");
    
    // Test user 1
    let jwt_token1 = create_valid_jwt_token(user1.id(), &secret);
    let response1 = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token1))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(response1.status(), 200);
    let response1_json: Value = response1.json().await.expect("Should return JSON");
    let token1 = response1_json["access_token"].as_str().expect("Should have access_token");
    
    // Test user 2
    let jwt_token2 = create_valid_jwt_token(user2.id(), &secret);
    let response2 = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token2))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(response2.status(), 200);
    let response2_json: Value = response2.json().await.expect("Should return JSON");
    let token2 = response2_json["access_token"].as_str().expect("Should have access_token");
    
    // ✅ Should return different tokens for different users
    assert_ne!(token1, token2, "Different users should get different provider tokens");
    assert_eq!(token1, "github_token_arthur_123", "User 1 should get their specific token");
    assert_eq!(token2, "github_token_bob_456", "User 2 should get their specific token");
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_user_with_multiple_providers() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create user with both GitHub and GitLab tokens
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    let _github_token = DbFixtures::provider_token()
        .github(user.id())
        .access_token("github_token_123")
        .commit(db.clone())
        .await
        .expect("Failed to create GitHub token");
    
    let _gitlab_token = DbFixtures::provider_token()
        .gitlab(user.id())
        .access_token("gitlab_token_456")
        .commit(db.clone())
        .await
        .expect("Failed to create GitLab token");
    
    let secret = test_fixture.config().jwt.get_secret_string().expect("Failed to get secret string");
    let jwt_token = create_valid_jwt_token(user.id(), &secret);
    
    // Test GitHub endpoint
    let github_response = client
        .post(&format!("{}/internal/github/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send GitHub request");
    
    assert_eq!(github_response.status(), 200);
    let github_json: Value = github_response.json().await.expect("Should return JSON");
    let github_token = github_json["access_token"].as_str().expect("Should have access_token");
    
    // Test GitLab endpoint
    let gitlab_response = client
        .post(&format!("{}/internal/gitlab/token", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send GitLab request");
    
    assert_eq!(gitlab_response.status(), 200);
    let gitlab_json: Value = gitlab_response.json().await.expect("Should return JSON");
    let gitlab_token = gitlab_json["access_token"].as_str().expect("Should have access_token");
    
    // ✅ Should return correct tokens for each provider
    assert_eq!(github_token, "github_token_123", "Should return correct GitHub token");
    assert_eq!(gitlab_token, "gitlab_token_456", "Should return correct GitLab token");
    assert_ne!(github_token, gitlab_token, "GitHub and GitLab tokens should be different");
}

#[tokio::test]
#[serial]
async fn test_internal_provider_token_concurrent_requests_same_user() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    
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
    
    let secret = test_fixture.config().jwt.get_secret_string().expect("Failed to get secret string");
    let jwt_token = create_valid_jwt_token(user.id(), &secret);
    
    // Make 5 concurrent requests
    let mut handles = vec![];
    for _ in 0..5 {
        let base_url = base_url.clone();
        let jwt_token = jwt_token.clone();
        
        let handle = tokio::spawn(async move {
            let client = create_test_client();
            client
                .post(&format!("{}/internal/github/token", base_url))
                .header("Authorization", format!("Bearer {}", jwt_token))
                .send()
                .await
                .expect("Failed to send request")
        });
        
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    let responses = futures::future::join_all(handles).await;
    
    // ✅ All requests should succeed
    for (i, response_result) in responses.into_iter().enumerate() {
        let response = response_result.expect("Task should complete successfully");
        assert_eq!(response.status(), 200, "Request {} should succeed", i);
        
        let response_json: Value = response.json().await.expect("Should return JSON");
        assert!(response_json["access_token"].is_string(), 
               "Request {} should return access token", i);
    }
} 