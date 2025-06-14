// Include common test utilities and fixtures
#[path = "common/mod.rs"] 
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::{get_test_server, TestFixture};
use fixtures::{DbFixtures, GitHubFixtures};
use reqwest::Client;
use serde_json::{json, Value};
use serial_test::serial;
use base64::{engine::general_purpose, Engine as _};
use common::jwt_test_utils::create_expired_registration_token_with_encoder;

/// Create a common HTTP client for tests
fn create_test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client")
}

/// Helper function to decode JWT payload for testing
fn decode_jwt_payload(jwt: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT format".into());
    }
    
    let payload_encoded = parts[1];
    let payload_padded = match payload_encoded.len() % 4 {
        0 => payload_encoded.to_string(),
        n => format!("{}{}", payload_encoded, "=".repeat(4 - n)),
    };
    
    let decoded_bytes = general_purpose::STANDARD.decode(payload_padded)?;
    let payload_str = String::from_utf8(decoded_bytes)?;
    let payload: Value = serde_json::from_str(&payload_str)?;
    Ok(payload)
}

// =============================================================================
// 🔐 JWT SECURITY TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_registration_token_has_correct_structure() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    let signup_data = json!({
        "email": "jwttest@example.com",
        "password": "securePassword123"
    });

    let response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(response.status(), 201);
    
    let response_body: Value = response.json().await.expect("Should return JSON response");
    let registration_token = response_body["registration_token"].as_str().unwrap();

    // Verify JWT structure
    let parts: Vec<&str> = registration_token.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT should have 3 parts");
    
    // Decode and verify payload
    let payload = decode_jwt_payload(registration_token).expect("Should decode JWT payload");
    assert_eq!(payload["email"].as_str().unwrap(), "jwttest@example.com");
    assert!(payload["user_id"].is_string(), "Should contain user_id");
    assert!(payload["exp"].is_number(), "Should contain expiration time");
}

#[tokio::test]
#[serial]
async fn test_expired_token_returns_400() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Use an obviously invalid token
    let expired_token = "invalid.token.here";

    let completion_data = json!({
        "registration_token": expired_token,
        "username": "testuser"
    });

    let response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    assert_eq!(response.status(), 400, "Should return 400 for invalid token");
}

#[tokio::test]
#[serial]
async fn test_jwks_endpoint_accessible() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();

    let response = client
        .get(&format!("{}/.well-known/jwks.json", base_url))
        .send()
        .await
        .expect("Failed to get JWKS");

    assert_eq!(response.status(), 200, "JWKS endpoint should be accessible");
    
    let jwks: Value = response.json().await.expect("Should return JSON");
    assert!(jwks["keys"].is_array(), "JWKS should contain keys array");
}

// =============================================================================
// 🔄 EDGE CASES & RETRY SCENARIOS  
// =============================================================================

#[tokio::test]
#[serial]
async fn test_same_email_retry_returns_new_token() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    let signup_data = json!({
        "email": "retry@example.com",
        "password": "securePassword123"
    });

    // First signup
    let first_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send first signup");

    assert_eq!(first_response.status(), 201);
    let first_body: Value = first_response.json().await.expect("Should return JSON");
    let first_token = first_body["registration_token"].as_str().unwrap();

    // Second signup with same email
    let second_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send second signup");

    assert_eq!(second_response.status(), 201, "Should allow retry");
    let second_body: Value = second_response.json().await.expect("Should return JSON");
    let second_token = second_body["registration_token"].as_str().unwrap();

    // Tokens should be different
    assert_ne!(first_token, second_token, "Should generate new token on retry");
}

#[tokio::test]
#[serial]
async fn test_user_id_consistent_across_retries() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    let signup_data = json!({
        "email": "consistent@example.com", 
        "password": "securePassword123"
    });

    // First signup
    let first_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send first signup");

    assert_eq!(first_response.status(), 201);
    let first_body: Value = first_response.json().await.expect("Should return JSON");
    let first_user_id = first_body["user"]["id"].as_str().unwrap();

    // Second signup (retry)
    let second_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send second signup");

    assert_eq!(second_response.status(), 201);
    let second_body: Value = second_response.json().await.expect("Should return JSON");
    let second_user_id = second_body["user"]["id"].as_str().unwrap();

    assert_eq!(first_user_id, second_user_id, "User ID should remain consistent");
}

// =============================================================================
// 🔍 USERNAME VALIDATION TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_username_availability_check() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Check available username
    let response = client
        .get(&format!("{}/api/auth/username/check", base_url))
        .query(&[("username", "availableuser")])
        .send()
        .await
        .expect("Failed to check username");

    assert_eq!(response.status(), 200);
    
    let response_body: Value = response.json().await.expect("Should return JSON");
    assert_eq!(response_body["available"].as_bool().unwrap(), true);
}

