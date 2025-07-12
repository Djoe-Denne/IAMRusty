// Include common test utilities and fixtures

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;
mod utils;

use chrono::{Duration, Utc};
use common::{create_test_client, setup_test_server};
use iam_configuration::{load_config_part, JwtConfig};
use fixtures::{DbFixtures, GitHubFixtures, GitLabFixtures};
use reqwest::Client;
use serde_json::Value;
use serial_test::serial;
use uuid::Uuid;

/// Create a valid JWT token for testing using the proper JWT service
fn create_valid_jwt_token(user_id: Uuid, config: &JwtConfig) -> String {
    utils::jwt::create_valid_jwt_token_with_encoder(user_id, config)
        .expect("Failed to create valid JWT token")
}

/// Create an expired JWT token for testing using the proper JWT service
fn create_expired_jwt_token(user_id: Uuid, config: &JwtConfig) -> String {
    utils::jwt::create_expired_jwt_token_with_encoder(user_id, config)
        .expect("Failed to create expired JWT token")
}

/// Create an invalid JWT token for testing using the proper JWT service
fn create_invalid_signature_jwt_token(user_id: Uuid, config: &JwtConfig) -> String {
    utils::jwt::create_invalid_jwt_token_with_encoder(user_id, config)
        .expect("Failed to create invalid JWT token")
}

// 👤 User Endpoint Tests
// 🔁 /me

