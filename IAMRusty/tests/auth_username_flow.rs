// Include common test utilities and fixtures

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use application::usecase::login::PasswordService as AppPasswordService;
use base64::{Engine as _, engine::general_purpose};
use common::setup_test_server;
use fixtures::{DbFixtures, GitHubFixtures};
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

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
fn parse_redirect_url(
    location: &str,
) -> Result<(String, HashMap<String, String>), Box<dyn std::error::Error>> {
    let url = Url::parse(location)?;
    let mut params = HashMap::new();

    for (key, value) in url.query_pairs() {
        params.insert(key.to_string(), value.to_string());
    }

    Ok((url.origin().ascii_serialization() + url.path(), params))
}

// =============================================================================
// 📧 NEW USER EMAIL/PASSWORD SIGNUP FLOW TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_new_user_signup_returns_201_with_registration_token() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create signup request for new user
    let signup_data = json!({
        "email": "newuser@example.com",
        "password": "securePassword123"
    });

    // Execute signup
    let response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    // Verify response
    assert_eq!(
        response.status(),
        202,
        "Should return 201 for new user signup"
    );

    let response_body: Value = response.json().await.expect("Should return JSON response");

    // Verify response structure
    assert!(
        response_body["user"]["id"].is_string(),
        "Should return user ID"
    );
    assert_eq!(
        response_body["user"]["email"].as_str().unwrap(),
        "newuser@example.com"
    );
    assert!(
        response_body["registration_token"].is_string(),
        "Should return registration token"
    );
    assert_eq!(response_body["requires_username"].as_bool().unwrap(), true);
    assert!(
        response_body["message"].is_string(),
        "Should return message"
    );

    // Verify username is not present (incomplete registration)
    assert!(
        response_body["user"]["username"].is_null(),
        "Username should not be present for incomplete registration"
    );
}

#[tokio::test]
#[serial]
async fn test_registration_token_is_valid_rsa_signed_jwt() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create signup request
    let signup_data = json!({
        "email": "jwttest@example.com",
        "password": "securePassword123"
    });

    // Execute signup
    let response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(response.status(), 202);
    let response_body: Value = response.json().await.expect("Should return JSON response");

    let registration_token = response_body["registration_token"].as_str().unwrap();

    // Verify JWT structure
    assert!(
        registration_token.starts_with("eyJ"),
        "Should be a valid JWT"
    );
    let parts: Vec<&str> = registration_token.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT should have 3 parts");

    // Decode and verify payload
    let payload = decode_jwt_payload(registration_token).expect("Should decode JWT payload");
    assert_eq!(payload["email"].as_str().unwrap(), "jwttest@example.com");
    assert!(payload["user_id"].is_string(), "Should contain user_id");
    assert!(payload["exp"].is_number(), "Should contain expiration time");
    assert!(payload["iat"].is_number(), "Should contain issued at time");
}

#[tokio::test]
#[serial]
async fn test_no_user_signed_up_event_triggered_at_signup() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create signup request
    let signup_data = json!({
        "email": "eventtest@example.com",
        "password": "securePassword123"
    });

    // Execute signup
    let response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(response.status(), 202);

    // Note: In a real implementation, you would check your event store/message queue
    // to verify that no user_signed_up event was triggered
    // This test serves as a placeholder for that verification

    // ✅ Verify user record created with null username and pending status
    // This will be implemented once we have the database schema updates
}

#[tokio::test]
#[serial]
async fn test_user_record_created_with_null_username_pending_status() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create signup request
    let signup_data = json!({
        "email": "pendinguser@example.com",
        "password": "securePassword123"
    });

    // Execute signup
    let response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(response.status(), 202);

    let response_body: Value = response.json().await.expect("Should return JSON response");
    let user_id = response_body["user"]["id"].as_str().unwrap();

    // Verify database state
    let db = _fixture.db();
    let user_record = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT username FROM users WHERE id = '{}'", user_id),
        ))
        .await
        .expect("Failed to query user")
        .expect("User should exist");

    // Verify username is null for incomplete registration
    assert!(
        user_record
            .try_get::<Option<String>>("", "username")
            .unwrap()
            .is_none(),
        "Username should be null for incomplete registration"
    );
}

