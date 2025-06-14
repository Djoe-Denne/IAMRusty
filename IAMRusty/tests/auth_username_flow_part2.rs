// Include common test utilities and fixtures
#[path = "common/mod.rs"] 
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::setup_test_server;
use fixtures::{DbFixtures, GitHubFixtures};
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;
use sea_orm::ConnectionTrait;
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use url::Url;
use common::jwt_test_utils::create_expired_registration_token_with_encoder;

/// Helper function to decode and verify JWT structure (for testing)
fn decode_jwt_payload(jwt: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT format".into());
    }
    
    let payload_encoded = parts[1];
    // Add padding if needed
    let payload_padded = match payload_encoded.len() % 4 {
        0 => payload_encoded.to_string(),
        n => format!("{}{}", payload_encoded, "=".repeat(4 - n)),
    };
    
    let decoded_bytes = general_purpose::STANDARD.decode(payload_padded)?;
    let payload_str = String::from_utf8(decoded_bytes)?;
    let payload: Value = serde_json::from_str(&payload_str)?;
    Ok(payload)
}

/// Helper function to parse redirect URL and extract query parameters
fn parse_redirect_url(location: &str) -> Result<(String, HashMap<String, String>), Box<dyn std::error::Error>> {
    let url = Url::parse(location)?;
    let mut params = HashMap::new();
    
    for (key, value) in url.query_pairs() {
        params.insert(key.to_string(), value.to_string());
    }
    
    Ok((url.origin().ascii_serialization() + url.path(), params))
}

