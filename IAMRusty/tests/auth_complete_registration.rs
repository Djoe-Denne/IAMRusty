// Include common test utilities and fixtures
#[path = "common/mod.rs"]
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;
mod utils;

use base64::{Engine as _, engine::general_purpose};
use common::setup_test_server;
use fixtures::DbFixtures;
use reqwest::Client;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use serial_test::serial;
use uuid;

// =============================================================================
// ✅ COMPLETE REGISTRATION ENDPOINT TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_complete_registration_success() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // First get a registration token via signup
    let signup_data = json!({
        "email": "completion@example.com",
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

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Complete registration
    let completion_data = json!({
        "registration_token": registration_token,
        "username": "completeduser"
    });

    let completion_response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    // Verify successful response
    let status = completion_response.status();
    if status != 200 {
        let error_body: Value = completion_response
            .json()
            .await
            .expect("Should return JSON response");
        panic!(
            "Expected 200 but got {}. Error response: {}",
            status,
            serde_json::to_string_pretty(&error_body).unwrap()
        );
    }

    let response_body: Value = completion_response
        .json()
        .await
        .expect("Should return JSON response");

    // Verify response structure
    assert_eq!(
        response_body["user"]["username"].as_str().unwrap(),
        "completeduser"
    );
    assert_eq!(
        response_body["user"]["email"].as_str().unwrap(),
        "completion@example.com"
    );
    assert!(
        response_body["user"]["id"].is_string(),
        "Should return user ID"
    );
    assert!(
        response_body["access_token"].is_string(),
        "Should return access token"
    );
    assert!(
        response_body["expires_in"].is_number(),
        "Should return expires_in"
    );
    assert!(
        response_body["refresh_token"].is_string(),
        "Should return refresh token"
    );
}

#[tokio::test]
#[serial]
async fn test_complete_registration_invalid_token_signature() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Use invalid token with bad signature
    let invalid_token = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJlbWFpbCI6InRlc3RAZXhhbXBsZS5jb20iLCJ1c2VyX2lkIjoiMTIzIiwiZXhwIjo5OTk5OTk5OTk5LCJpYXQiOjE2MDAwMDAwMDAsInN1YiI6IjEyMyJ9.invalid_signature";

    let completion_data = json!({
        "registration_token": invalid_token,
        "username": "testuser"
    });

    let completion_response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    // Should return 400 for invalid signature
    assert_eq!(
        completion_response.status(),
        400,
        "Should return 400 for invalid token signature"
    );

    let error_response: Value = completion_response
        .json()
        .await
        .expect("Should return JSON response");
    assert_eq!(
        error_response["error"]["error_code"].as_str().unwrap(),
        "invalid_token"
    );
    assert_eq!(
        error_response["error"]["message"].as_str().unwrap(),
        "Invalid registration token signature"
    );
}

#[tokio::test]
#[serial]
async fn test_complete_registration_expired_token() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create an expired registration token using the utility function
    let user_id = uuid::Uuid::new_v4();
    let email = "test@example.com".to_string();
    let config = _fixture.config();

            let expired_token = utils::jwt::create_expired_registration_token_with_encoder(
        user_id, email, &config,
    )
    .expect("Failed to create expired registration token");

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
    assert_eq!(
        completion_response.status(),
        400,
        "Should return 400 for expired token"
    );

    let error_response: Value = completion_response
        .json()
        .await
        .expect("Should return JSON response");
    assert_eq!(
        error_response["error"]["error_code"].as_str().unwrap(),
        "token_expired"
    );
    assert_eq!(
        error_response["error"]["message"].as_str().unwrap(),
        "Registration token has expired"
    );
}

#[tokio::test]
#[serial]
async fn test_complete_registration_username_already_taken() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Pre-create user with taken username
    let _existing_user = DbFixtures::user()
        .username("takenusername")
        .commit(db.clone())
        .await
        .expect("Failed to create existing user");

    // Get registration token
    let signup_data = json!({
        "email": "newuser@example.com",
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

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Try to complete with taken username
    let completion_data = json!({
        "registration_token": registration_token,
        "username": "takenusername"
    });

    let completion_response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    // Should return 409 for username conflict
    assert_eq!(
        completion_response.status(),
        409,
        "Should return 409 for taken username"
    );
}

#[tokio::test]
#[serial]
async fn test_complete_registration_invalid_username_format() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Get registration token
    let signup_data = json!({
        "email": "formattest@example.com",
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

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Test various invalid username formats
    let long_username = "a".repeat(51);
    let invalid_usernames = vec![
        "ab",           // Too short (< 3 chars)
        &long_username, // Too long (> 50 chars)
        "user@name",    // Invalid character (@)
        "user name",    // Space
        "user.name",    // Dot
        "",             // Empty string
        "user<>pipes",  // Invalid characters
        "user&entity",  // Invalid character
    ];

    for invalid_username in invalid_usernames {
        let completion_data = json!({
            "registration_token": registration_token,
            "username": invalid_username
        });

        let completion_response = client
            .post(&format!("{}/api/auth/complete-registration", base_url))
            .header("Content-Type", "application/json")
            .json(&completion_data)
            .send()
            .await
            .expect("Failed to send completion request");

        // Should return 422 for invalid format
        assert_eq!(
            completion_response.status(),
            422,
            "Should return 422 for invalid username format: '{}'",
            invalid_username
        );
    }
}

#[tokio::test]
#[serial]
async fn test_complete_registration_valid_username_formats() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Valid username formats that should be accepted
    let max_length_username = "a".repeat(50);
    let valid_usernames = vec![
        "abc",                // Minimum length (3 chars)
        "user123",            // Alphanumeric
        "user_name",          // Underscore
        "user-name",          // Dash/hyphen
        "User123",            // Mixed case
        "123user",            // Starting with number
        &max_length_username, // Maximum length (50 chars)
    ];

    for valid_username in valid_usernames {
        // Get fresh registration token for each test
        let signup_data = json!({
            "email": format!("{}@example.com", valid_username),
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

        let signup_body: Value = signup_response
            .json()
            .await
            .expect("Should return JSON response");
        let registration_token = signup_body["registration_token"].as_str().unwrap();

        // Complete registration with valid username
        let completion_data = json!({
            "registration_token": registration_token,
            "username": valid_username
        });

        let completion_response = client
            .post(&format!("{}/api/auth/complete-registration", base_url))
            .header("Content-Type", "application/json")
            .json(&completion_data)
            .send()
            .await
            .expect("Failed to send completion request");

        // Should accept valid username format
        assert_eq!(
            completion_response.status(),
            200,
            "Should accept valid username format: '{}'",
            valid_username
        );

        let response_body: Value = completion_response
            .json()
            .await
            .expect("Should return JSON response");
        assert_eq!(
            response_body["user"]["username"].as_str().unwrap(),
            valid_username
        );
    }
}

#[tokio::test]
#[serial]
async fn test_complete_registration_malformed_request() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test missing registration_token
    let missing_token_data = json!({
        "username": "testuser"
    });

    let response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&missing_token_data)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        422,
        "Should return 422 for missing registration_token"
    );

    // Test missing username
    let missing_username_data = json!({
        "registration_token": "some_token"
    });

    let response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&missing_username_data)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        422,
        "Should return 422 for missing username"
    );

    // Test empty request body
    let response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&json!({}))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        422,
        "Should return 422 for empty request body"
    );

    // Test invalid JSON
    let response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .body("invalid json")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 400, "Should return 400 for invalid JSON");
}

