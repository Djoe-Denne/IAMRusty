// Include common test utilities and fixtures
#[path = "common/mod.rs"] 
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::{get_test_server, TestFixture};
use fixtures::{GitHubFixtures, GitLabFixtures, DbFixtures};
use reqwest::Client;
use serde_json::Value;
use serial_test::serial;
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose};
use chrono::{Utc, Duration};
use jsonwebtoken::{encode, decode, Header, Algorithm, EncodingKey, DecodingKey, Validation};
use serde::{Serialize, Deserialize};

/// Create a common HTTP client for tests
fn create_test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client")
}

/// JWT claims for test tokens
#[derive(Debug, Serialize, Deserialize)]
struct TestClaims {
    sub: String,        // Subject (user ID)
    exp: usize,         // Expiration time (as UTC timestamp)
    iat: usize,         // Issued at (as UTC timestamp)
    jti: String,        // JWT ID (unique identifier for the token)
}

/// Generate a valid JWT token for testing
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

/// Generate an expired JWT token for testing
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

/// Generate a JWT token with invalid signature
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

// 👤 User Endpoint Tests
// 🔁 /me

#[tokio::test]
#[serial]
async fn test_get_user_returns_correct_info_when_token_is_valid() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
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
    
    // Create valid JWT token
    let secret = test_fixture.config().jwt.secret;
    let jwt_token = create_valid_jwt_token(user.id(), &secret);
    
    // Make request to /me endpoint
    let response = client
        .get(&format!("{}/api/me", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ✅ Should return 200 OK with user info
    assert_eq!(response.status(), 200, "Should return 200 OK for valid token");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON response");
    
    // ✅ Should contain correct user information
    assert_eq!(response_json["id"], user.id().to_string(), 
              "Should return correct user ID");
    assert_eq!(response_json["username"], "arthur", 
              "Should return correct username");
    assert_eq!(response_json["email"], "arthur@example.com", 
              "Should return primary email");
    assert!(response_json["avatar_url"].is_string() || response_json["avatar_url"].is_null(), 
           "Should have avatar_url field");
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_token_is_expired() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create expired JWT token
    let user_id = Uuid::new_v4();
    let secret = test_fixture.config().jwt.secret;
    let expired_token = create_expired_jwt_token(user_id, &secret);
    
    // Make request with expired token
    let response = client
        .get(&format!("{}/api/me", base_url))
        .header("Authorization", format!("Bearer {}", expired_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 401 Unauthorized for expired token
    assert_eq!(response.status(), 401, "Should return 401 for expired token");
    
    // The middleware returns plain StatusCode without JSON body
    let response_text = response.text().await.expect("Should be able to read response text");
    assert!(response_text.is_empty() || response_text.len() <= 50, 
           "Should return minimal response body (not JSON)");
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_token_is_malformed() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    let malformed_tokens = vec![
        ("invalid.jwt.token", 401),
        ("not.a.jwt", 401),  
        ("Bearer invalid_token", 401),
        ("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.invalid_payload.invalid_signature", 401),
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
        assert_eq!(response.status(), expected_status, 
                  "Should return {} for malformed token: '{}'", expected_status, malformed_token);
        
        if expected_status == 401 {
            // Middleware returns plain StatusCode for auth failures
            let response_text = response.text().await.expect("Should be able to read response text");
            assert!(response_text.is_empty() || response_text.len() <= 50, 
                   "Should return minimal response body for malformed token: '{}'", malformed_token);
        } else {
            // Service returns JSON error (this branch shouldn't be used anymore)
            let response_json: Value = response
                .json()
                .await
                .expect("Should return JSON error response");
            
            assert!(response_json["error"].is_object(), 
                   "Should return error object for malformed token: '{}'", malformed_token);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_token_has_invalid_signature() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create token with invalid signature
    let user_id = Uuid::new_v4();
    let invalid_token = create_invalid_signature_jwt_token(user_id);
    
    // Make request with invalid signature token
    let response = client
        .get(&format!("{}/api/me", base_url))
        .header("Authorization", format!("Bearer {}", invalid_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 401 Unauthorized for invalid signature (authentication failure)
    assert_eq!(response.status(), 401, "Should return 401 for invalid signature");
    
    // The middleware returns plain StatusCode without JSON body for auth failures
    let response_text = response.text().await.expect("Should be able to read response text");
    assert!(response_text.is_empty() || response_text.len() <= 50, 
           "Should return minimal response body (not JSON)");
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_no_authorization_header() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Make request without Authorization header
    let response = client
        .get(&format!("{}/api/me", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 401 Unauthorized for missing header
    assert_eq!(response.status(), 401, "Should return 401 for missing Authorization header");
    
    // The middleware returns plain StatusCode without JSON body
    let response_text = response.text().await.expect("Should be able to read response text");
    assert!(response_text.is_empty() || response_text.len() <= 50, 
           "Should return minimal response body (not JSON)");
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_authorization_header_format_is_invalid() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    let invalid_headers = vec![
        "Basic dXNlcjpwYXNzd29yZA==", // Basic auth instead of Bearer
        "Bearer", // Missing token
        "bearer token", // Wrong case
        "Token jwt_token", // Wrong scheme
        "jwt_token", // Missing Bearer prefix
    ];
    
    for invalid_header in invalid_headers {
        let response = client
            .get(&format!("{}/api/me", base_url))
            .header("Authorization", invalid_header)
            .send()
            .await
            .expect("Failed to send request");
        
        // ❌ Should return 401 Unauthorized for invalid header format
        assert_eq!(response.status(), 401, 
                  "Should return 401 for invalid header format: '{}'", invalid_header);
        
        // The middleware returns plain StatusCode without JSON body
        let response_text = response.text().await.expect("Should be able to read response text");
        assert!(response_text.is_empty() || response_text.len() <= 50, 
               "Should return minimal response body for invalid header: '{}'", invalid_header);
    }
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_404_when_user_not_found_in_database() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create valid JWT token for non-existent user
    let non_existent_user_id = Uuid::new_v4();
    let secret = test_fixture.config().jwt.secret;
    let jwt_token = create_valid_jwt_token(non_existent_user_id, &secret);
    
    // Make request with token for non-existent user
    let response = client
        .get(&format!("{}/api/me", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 404 Not Found when user not found
    assert_eq!(response.status(), 404, "Should return 404 when user not found");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON error response");
    
    assert!(response_json["error"].is_object(), 
           "Should return error object");
    assert_eq!(response_json["error"]["status"], 404, 
              "Error status should be 404");
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_correct_primary_email_when_user_has_multiple_emails() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create user with multiple emails
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    // Create multiple emails - secondary email first, then primary
    let secondary_email = DbFixtures::user_email()
        .arthur_secondary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create secondary email");
    
    let primary_email = DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");
    
    // Create valid JWT token
    let secret = test_fixture.config().jwt.secret;
    let jwt_token = create_valid_jwt_token(user.id(), &secret);
    
    // Make request to /me endpoint
    let response = client
        .get(&format!("{}/api/me", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ✅ Should return 200 OK with primary email
    assert_eq!(response.status(), 200, "Should return 200 OK");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON response");
    
    // ✅ Should return primary email, not secondary
    assert_eq!(response_json["email"], "arthur@example.com", 
              "Should return primary email");
    assert_ne!(response_json["email"], "arthur.secondary@example.com", 
              "Should not return secondary email");
}

#[tokio::test]
#[serial]
async fn test_get_user_handles_user_with_no_primary_email() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create user without any emails
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    // Create valid JWT token
    let secret = test_fixture.config().jwt.secret;
    let jwt_token = create_valid_jwt_token(user.id(), &secret);
    
    // Make request to /me endpoint
    let response = client
        .get(&format!("{}/api/me", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");
    
    // ✅ Should return 200 OK with null email
    assert_eq!(response.status(), 200, "Should return 200 OK even without email");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON response");
    
    // ✅ Should return null email when no primary email exists
    assert!(response_json["email"].is_null(), 
           "Should return null email when no primary email exists");
    assert_eq!(response_json["username"], "arthur", 
              "Should still return correct username");
}

#[tokio::test]
#[serial]
async fn test_get_user_concurrent_requests_with_same_token() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    
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
    
    // Create valid JWT token
    let secret = test_fixture.config().jwt.secret;
    let jwt_token = create_valid_jwt_token(user.id(), &secret);
    
    // Make multiple concurrent requests with the same token
    let mut handles = vec![];
    
    for i in 0..5 {
        let base_url = base_url.clone();
        let token = jwt_token.clone();
        
        let handle = tokio::spawn(async move {
            let client = create_test_client();
            let response = client
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
        assert_eq!(response_json["username"], "arthur", 
                  "Request {} should return correct username", request_id);
    }
}

#[tokio::test]
#[serial]
async fn test_get_user_security_jwt_claims_validation() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create user
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
    
    let secret = test_fixture.config().jwt.secret;
    
    // Test various invalid claim scenarios
    let test_cases = vec![
        // Missing 'sub' claim
        (serde_json::json!({
            "exp": (Utc::now() + Duration::hours(1)).timestamp(),
            "iat": Utc::now().timestamp(),
            "jti": Uuid::new_v4().to_string()
        }), "missing sub claim"),
        
        // Invalid 'sub' claim (not a UUID)
        (serde_json::json!({
            "sub": "invalid_user_id",
            "exp": (Utc::now() + Duration::hours(1)).timestamp(),
            "iat": Utc::now().timestamp(),
            "jti": Uuid::new_v4().to_string()
        }), "invalid sub claim"),
        
        // Missing 'exp' claim
        (serde_json::json!({
            "sub": user.id().to_string(),
            "iat": Utc::now().timestamp(),
            "jti": Uuid::new_v4().to_string()
        }), "missing exp claim"),
        
        // Missing 'iat' claim
        (serde_json::json!({
            "sub": user.id().to_string(),
            "exp": (Utc::now() + Duration::hours(1)).timestamp(),
            "jti": Uuid::new_v4().to_string()
        }), "missing iat claim"),
    ];
    
    for (claims, description) in test_cases {
        let header = Header::new(Algorithm::HS256);
        let encoding_key = EncodingKey::from_secret(secret.as_bytes());
        
        let invalid_token = encode(&header, &claims, &encoding_key)
            .expect("Failed to encode token");
        
        let response = client
            .get(&format!("{}/api/me", base_url))
            .header("Authorization", format!("Bearer {}", invalid_token))
            .send()
            .await
            .expect("Failed to send request");
        
        // ❌ Should return 401 for invalid claims (authentication failure)
        assert_eq!(response.status(), 401, 
                  "Should return 401 for {}", description);
    }
} 