#[tokio::test]
#[serial]
async fn test_get_user_returns_correct_info_when_token_is_valid() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Pre-create user and email
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let primary_email = DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");

    // Create valid JWT token using proper configuration
    let jwt_token = create_valid_jwt_token(user.id(), &load_config_part::<JwtConfig>("jwt").expect("Failed to load JWT config"));

    // Make request to /me endpoint
    let response = client
        .get(&format!("{}/api/me", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 OK with user info
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for valid token"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    // ✅ Should contain correct user information
    assert_eq!(
        response_json["id"],
        user.id().to_string(),
        "Should return correct user ID"
    );
    assert_eq!(
        response_json["username"], "arthur",
        "Should return correct username"
    );
    assert_eq!(
        response_json["email"], "arthur@example.com",
        "Should return primary email"
    );
    assert!(
        response_json["avatar_url"].is_string() || response_json["avatar_url"].is_null(),
        "Should have avatar_url field"
    );
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_token_is_expired() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create expired JWT token
    let user_id = Uuid::new_v4();
    let expired_token = create_expired_jwt_token(user_id, &load_config_part::<JwtConfig>("jwt").expect("Failed to load JWT config"));

    // Make request with expired token
    let response = client
        .get(&format!("{}/api/me", base_url))
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

    // The middleware returns plain StatusCode without JSON body
    let response_text = response
        .text()
        .await
        .expect("Should be able to read response text");
    
    assert!(
        response_text.is_empty() || response_text.len() <= 50,
        "Should return minimal response body (not JSON)"
    );
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_token_is_malformed() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    let malformed_tokens = vec![
        ("invalid.jwt.token", 401),
        ("not.a.jwt", 401),
        ("Bearer invalid_token", 401),
        (
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.invalid_payload.invalid_signature",
            401,
        ),
        ("", 401), // Empty token is caught by middleware
        ("malformed_token_without_dots", 401),
    ];

    for (malformed_token, expected_status) in malformed_tokens {
        let response = client
            .get(&format!("{}/api/me", base_url))
            .header("Authorization", format!("Bearer {}", malformed_token))
            .send()
            .await
            .expect("Failed to send request");

        // Should return 401 for malformed tokens (authentication failures)
        assert_eq!(
            response.status(),
            expected_status,
            "Should return {} for malformed token: '{}'",
            expected_status,
            malformed_token
        );

        if expected_status == 401 {
            // Middleware returns plain StatusCode for auth failures
            let response_text = response
                .text()
                .await
                .expect("Should be able to read response text");
            assert!(
                response_text.is_empty() || response_text.len() <= 50,
                "Should return minimal response body for malformed token: '{}'",
                malformed_token
            );
        } else {
            // Service returns JSON error (this branch shouldn't be used anymore)
            let response_json: Value = response
                .json()
                .await
                .expect("Should return JSON error response");

            assert!(
                response_json["error"].is_object(),
                "Should return error object for malformed token: '{}'",
                malformed_token
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_no_authorization_header() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Make request without Authorization header
    let response = client
        .get(&format!("{}/api/me", base_url))
        .send()
        .await
        .expect("Failed to send request");

    // ❌ Should return 401 Unauthorized for missing header
    assert_eq!(
        response.status(),
        401,
        "Should return 401 for missing Authorization header"
    );

    // The middleware returns plain StatusCode without JSON body
    let response_text = response
        .text()
        .await
        .expect("Should be able to read response text");
    assert!(
        response_text.is_empty() || response_text.len() <= 50,
        "Should return minimal response body (not JSON)"
    );
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_authorization_header_format_is_invalid() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    let invalid_headers = vec![
        "Basic dXNlcjpwYXNzd29yZA==", // Basic auth instead of Bearer
        "Bearer",                     // Missing token
        "bearer token",               // Wrong case
        "Token jwt_token",            // Wrong scheme
        "jwt_token",                  // Missing Bearer prefix
    ];

    for invalid_header in invalid_headers {
        let response = client
            .get(&format!("{}/api/me", base_url))
            .header("Authorization", invalid_header)
            .send()
            .await
            .expect("Failed to send request");

        // ❌ Should return 401 Unauthorized for invalid header format
        assert_eq!(
            response.status(),
            401,
            "Should return 401 for invalid header format: '{}'",
            invalid_header
        );

        // The middleware returns plain StatusCode without JSON body
        let response_text = response
            .text()
            .await
            .expect("Should be able to read response text");
        assert!(
            response_text.is_empty() || response_text.len() <= 50,
            "Should return minimal response body for invalid header: '{}'",
            invalid_header
        );
    }
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_user_not_found_in_database() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create valid JWT token for non-existent user
    let non_existent_user_id = Uuid::new_v4();
    let jwt_token = create_valid_jwt_token(non_existent_user_id, &load_config_part::<JwtConfig>("jwt").expect("Failed to load JWT config"));

    // Make request with token for non-existent user
    let response = client
        .get(&format!("{}/api/me", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // Re-make the request to get the JSON response
    let response = client
        .get(&format!("{}/api/me", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        401,
        "Should return 401 when user not found"
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
        response_json["error"]["error_code"],
        "user_not_found".to_string(),
        "Error code should be user_not_found"
    );
    assert_eq!(
        response_json["error"]["status"], 401,
        "Error status should be 401"
    );
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_correct_primary_email_when_user_has_multiple_emails() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Pre-create user with multiple emails
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    // Create multiple emails - secondary email first, then primary
    let _secondary_email = DbFixtures::user_email()
        .arthur_secondary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create secondary email");

    let _primary_email = DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");

    // Create valid JWT token
    let jwt_token = create_valid_jwt_token(user.id(), &&load_config_part::<JwtConfig>("jwt").expect("Failed to load JWT config"));

    // Make request to /me endpoint
    let response = client
        .get(&format!("{}/api/me", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 OK with primary email
    assert_eq!(response.status(), 200, "Should return 200 OK");

    let response_json: Value = response.json().await.expect("Should return JSON response");

    // ✅ Should return primary email, not secondary
    assert_eq!(
        response_json["email"], "arthur@example.com",
        "Should return primary email"
    );
    assert_ne!(
        response_json["email"], "arthur.secondary@example.com",
        "Should not return secondary email"
    );
}

#[tokio::test]
#[serial]
async fn test_get_user_handles_user_with_no_primary_email() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Pre-create user without any emails
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    // Create valid JWT token
    let jwt_token = create_valid_jwt_token(user.id(), &&load_config_part::<JwtConfig>("jwt").expect("Failed to load JWT config"));

    // Make request to /me endpoint
    let response = client
        .get(&format!("{}/api/me", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 OK with null email
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK even without email"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    // ✅ Should return null email when no primary email exists
    assert!(
        response_json["email"].is_null(),
        "Should return null email when no primary email exists"
    );
    assert_eq!(
        response_json["username"], "arthur",
        "Should still return correct username"
    );
}

#[tokio::test]
#[serial]
async fn test_get_user_concurrent_requests_with_same_token() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Pre-create user and email
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let _primary_email = DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");

    // Create valid JWT token
    let jwt_token = create_valid_jwt_token(user.id(), &&load_config_part::<JwtConfig>("jwt").expect("Failed to load JWT config"));

    // Make multiple concurrent requests with the same token
    let mut handles = vec![];

    for i in 0..5 {
        let base_url = base_url.clone();
        let token = jwt_token.clone();

        let handle = tokio::spawn(async move {
            let client2 = create_test_client();
            let response = client2
                .get(&format!("{}/api/me", base_url))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .expect("Failed to send request");

            (i, response.status(), response.json::<Value>().await)
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let (request_id, status, response_result) = handle.await.expect("Request failed");

        // ✅ All requests should succeed
        assert_eq!(status, 200, "Request {} should return 200 OK", request_id);

        let response_json = response_result.expect("Should return JSON response");
        assert_eq!(
            response_json["username"], "arthur",
            "Request {} should return correct username",
            request_id
        );
    }
}

#[tokio::test]
#[serial]
async fn test_get_user_security_jwt_claims_validation() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Pre-create user
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let _primary_email = DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");

    // Get the JWT configuration to create custom tokens
    let config = load_config_part::<JwtConfig>("jwt").expect("Failed to load JWT config");
    let jwt_algorithm_config = config
        .create_jwt_algorithm()
        .expect("Failed to create JWT algorithm");

    // Get algorithm and keys for manual token creation
    let (algorithm, encoding_key) = match jwt_algorithm_config {
        iam_domain::JwtAlgorithm::HS256(secret) => (
            jsonwebtoken::Algorithm::HS256,
            jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
        ),
        iam_domain::JwtAlgorithm::RS256(key_pair) => (
            jsonwebtoken::Algorithm::RS256,
            jsonwebtoken::EncodingKey::from_rsa_pem(key_pair.private_key.as_bytes())
                .expect("Failed to create RSA encoding key"),
        ),
    };

    // Test various invalid claim scenarios
    let test_cases = vec![
        // Missing 'sub' claim
        (
            serde_json::json!({
                "exp": (Utc::now() + Duration::hours(1)).timestamp(),
                "iat": Utc::now().timestamp(),
                "jti": Uuid::new_v4().to_string()
            }),
            "missing sub claim",
        ),
        // Invalid 'sub' claim (not a UUID)
        (
            serde_json::json!({
                "sub": "invalid_user_id",
                "exp": (Utc::now() + Duration::hours(1)).timestamp(),
                "iat": Utc::now().timestamp(),
                "jti": Uuid::new_v4().to_string()
            }),
            "invalid sub claim",
        ),
        // Missing 'exp' claim
        (
            serde_json::json!({
                "sub": user.id().to_string(),
                "iat": Utc::now().timestamp(),
                "jti": Uuid::new_v4().to_string()
            }),
            "missing exp claim",
        ),
        // Missing 'iat' claim
        (
            serde_json::json!({
                "sub": user.id().to_string(),
                "exp": (Utc::now() + Duration::hours(1)).timestamp(),
                "jti": Uuid::new_v4().to_string()
            }),
            "missing iat claim",
        ),
    ];

    for (claims, description) in test_cases {
        let header = jsonwebtoken::Header::new(algorithm);

        let invalid_token =
            jsonwebtoken::encode(&header, &claims, &encoding_key).expect("Failed to encode token");

        let response = client
            .get(&format!("{}/api/me", base_url))
            .header("Authorization", format!("Bearer {}", invalid_token))
            .send()
            .await
            .expect("Failed to send request");

        // ❌ Should return 401 for invalid claims (authentication failure)
        assert_eq!(
            response.status(),
            401,
            "Should return 401 for {}",
            description
        );
    }
}