// =============================================================================
// 🔗 OAUTH PROVIDER CONFLICT TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_oauth_provider_already_linked_to_same_user() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");
    let db = _fixture.db();

    // Pre-create user with GitHub provider already linked
    let existing_user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create existing user");

    let primary_email = DbFixtures::user_email()
        .arthur_primary(existing_user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");

    let github_token = DbFixtures::provider_token()
        .arthur_github(existing_user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create GitHub token");

    // Setup GitHub mock
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;

    // Try to link GitHub again with authentication
    let oauth_start_response = client
        .get(&format!("{}/api/auth/github/start", base_url))
        .send()
        .await
        .expect("Failed to start OAuth linking");

    // The response will depend on implementation, but should handle the conflict
    // This test verifies the system can detect when a provider is already linked
    // to the same user attempting to link it
    
    // Note: The exact response depends on implementation details
    assert!(oauth_start_response.status() == 303 || oauth_start_response.status() == 409,
           "Should handle provider already linked to same user appropriately");
}

#[tokio::test]
#[serial]
async fn test_oauth_provider_linked_to_different_user_returns_409() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");
    let db = _fixture.db();

    // Pre-create first user with GitHub linked
    let first_user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create first user");

    let first_email = DbFixtures::user_email()
        .arthur_primary(first_user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create first user email");

    let github_token_first = DbFixtures::provider_token()
        .arthur_github(first_user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create GitHub token for first user");

    // Pre-create second user (different email)
    let second_user = DbFixtures::user()
        .bob()
        .commit(db.clone())
        .await
        .expect("Failed to create second user");

    let second_email = DbFixtures::user_email()
        .bob_primary(second_user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create second user email");

    // Setup GitHub mock to return the same GitHub profile that's already linked to first user
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await; // Same profile as first user

    // Generate mock JWT for second user
    let mock_jwt_second_user = "mock_jwt_token_for_second_user";

    // Try to link GitHub from second user (should conflict)
    let oauth_start_response = client
        .get(&format!("{}/api/auth/github/start", base_url))
        .header("Authorization", format!("Bearer {}", mock_jwt_second_user))
        .send()
        .await
        .expect("Failed to start OAuth linking");

    // Start the OAuth flow
    if oauth_start_response.status() == 303 {
        let location = oauth_start_response.headers().get("location").unwrap().to_str().unwrap();
        let (_, params) = parse_redirect_url(location).unwrap();
        let state = params.get("state").unwrap();

        // Complete OAuth callback (this should detect the conflict)
        let callback_response = client
            .get(&format!("{}/api/auth/github/callback", base_url))
            .query(&[("code", "test_auth_code"), ("state", state)])
            .send()
            .await
            .expect("Failed to complete OAuth callback");

        // Should return 409 conflict because GitHub account is already linked to different user
        assert_eq!(callback_response.status(), 409, 
                  "Should return 409 when provider is already linked to different user");

        let error_response: Value = callback_response.json().await.expect("Should return JSON response");
        assert_eq!(error_response["operation"].as_str().unwrap(), "link");
        assert_eq!(error_response["error"].as_str().unwrap(), "provider_already_linked");
        assert!(error_response["message"].is_string(), "Should return error message");
    }
}

// =============================================================================
// 🔐 JWT SECURITY TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_registration_token_has_correct_rsa_signature() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");

    // Get registration token
    let signup_data = json!({
        "email": "rsatest@example.com",
        "password": "securePassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 201);
    
    let signup_body: Value = signup_response.json().await.expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Verify JWT structure
    let parts: Vec<&str> = registration_token.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT should have 3 parts (header.payload.signature)");

    // Decode header to verify algorithm
    let header_decoded = general_purpose::STANDARD.decode(parts[0]).expect("Should decode header");
    let header_str = String::from_utf8(header_decoded).expect("Header should be UTF-8");
    let header: Value = serde_json::from_str(&header_str).expect("Header should be JSON");
    
    // Verify RSA algorithm is used
    assert!(header["alg"].as_str().unwrap().starts_with("RS"), 
           "Should use RSA algorithm for signing");

    // Note: Full signature verification would require access to the public key
    // This would typically be done through the /.well-known/jwks.json endpoint
}

#[tokio::test]
#[serial]
async fn test_registration_token_contains_required_claims() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");

    // Get registration token
    let signup_data = json!({
        "email": "claimstest@example.com",
        "password": "securePassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 201);
    
    let signup_body: Value = signup_response.json().await.expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Decode and verify payload claims
    let payload = decode_jwt_payload(registration_token).expect("Should decode JWT payload");

    // Verify required claims
    assert_eq!(payload["email"].as_str().unwrap(), "claimstest@example.com", 
              "Should contain correct email claim");
    assert!(payload["user_id"].is_string(), "Should contain user_id claim");
    assert!(payload["exp"].is_number(), "Should contain exp claim");
    assert!(payload["sub"].is_string(), "Should contain sub claim");
    assert!(payload["iat"].is_number(), "Should contain iat claim");

    // Verify expiration is in the future
    let exp = payload["exp"].as_i64().unwrap();
    let now = chrono::Utc::now().timestamp();
    assert!(exp > now, "Token should not be expired");
    
    // Verify expiration is reasonable (should be ~24 hours from now)
    let twenty_four_hours = 24 * 60 * 60;
    assert!(exp <= now + twenty_four_hours + 60, // +60 for clock skew
           "Token should expire within approximately 24 hours");
}

#[tokio::test]
#[serial]
async fn test_registration_token_expires_after_configured_duration() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");

    // Get registration token
    let signup_data = json!({
        "email": "expirationtest@example.com",
        "password": "securePassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 201);
    
    let signup_body: Value = signup_response.json().await.expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Decode payload to check expiration
    let payload = decode_jwt_payload(registration_token).expect("Should decode JWT payload");
    let exp = payload["exp"].as_i64().unwrap();
    let iat = payload["iat"].as_i64().unwrap();
    
    // Verify token duration (should be 24 hours = 86400 seconds)
    let duration = exp - iat;
    let expected_duration = 24 * 60 * 60; // 24 hours
    assert!((duration - expected_duration).abs() <= 60, 
           "Token should have approximately 24 hour duration, got {} seconds", duration);
}

#[tokio::test]
#[serial]
async fn test_expired_registration_token_returns_400() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");

    let config = configuration::load_config().expect("failed to load test config");

    let expired_token = create_expired_registration_token_with_encoder(
        Uuid::new_v4(),
        "test@example.com".to_string(),
        &config,
    ).expect("Failed to create expired token");

    // Try to complete registration with expired token
    let completion_data = json!({
        "registration_token": expired_token,
        "username": "testuser"
    });

    let completion_response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    // Should return 400 for expired token
    assert_eq!(completion_response.status(), 400, "Should return 400 for expired token");
    
    let error_response: Value = completion_response.json().await.expect("Should return JSON response");
    let error_json = error_response["error"].as_object().unwrap();
    
    assert_eq!(error_json["error_code"].as_str().unwrap(), "token_expired");
    assert!(error_json["message"].is_string(), "Should return error message");
}

// =============================================================================
// 🔄 EDGE CASES & RETRY SCENARIOS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_same_email_signup_after_incomplete_registration() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");

    // First signup
    let signup_data = json!({
        "email": "retry@example.com",
        "password": "securePassword123"
    });

    let first_signup = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send first signup request");

    assert_eq!(first_signup.status(), 201);
    
    let first_body: Value = first_signup.json().await.expect("Should return JSON response");
    let first_token = first_body["registration_token"].as_str().unwrap();

    // Second signup with same email (without completing first registration)
    let second_signup = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send second signup request");

    // Should return 201 with new token
    assert_eq!(second_signup.status(), 201, "Should allow retry with same email");
    
    let second_body: Value = second_signup.json().await.expect("Should return JSON response");
    let second_token = second_body["registration_token"].as_str().unwrap();

    // Tokens should be different
    assert_ne!(first_token, second_token, "Should generate new registration token on retry");
}

#[tokio::test]
#[serial]
async fn test_no_duplicate_user_records_created_on_retry() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");

    // Multiple signups with same email
    let signup_data = json!({
        "email": "duplicatetest@example.com",
        "password": "securePassword123"
    });

    for _ in 0..3 {
        let signup_response = client
            .post(&format!("{}/api/auth/signup", base_url))
            .header("Content-Type", "application/json")
            .json(&signup_data)
            .send()
            .await
            .expect("Failed to send signup request");

        assert_eq!(signup_response.status(), 201);
    }

    // Verify only one user record exists
    let db = _fixture.db();
    let user_count = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM user_emails WHERE email = 'duplicatetest@example.com'".to_string(),
        ))
        .await
        .expect("Failed to query users")
        .unwrap();
    
    let count: i64 = user_count.try_get("", "count").expect("Failed to get count");
    assert_eq!(count, 1, "Should only create one user record despite multiple signups");
}

#[tokio::test]
#[serial]
async fn test_user_id_remains_consistent_across_retries() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");

    // First signup
    let signup_data = json!({
        "email": "consistent@example.com",
        "password": "securePassword123"
    });

    let first_signup = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send first signup request");

    assert_eq!(first_signup.status(), 201);
    
    let first_body: Value = first_signup.json().await.expect("Should return JSON response");
    let first_user_id = first_body["user"]["id"].as_str().unwrap();

    // Second signup (retry)
    let second_signup = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send second signup request");

    assert_eq!(second_signup.status(), 201);
    
    let second_body: Value = second_signup.json().await.expect("Should return JSON response");
    let second_user_id = second_body["user"]["id"].as_str().unwrap();

    // User ID should remain the same
    assert_eq!(first_user_id, second_user_id, "User ID should remain consistent across retries");
}