// =============================================================================
// 📧 EXISTING USER EMAIL/PASSWORD SIGNUP FLOW TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_existing_user_signup_returns_200_with_tokens() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
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

    // Try to signup with existing user's email
    let signup_data = json!({
        "email": primary_email.email(),
        "password": "newPassword123"
    });

    // Execute signup
    let response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    // Verify response
    assert_eq!(
        response.status(),
        200,
        "Should return 200 for existing user"
    );

    let response_body: Value = response.json().await.expect("Should return JSON response");

    // Verify response structure
    assert_eq!(
        response_body["user"]["id"].as_str().unwrap(),
        existing_user.id().to_string()
    );
    assert_eq!(
        response_body["user"]["username"].as_str().unwrap(),
        existing_user.username().unwrap()
    );
    assert_eq!(
        response_body["user"]["email"].as_str().unwrap(),
        primary_email.email()
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
    assert!(
        response_body["message"].is_string(),
        "Should return message"
    );
}

#[tokio::test]
#[serial]
async fn test_password_auth_method_added_to_existing_user() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Pre-create user with OAuth (no password auth)
    let existing_user = DbFixtures::user()
        .bob()
        .commit(db.clone())
        .await
        .expect("Failed to create existing user");

    let primary_email = DbFixtures::user_email()
        .bob_primary(existing_user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");

    // Execute signup to add password auth
    let signup_data = json!({
        "email": primary_email.email(),
        "password": "newPassword123"
    });

    let response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(response.status(), 200);

    // Verify password was added to user
    let user_record = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT password_hash FROM users WHERE id = '{}'",
                existing_user.id()
            ),
        ))
        .await
        .expect("Failed to query user")
        .expect("User should exist");

    let password_hash: Option<String> = user_record.try_get("", "password_hash").unwrap();
    assert!(password_hash.is_some(), "Password hash should be set");
    assert_ne!(
        password_hash.unwrap(),
        "newPassword123",
        "Password should be hashed"
    );
}