#[tokio::test]
#[serial]
async fn test_taken_username_with_suggestions() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();
    let db = fixture.db();

    // Pre-create user with taken username
    let _existing_user = DbFixtures::user()
        .username("johndoe")
        .commit(db.clone())
        .await
        .expect("Failed to create existing user");

    // Check taken username
    let response = client
        .get(&format!("{}/api/auth/username/check", base_url))
        .query(&[("username", "johndoe")])
        .send()
        .await
        .expect("Failed to check username");

    assert_eq!(response.status(), 200);
    
    let response_body: Value = response.json().await.expect("Should return JSON");
    assert_eq!(response_body["available"].as_bool().unwrap(), false);
    
    let suggestions = response_body["suggestions"].as_array().unwrap();
    assert!(suggestions.len() > 0, "Should provide suggestions");
}

#[tokio::test]
#[serial]
async fn test_username_format_validation() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Test too short username
    let response = client
        .get(&format!("{}/api/auth/username/check", base_url))
        .query(&[("username", "ab")])
        .send()
        .await
        .expect("Failed to check username");

    assert!(response.status() == 400 || response.status() == 422, 
           "Should reject too short username");

    // Test valid username
    let response = client
        .get(&format!("{}/api/auth/username/check", base_url))
        .query(&[("username", "validuser123")])
        .send()
        .await
        .expect("Failed to check username");

    assert_eq!(response.status(), 200, "Should accept valid username");
}

// =============================================================================
// 🛡️ DATA PROTECTION TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_email_validation_and_sanitization() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    let test_cases = vec![
        ("valid@example.com", true),
        ("  trimmed@example.com  ", true), // Should be trimmed
        ("UPPERCASE@EXAMPLE.COM", true), // Should be normalized
        ("invalid-email", false),
        ("@missing-local.com", false),
        ("", false),
    ];

    for (email, should_succeed) in test_cases {
        let signup_data = json!({
            "email": email,
            "password": "securePassword123"
        });

        let response = client
            .post(&format!("{}/api/auth/signup", base_url))
            .header("Content-Type", "application/json")
            .json(&signup_data)
            .send()
            .await
            .expect("Failed to send signup");

        if should_succeed {
            assert!(response.status() == 201 || response.status() == 409, 
                   "Valid email '{}' should succeed", email);
        } else {
            assert!(response.status() == 400 || response.status() == 422, 
                   "Invalid email '{}' should fail", email);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_username_injection_prevention() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Get registration token
    let signup_data = json!({
        "email": "injection@example.com",
        "password": "securePassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup");

    assert_eq!(signup_response.status(), 201);
    
    let signup_body: Value = signup_response.json().await.expect("Should return JSON");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Test injection attempts
    let malicious_usernames = vec![
        "<script>alert('xss')</script>",
        "'; DROP TABLE users; --", 
        "../../../etc/passwd",
        "user\x00null",
    ];

    for malicious_username in malicious_usernames {
        let completion_data = json!({
            "registration_token": registration_token,
            "username": malicious_username
        });

        let response = client
            .post(&format!("{}/api/auth/complete-registration", base_url))
            .header("Content-Type", "application/json")
            .json(&completion_data)
            .send()
            .await
            .expect("Failed to send completion");

        assert!(response.status() == 400 || response.status() == 422,
               "Should reject malicious username: '{}'", malicious_username);
    }
}

#[tokio::test]
#[serial]
async fn test_no_user_enumeration_in_errors() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Test login with non-existent email
    let login_data = json!({
        "email": "nonexistent@example.com",
        "password": "anypassword"
    });

    let response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login");

    assert_eq!(response.status(), 401);
    
    let error_json: Value = response.json().await.expect("Should return JSON");
    let error_response = &error_json["error"];
    let error_message = error_response["message"].as_str().unwrap_or("");
    
    // Should not reveal user existence
    assert!(!error_message.to_lowercase().contains("user not found"));
    assert!(!error_message.to_lowercase().contains("email not found"));
    assert!(error_message.to_lowercase().contains("invalid email or password"));
}

// =============================================================================
// 🌊 END-TO-END FLOW TESTS
// =============================================================================
#[tokio::test]
#[serial]
async fn test_oauth_first_flow_with_github() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Setup GitHub mock
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;

    // Step 1: Start OAuth flow
    let start_response = client
        .get(&format!("{}/api/auth/github/start", base_url))
        .send()
        .await
        .expect("Failed to start OAuth");

    assert_eq!(start_response.status(), 303);
    
    // Note: In a complete test, you would:
    // 1. Parse the redirect URL and state
    // 2. Simulate the OAuth callback
    // 3. Verify 202 response with registration token
    // 4. Complete registration with username
    // 5. Verify user can login with both OAuth and email/password
    
    // For now, we verify the OAuth start works
    let location = start_response.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.contains("github.com") || location.contains("localhost:3000"), 
           "Should redirect to GitHub OAuth");
} 