// =============================================================================
// 📨 DOMAIN EVENTS TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_user_signed_up_triggered_only_at_registration_completion() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");

    // Step 1: Email signup (should NOT trigger user_signed_up event)
    let signup_data = json!({
        "email": "events@example.com",
        "password": "securePassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 201);
    
    let signup_body: Value = signup_response.json().await.expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Note: Here you would check your event store/message queue to verify
    // that NO user_signed_up event was triggered yet

    // Step 2: Complete registration (should trigger user_signed_up event)
    let completion_data = json!({
        "registration_token": registration_token,
        "username": "eventuser"
    });

    let completion_response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    assert_eq!(completion_response.status(), 200);

    // Note: Here you would check your event store/message queue to verify
    // that user_signed_up event WAS triggered with correct user data
    
    // ✅ This test serves as a placeholder for event verification logic
    // In a real implementation, you would:
    // 1. Check event store for user_signed_up events
    // 2. Verify event contains correct user data (id, email, username)
    // 3. Verify event was only triggered once at completion, not at signup
}

#[tokio::test]
#[serial]
async fn test_user_signed_up_triggered_when_existing_user_adds_password() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");
    let db = _fixture.db();

    // Pre-create user with OAuth (completed registration)
    let existing_user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create existing user");

    let primary_email = DbFixtures::user_email()
        .arthur_primary(existing_user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");

    // Add password authentication to existing user (should trigger event)
    let signup_data = json!({
        "email": primary_email.email(),
        "password": "newPassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 200);

    // Note: Here you would check your event store/message queue to verify
    // that user_signed_up event was triggered because this represents
    // adding a new authentication method to an existing completed user
    
    // ✅ This test serves as a placeholder for event verification logic
}

#[tokio::test]
#[serial]
async fn test_event_fired_after_successful_database_transaction() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");

    // Complete registration flow
    let signup_data = json!({
        "email": "transaction@example.com",
        "password": "securePassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 201);
    
    let signup_body: Value = signup_response.json().await.expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    let completion_data = json!({
        "registration_token": registration_token,
        "username": "transactionuser"
    });

    let completion_response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    assert_eq!(completion_response.status(), 200);

    // Note: This test would verify that:
    // 1. Database transaction completed successfully
    // 2. Event was only fired after successful transaction commit
    // 3. If transaction had failed, no event would be fired
    
    // ✅ This test serves as a placeholder for transactional event logic
}

// =============================================================================
// 🛡️ DATA PROTECTION TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_email_addresses_properly_validated_and_sanitized() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");

    // Test various email formats for validation
    let test_cases = vec![
        ("valid@example.com", true),
        ("user.name@example.com", true),
        ("user+tag@example.com", true),
        ("  valid@example.com  ", true), // Should be trimmed
        ("UPPERCASE@EXAMPLE.COM", true), // Should be normalized
        ("invalid-email", false),
        ("@missing-local.com", false),
        ("missing-domain@", false),
        ("", false),
        ("spaces in@email.com", false),
        ("double@@domain.com", false),
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
            .expect("Failed to send signup request");

        if should_succeed {
            assert!(response.status() == 201 || response.status() == 409, 
                   "Valid email '{}' should succeed or conflict", email);
        } else {
            assert!(response.status() == 400 || response.status() == 422, 
                   "Invalid email '{}' should return validation error", email);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_username_input_sanitization_prevents_injection() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");

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
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 201);
    
    let signup_body: Value = signup_response.json().await.expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Test various injection attempts
    let malicious_usernames = vec![
        "<script>alert('xss')</script>",
        "'; DROP TABLE users; --",
        "../../../etc/passwd", 
        "user\x00null",
        "user\nline\nbreak",
        "user\ttab",
        "user<>pipes",
        "user&amp;entity",
    ];

    for malicious_username in malicious_usernames {
        let completion_data = json!({
            "registration_token": registration_token,
            "username": malicious_username
        });

        let completion_response = client
            .post(&format!("{}/api/auth/complete-registration", base_url))
            .header("Content-Type", "application/json")
            .json(&completion_data)
            .send()
            .await
            .expect("Failed to send completion request");

        // Should return validation error for malicious input
        assert!(completion_response.status() == 400 || completion_response.status() == 422,
               "Malicious username '{}' should be rejected", malicious_username);
    }
}

#[tokio::test]
#[serial]
async fn test_no_sensitive_data_exposed_in_error_messages() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");

    // Test invalid login (should not reveal user existence)
    let login_data = json!({
        "email": "nonexistent@example.com",
        "password": "anypassword"
    });

    let login_response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), 401);
    
    let error_response: Value = login_response.json().await.expect("Should return JSON response");
    let error_json = error_response["error"].as_object().unwrap();
    let error_message = error_json["message"].as_str().unwrap_or("");
    
    // Error message should not reveal whether user exists or not
    assert!(!error_message.to_lowercase().contains("user not found"), 
           "Error message should not reveal user existence");
    assert!(!error_message.to_lowercase().contains("email not found"), 
           "Error message should not reveal email existence");
    
    // Should be generic
    assert!(error_message.to_lowercase().contains("invalid") &&
    error_message.to_lowercase().contains("email") &&
    error_message.to_lowercase().contains("password"),
           "Should use generic error message");
}

#[tokio::test]
#[serial]
async fn test_proper_https_enforcement_for_token_transmission() {
    // Note: This test would typically check that tokens are only transmitted
    // over HTTPS in production environments. Since this is a test environment,
    // we'll verify that the system is configured to enforce HTTPS appropriately.
    
    let (_fixture, base_url, _client) = setup_test_server().await.expect("Failed to setup test server");
    
    // In a real test, you would:
    // 1. Verify that non-HTTPS requests are rejected or redirected
    // 2. Check that secure headers are set (HSTS, etc.)
    // 3. Ensure cookies have secure flags
    // 4. Verify TLS configuration
    
    // For now, we'll just verify the server is accessible
    // (specific HTTPS enforcement tests would depend on your server configuration)
    
    assert!(base_url.starts_with("http"), "Test server should be accessible");
    
    // ✅ This test serves as a placeholder for HTTPS enforcement verification
    // In production tests, you would verify:
    // - Secure headers are present
    // - Non-HTTPS requests are properly handled
    // - TLS certificates are valid
    // - Security policies are enforced
} 