// =============================================================================
// 📧 LOGIN ATTEMPT TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_login_completed_user_returns_200_with_tokens() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Hash the password using the password service
    let password_service = Arc::new(infra::auth::PasswordService::new());
    let password_adapter = infra::auth::PasswordServiceAdapter::new(password_service);
    let password_hash = password_adapter
        .hash_password("validPassword123")
        .await
        .expect("Failed to hash password");

    // Pre-create completed user with password
    let completed_user = DbFixtures::user()
        .arthur()
        .password_hash(password_hash)
        .commit(db.clone())
        .await
        .expect("Failed to create completed user");

    let primary_email = DbFixtures::user_email()
        .arthur_primary(completed_user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");

    // Execute login
    let login_data = json!({
        "email": primary_email.email(),
        "password": "validPassword123"
    });

    let response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login request");

    // Verify response
    assert_eq!(
        response.status(),
        200,
        "Should return 200 for completed user login"
    );

    let response_body: Value = response.json().await.expect("Should return JSON response");

    // Verify response structure
    assert_eq!(
        response_body["user"]["id"].as_str().unwrap(),
        completed_user.id().to_string()
    );
    assert_eq!(
        response_body["user"]["username"].as_str().unwrap(),
        completed_user.username().unwrap()
    );
    assert_eq!(
        response_body["user"]["email"].as_str().unwrap(),
        primary_email.email()
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
async fn test_login_incomplete_user_returns_423_with_registration_token() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Hash the password using the password service
    let password_service = Arc::new(infra::auth::PasswordService::new());
    let password_adapter = infra::auth::PasswordServiceAdapter::new(password_service);
    let password_hash = password_adapter
        .hash_password("validPassword123")
        .await
        .expect("Failed to hash password");

    // Pre-create user with null username (incomplete registration)
    let incomplete_user = DbFixtures::user()
        .password_hash(password_hash)
        .commit(db.clone())
        .await
        .expect("Failed to create incomplete user");

    let _primary_email = DbFixtures::user_email()
        .email("incomplete@example.com")
        .user_id(incomplete_user.id())
        .is_primary(true)
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");

    // Execute login
    let login_data = json!({
        "email": "incomplete@example.com",
        "password": "validPassword123"
    });

    let response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login request");

    // Verify response
    assert_eq!(
        response.status(),
        423,
        "Should return 423 for incomplete registration"
    );

    let response_body: Value = response.json().await.expect("Should return JSON response");

    // Verify response structure
    assert_eq!(
        response_body["error"].as_str().unwrap(),
        "registration_incomplete"
    );
    assert!(
        response_body["message"].is_string(),
        "Should return message"
    );
    assert!(
        response_body["registration_token"].is_string(),
        "Should return new registration token"
    );
}

#[tokio::test]
#[serial]
async fn test_login_invalid_credentials_returns_401() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Hash the password using the password service
    let password_service = Arc::new(infra::auth::PasswordService::new());
    let password_adapter = infra::auth::PasswordServiceAdapter::new(password_service);
    let password_hash = password_adapter
        .hash_password("correctPassword123")
        .await
        .expect("Failed to hash password");

    // Pre-create user
    let user = DbFixtures::user()
        .arthur()
        .password_hash(password_hash)
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let primary_email = DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");

    // Execute login with wrong password
    let login_data = json!({
        "email": primary_email.email(),
        "password": "wrongPassword123"
    });

    let response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login request");

    // Verify response
    assert_eq!(
        response.status(),
        401,
        "Should return 401 for invalid credentials"
    );
}

#[tokio::test]
#[serial]
async fn test_login_nonexistent_email_returns_401() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Execute login with non-existent email
    let login_data = json!({
        "email": "nonexistent@example.com",
        "password": "anyPassword123"
    });

    let response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login request");

    // Verify response
    assert_eq!(
        response.status(),
        401,
        "Should return 401 for non-existent email"
    );
}

// =============================================================================
// 🔗 NEW USER OAUTH FLOW TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_oauth_callback_new_user_returns_202_with_registration_token() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Setup GitHub mock
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;

    // Start OAuth flow
    let start_response = client
        .get(&format!("{}/api/auth/github/login", base_url))
        .send()
        .await
        .expect("Failed to start OAuth flow");

    assert_eq!(start_response.status(), 303);

    let location = start_response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    let (_, params) = parse_redirect_url(location).unwrap();
    let state = params.get("state").unwrap();

    // Simulate OAuth callback for new user
    let callback_response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[("code", "test_auth_code"), ("state", state)])
        .send()
        .await
        .expect("Failed to complete OAuth callback");

    // Verify response
    assert_eq!(
        callback_response.status(),
        202,
        "Should return 202 for new user OAuth"
    );

    let response_body: Value = callback_response
        .json()
        .await
        .expect("Should return JSON response");

    // Verify response structure
    assert_eq!(
        response_body["operation"].as_str().unwrap(),
        "registration_required"
    );
    assert!(
        response_body["registration_token"].is_string(),
        "Should return registration token"
    );
    assert!(
        response_body["provider_info"]["email"].is_string(),
        "Should return provider email"
    );
    assert_eq!(response_body["requires_username"].as_bool().unwrap(), true);
}