#[tokio::test]
#[serial]
async fn test_registration_token_single_use() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Get registration token
    let signup_data = json!({
        "email": "singleuse@example.com",
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

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Complete registration (first use - should succeed)
    let completion_data = json!({
        "registration_token": registration_token,
        "username": "firstuse"
    });

    let first_completion = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send first completion request");

    assert_eq!(first_completion.status(), 200, "First use should succeed");

    // Try to use the same token again (should fail)
    let second_completion_data = json!({
        "registration_token": registration_token,
        "username": "seconduse"
    });

    let second_completion = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&second_completion_data)
        .send()
        .await
        .expect("Failed to send second completion request");

    // Should fail because token is already used
    assert_eq!(second_completion.status(), 400, "Second use should fail");

    let error_response: Value = second_completion
        .json()
        .await
        .expect("Should return JSON response");
    assert_eq!(
        error_response["error"]["error_code"].as_str().unwrap(),
        "invalid_token"
    );
}

#[tokio::test]
#[serial]
async fn test_complete_registration_updates_user_record() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Get registration token
    let signup_data = json!({
        "email": "updatetest@example.com",
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

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();
    let user_id = signup_body["user"]["id"].as_str().unwrap();

    // Complete registration
    let completion_data = json!({
        "registration_token": registration_token,
        "username": "updateduser"
    });

    let completion_response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    assert_eq!(completion_response.status(), 200);

    // Verify user record was updated in database
    let db = _fixture.db();
    let user_record = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT username FROM users WHERE id = '{}'", user_id),
        ))
        .await
        .expect("Failed to query user")
        .expect("User should exist");

    let username: String = user_record
        .try_get("", "username")
        .expect("Should have username");
    assert_eq!(
        username, "updateduser",
        "Username should be updated in database"
    );
}

#[tokio::test]
#[serial]
async fn test_complete_registration_user_can_login_afterward() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Complete full registration flow
    let signup_data = json!({
        "email": "loginafter@example.com",
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

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Complete registration
    let completion_data = json!({
        "registration_token": registration_token,
        "username": "loginuser"
    });

    let completion_response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    assert_eq!(completion_response.status(), 200);

    // Get verification token from database
    let verification_record = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT verification_token FROM user_email_verification WHERE email = 'loginafter@example.com'".to_string(),
        ))
        .await
        .expect("Failed to query verification token")
        .unwrap();

    let verification_token: String = verification_record
        .try_get("", "verification_token")
        .expect("Failed to get verification token");

    // Verify email before attempting login
    let verify_data = json!({
        "email": "loginafter@example.com",
        "verification_token": verification_token
    });

    let verify_response = client
        .post(&format!("{}/api/auth/verify", base_url))
        .header("Content-Type", "application/json")
        .json(&verify_data)
        .send()
        .await
        .expect("Failed to send verify request");

    assert_eq!(
        verify_response.status(),
        200,
        "Email verification should succeed"
    );

    // Now try to login with the completed and verified user
    let login_data = json!({
        "email": "loginafter@example.com",
        "password": "securePassword123"
    });

    let login_response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login request");

    // Should be able to login successfully after completion and verification
    assert_eq!(
        login_response.status(),
        200,
        "Should be able to login after completing registration and verifying email"
    );

    let login_body: Value = login_response
        .json()
        .await
        .expect("Should return JSON response");
    assert_eq!(
        login_body["user"]["username"].as_str().unwrap(),
        "loginuser"
    );
    assert_eq!(
        login_body["user"]["email"].as_str().unwrap(),
        "loginafter@example.com"
    );
    assert!(
        login_body["access_token"].is_string(),
        "Should return access token"
    );
    assert!(
        login_body["refresh_token"].is_string(),
        "Should return refresh token"
    );
}