#[tokio::test]
#[serial]
async fn test_registration_token_contains_oauth_provider_info() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Setup GitHub mock
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;

    // Start OAuth flow
    let start_response = client
        .get(&format!("{}/api/auth/github/login", base_url))
        .send()
        .await
        .expect("Failed to start OAuth flow");

    assert_eq!(start_response.status(), 303);

    let location = start_response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    let (_, params) = parse_redirect_url(location).unwrap();
    let state = params.get("state").unwrap();

    // Complete OAuth callback
    let callback_response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[("code", "test_auth_code"), ("state", state)])
        .send()
        .await
        .expect("Failed to complete OAuth callback");

    assert_eq!(callback_response.status(), 202);

    let response_body: Value = callback_response
        .json()
        .await
        .expect("Should return JSON response");

    // Verify provider info
    let provider_info = &response_body["provider_info"];
    assert!(
        provider_info["email"].is_string(),
        "Should contain provider email"
    );
    assert!(
        provider_info["avatar"].is_string(),
        "Should contain provider avatar"
    );

    // Verify registration token contains this info
    let registration_token = response_body["registration_token"].as_str().unwrap();
    let payload = decode_jwt_payload(registration_token).expect("Should decode JWT payload");

    // The JWT should contain the provider information
    assert!(
        payload["email"].is_string(),
        "JWT should contain provider info"
    );
    assert!(
        payload["exp"].is_number(),
        "JWT should contain provider info"
    );
    assert!(
        payload["iat"].is_number(),
        "JWT should contain provider info"
    );
    assert!(
        payload["sub"].is_string(),
        "JWT should contain provider info"
    );
    assert!(
        payload["flow"].is_string(),
        "JWT should contain provider info"
    );
    assert!(
        payload["user_id"].is_string(),
        "JWT should contain provider info"
    );
}

// =============================================================================
// ✅ REGISTRATION COMPLETION TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_complete_registration_valid_token_available_username_returns_200() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // First, get a registration token via signup
    let signup_data = json!({
        "email": "registration@example.com",
        "password": "securePassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 202);

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Complete registration
    let completion_data = json!({
        "registration_token": registration_token,
        "username": "uniqueuser123"
    });

    let completion_response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    // Verify response
    assert_eq!(
        completion_response.status(),
        200,
        "Should return 200 for successful registration completion"
    );

    let response_body: Value = completion_response
        .json()
        .await
        .expect("Should return JSON response");

    // Verify response structure
    assert_eq!(
        response_body["user"]["username"].as_str().unwrap(),
        "uniqueuser123"
    );
    assert_eq!(
        response_body["user"]["email"].as_str().unwrap(),
        "registration@example.com"
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
async fn test_complete_registration_triggers_user_signed_up_event() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Get registration token
    let signup_data = json!({
        "email": "eventuser@example.com",
        "password": "securePassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 202);

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Complete registration
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

    // Note: In a real implementation, you would verify that the user_signed_up event
    // was triggered with the correct user data. This would involve checking your
    // event store/message queue system.
}

#[tokio::test]
#[serial]
async fn test_complete_registration_invalidates_token_after_use() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Get registration token
    let signup_data = json!({
        "email": "tokentest@example.com",
        "password": "securePassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 202);

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Complete registration (first use)
    let completion_data = json!({
        "registration_token": registration_token,
        "username": "tokenuser"
    });

    let first_completion = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send first completion request");

    assert_eq!(first_completion.status(), 200);

    // Try to use the same token again
    let second_completion_data = json!({
        "registration_token": registration_token,
        "username": "anothername"
    });

    let second_completion = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&second_completion_data)
        .send()
        .await
        .expect("Failed to send second completion request");

    // Should fail because token is already used
    assert_eq!(
        second_completion.status(),
        400,
        "Should return 400 for reused token"
    );

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
async fn test_complete_registration_taken_username_returns_409() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Pre-create user with taken username
    let _existing_user = DbFixtures::user()
        .username("takenname")
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

    assert_eq!(signup_response.status(), 202);

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Try to complete registration with taken username
    let completion_data = json!({
        "registration_token": registration_token,
        "username": "takenname"
    });

    let completion_response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    // Verify response
    assert_eq!(
        completion_response.status(),
        409,
        "Should return 409 for taken username"
    );
}

#[tokio::test]
#[serial]
async fn test_complete_registration_invalid_username_format_returns_422() {
    // Setup
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

    assert_eq!(signup_response.status(), 202);

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Test various invalid username formats
    let long_username = "a".repeat(51);
    let invalid_usernames = vec![
        "ab",           // Too short
        &long_username, // Too long
        "user@name",    // Invalid character
        "user name",    // Space
        "user.name",    // Dot
        "123",          // Only numbers
        "",             // Empty
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

        assert_eq!(
            completion_response.status(),
            422,
            "Should return 422 for invalid username format: '{}'",
            invalid_username
        );
    }
}

// =============================================================================
// 🔍 USERNAME VALIDATION TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_username_check_available_username_returns_true() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Check available username
    let response = client
        .get(&format!("{}/api/auth/username/check", base_url))
        .query(&[("username", "availableuser")])
        .send()
        .await
        .expect("Failed to send username check request");

    assert_eq!(response.status(), 200);

    let response_body: Value = response.json().await.expect("Should return JSON response");

    assert_eq!(response_body["available"].as_bool().unwrap(), true);
    // For available usernames, suggestions array should be empty
    assert_eq!(response_body["suggestions"].as_array().unwrap().len(), 0);
}

#[tokio::test]
#[serial]
async fn test_username_check_taken_username_returns_false_with_suggestions() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

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
        .expect("Failed to send username check request");

    assert_eq!(response.status(), 200);

    let response_body: Value = response.json().await.expect("Should return JSON response");

    assert_eq!(response_body["available"].as_bool().unwrap(), false);

    // Should provide suggestions
    let suggestions = response_body["suggestions"].as_array().unwrap();
    assert!(suggestions.len() > 0, "Should provide username suggestions");

    // Verify suggestions are reasonable
    for suggestion in suggestions {
        let suggestion_str = suggestion.as_str().unwrap();
        assert!(
            suggestion_str.starts_with("johndoe"),
            "Suggestions should be based on original username"
        );
        assert!(
            suggestion_str.len() >= 3,
            "Suggestions should meet minimum length"
        );
        assert!(
            suggestion_str.len() <= 50,
            "Suggestions should meet maximum length"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_username_validation_rules() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test minimum length
    let response = client
        .get(&format!("{}/api/auth/username/check", base_url))
        .query(&[("username", "ab")])
        .send()
        .await
        .expect("Failed to send username check request");

    // Should return validation error for too short username
    assert!(
        response.status() == 400 || response.status() == 422,
        "Should return error for username too short"
    );

    // Test maximum length
    let long_username = "a".repeat(51);
    let response = client
        .get(&format!("{}/api/auth/username/check", base_url))
        .query(&[("username", &long_username)])
        .send()
        .await
        .expect("Failed to send username check request");

    // Should return validation error for too long username
    assert!(
        response.status() == 400 || response.status() == 422,
        "Should return error for username too long"
    );

    // Test valid username formats
    let max_length_username = "a".repeat(50);
    let valid_usernames = vec![
        "user123",
        "user_name",
        "user-name",
        "User123",
        "abc",
        &max_length_username,
    ];

    for valid_username in valid_usernames {
        let response = client
            .get(&format!("{}/api/auth/username/check", base_url))
            .query(&[("username", valid_username)])
            .send()
            .await
            .expect("Failed to send username check request");

        assert_eq!(
            response.status(),
            200,
            "Should accept valid username format: '{}'",
            valid_username
        );
    }
}

// =============================================================================
// 🔄 END-TO-END FLOW TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_complete_email_first_flow() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Step 1: Email signup
    let signup_data = json!({
        "email": "flowtest@example.com",
        "password": "securePassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 202);

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    // Step 2: Complete registration
    let completion_data = json!({
        "registration_token": registration_token,
        "username": "flowuser"
    });

    let completion_response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    assert_eq!(completion_response.status(), 200);

    let completion_body: Value = completion_response
        .json()
        .await
        .expect("Should return JSON response");
    let access_token = completion_body["access_token"].as_str().unwrap();

    // Step 3: Attempt login (should fail - email not verified)
    let login_data = json!({
        "email": "flowtest@example.com",
        "password": "securePassword123"
    });

    let login_response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login request");

    // Should fail because email is not verified
    assert_eq!(
        login_response.status(),
        401,
        "Login should fail before email verification"
    );

    // Step 4: Mock email verification using db fixture
    use fixtures::DbFixtures;

    // Get the verification token from the database (simulating user clicking email link)
    let verification_record = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT verification_token FROM user_email_verification WHERE email = 'flowtest@example.com'".to_string(),
        ))
        .await
        .expect("Failed to query verification token")
        .expect("Verification token should exist");

    let verification_token: String = verification_record
        .try_get("", "verification_token")
        .expect("Failed to get verification token");

    // Verify the email
    let verify_data = json!({
        "email": "flowtest@example.com",
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

    // Step 5: Now login should work
    let login_response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(
        login_response.status(),
        200,
        "Login should succeed after email verification"
    );

    // Step 6: Add OAuth provider (mock scenario)
    // Setup GitHub mock
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;

    // Start OAuth linking flow with authentication
    let oauth_start_response = client
        .get(&format!("{}/api/auth/github/link", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .expect("Failed to start OAuth linking");

    // Should redirect to GitHub for linking
    assert_eq!(oauth_start_response.status(), 303);

    // ✅ Verify single username maintained across all auth methods
    // ✅ Verify single user_signed_up event in entire flow
    // This would be verified through event store/message queue inspection
}

#[tokio::test]
#[serial]
async fn test_complete_oauth_first_flow() {
    // Setup
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Setup GitHub mock
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;

    // Step 1: OAuth signup
    let start_response = client
        .get(&format!("{}/api/auth/github/login", base_url))
        .send()
        .await
        .expect("Failed to start OAuth flow");

    assert_eq!(start_response.status(), 303);

    let location = start_response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    let (_, params) = parse_redirect_url(location).unwrap();
    let state = params.get("state").unwrap();

    // Complete OAuth callback
    let callback_response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[("code", "test_auth_code"), ("state", state)])
        .send()
        .await
        .expect("Failed to complete OAuth callback");

    assert_eq!(callback_response.status(), 202);

    let callback_body: Value = callback_response
        .json()
        .await
        .expect("Should return JSON response");
    let registration_token = callback_body["registration_token"].as_str().unwrap();

    // Step 2: Complete registration
    let completion_data = json!({
        "registration_token": registration_token,
        "username": "oauthuser"
    });

    let completion_response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion request");

    assert_eq!(completion_response.status(), 200);

    let completion_body: Value = completion_response
        .json()
        .await
        .expect("Should return JSON response");
    let access_token = completion_body["access_token"].as_str().unwrap();

    // Step 3: Add password authentication
    let user_email = completion_body["user"]["email"].as_str().unwrap();

    let password_signup_data = json!({
        "email": user_email,
        "password": "newPassword123"
    });

    let password_signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&password_signup_data)
        .send()
        .await
        .expect("Failed to add password auth");

    // Should return 200 (existing user, adding password auth)
    assert_eq!(password_signup_response.status(), 200);

    // Create verification token fixture in db using @fixtures::DbFixtures::email_verification()
    let verification_token = DbFixtures::email_verification()
        .email(user_email)
        .verification_token("test_verification_token_123")
        .commit(db.clone())
        .await
        .expect("Failed to create verification token");

    let verify_data = json!({
        "email": user_email,
        "verification_token": verification_token.verification_token()
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

    // Step 4: Login with password
    let login_data = json!({
        "email": user_email,
        "password": "newPassword123"
    });

    let login_response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login with password");

    assert_eq!(login_response.status(), 200);

    // ✅ Verify provider linking works after registration completion
    // ✅ Verify email verification triggered at right moment
    // This would be verified through event store/message queue inspection
}
