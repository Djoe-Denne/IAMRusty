// Include common test utilities and fixtures

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;
mod utils;

use common::setup_test_server;
use fixtures::DbFixtures;
use iam_infra::auth::PasswordService;
use reqwest::Client;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;
use utils::auth::AuthTestUtils;

// 🔐 Email/Password Authentication Tests
// 📝 POST /auth/signup

#[tokio::test]
#[serial]
async fn test_signup_duplicate_email_fails() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create first user
    let signup_data = json!({
        "email": "duplicate@example.com",
        "password": "securePassword123"
    });

    let response1 = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send first signup request");

    assert_eq!(response1.status(), 202, "First signup should succeed");

    let signup_body: Value = response1.json().await.expect("Should return JSON");
    let registration_token = signup_body["registration_token"].as_str().unwrap();

    let completion_data = json!({
        "registration_token": registration_token,
        "username": "testuser"
    });

    let response = client
        .post(&format!("{}/api/auth/complete-registration", base_url))
        .header("Content-Type", "application/json")
        .json(&completion_data)
        .send()
        .await
        .expect("Failed to send completion");

    // Try to create second user with same email
    let signup_data2 = json!({
        "email": "duplicate@example.com",
        "password": "anotherPassword456"
    });

    let response2 = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data2)
        .send()
        .await
        .expect("Failed to send second signup request");

    // ✅ Should return 409 Conflict for duplicate email
    assert_eq!(
        response2.status(),
        409,
        "Should return 409 for duplicate email"
    );

    let error_response: Value = response2
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(
        error_response.get("error").is_some() || error_response.get("message").is_some(),
        "Should contain error message"
    );
}

#[tokio::test]
#[serial]
async fn test_signup_invalid_email_format() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    let invalid_emails = vec![
        "invalid-email",
        "missing@domain",
        "@missing-local.com",
        "spaces in@email.com",
        "double@@domain.com",
        "",
    ];

    for invalid_email in invalid_emails {
        let signup_data = json!({
            "email": invalid_email,
            "password": "securePassword123"
        });

        let response = client
            .post(&format!("{}/api/auth/signup", base_url))
            .header("Content-Type", "application/json")
            .json(&signup_data)
            .send()
            .await
            .expect("Failed to send signup request");

        // ✅ Should return 422 for invalid email format
        assert_eq!(
            response.status(),
            422,
            "Should return 422 for invalid email: {}",
            invalid_email
        );
    }
}

#[tokio::test]
#[serial]
async fn test_signup_weak_password_validation() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    let weak_passwords = vec![
        "",         // Empty
        "123",      // Too short
        "1234567",  // Still too short (< 8 chars)
        "password", // Common password
    ];

    for weak_password in weak_passwords {
        let signup_data = json!({
            "email": "test@example.com",
            "password": weak_password
        });

        let response = client
            .post(&format!("{}/api/auth/signup", base_url))
            .header("Content-Type", "application/json")
            .json(&signup_data)
            .send()
            .await
            .expect("Failed to send signup request");

        // ✅ Should return 422 for weak password
        assert_eq!(
            response.status(),
            422,
            "Should return 422 for weak password: '{}'",
            weak_password
        );
    }
}

#[tokio::test]
#[serial]
async fn test_signup_missing_required_fields() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    let test_cases = vec![
        (
            json!({"email": "test@example.com", "password": "password123"}),
            "missing username",
        ),
        (
            json!({"username": "testuser", "password": "password123"}),
            "missing email",
        ),
        (
            json!({"username": "testuser", "email": "test@example.com"}),
            "missing password",
        ),
        (json!({}), "missing all fields"),
    ];

    for (signup_data, description) in test_cases {
        let response = client
            .post(&format!("{}/api/auth/signup", base_url))
            .header("Content-Type", "application/json")
            .json(&signup_data)
            .send()
            .await
            .expect("Failed to send signup request");

        // ✅ Should return 422 for missing required fields
        assert_eq!(
            response.status(),
            422,
            "Should return 422 for {}",
            description
        );
    }
}

// 🔑 POST /auth/login

#[tokio::test]
#[serial]
async fn test_login_unverified_email_fails() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create unverified user
    let password_service = PasswordService::new();
    let hashed_password = password_service
        .hash_password("originalPassword123")
        .expect("Failed to hash password");

    let user = DbFixtures::user()
        .username("unverifieduser")
        .password_hash(hashed_password)
        .commit(_fixture.db())
        .await
        .expect("Failed to create user");

    let _user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("unverified@example.com")
        .is_primary(true)
        .is_verified(false) // Not verified
        .commit(_fixture.db())
        .await
        .expect("Failed to create user email");

    // Make login request
    let login_data = json!({
        "email": "unverified@example.com",
        "password": "originalPassword123"
    });

    let response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login request");

    // ✅ Should return 401 for unverified email
    assert_eq!(
        response.status(),
        401,
        "Should return 401 for unverified email"
    );

    let error_response: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(
        error_response.get("error").is_some() || error_response.get("message").is_some(),
        "Should contain error message about email verification"
    );
}

#[tokio::test]
#[serial]
async fn test_login_invalid_credentials() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create verified user
    let password_service = PasswordService::new();
    let hashed_password = password_service
        .hash_password("originalPassword123")
        .expect("Failed to hash password");

    let user = DbFixtures::user()
        .username("testuser")
        .password_hash(hashed_password)
        .commit(_fixture.db())
        .await
        .expect("Failed to create user");

    let _user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("test@example.com")
        .is_primary(true)
        .is_verified(true)
        .commit(_fixture.db())
        .await
        .expect("Failed to create user email");

    // Test cases for invalid credentials
    let invalid_cases = vec![
        (
            json!({"email": "test@example.com", "password": "wrongpassword"}),
            "wrong password",
            401,
        ),
        (
            json!({"email": "nonexistent@example.com", "password": "anypassword"}),
            "nonexistent email",
            401,
        ),
        (
            json!({"email": "test@example.com", "password": ""}),
            "empty password",
            422,
        ), // Validation error
    ];

    for (login_data, description, expected_status) in invalid_cases {
        let response = client
            .post(&format!("{}/api/auth/login", base_url))
            .header("Content-Type", "application/json")
            .json(&login_data)
            .send()
            .await
            .expect("Failed to send login request");

        // ✅ Should return correct status code for each case
        assert_eq!(
            response.status(),
            expected_status,
            "Should return {} for {}",
            expected_status,
            description
        );

        let error_response: Value = response
            .json()
            .await
            .expect("Should return JSON error response");

        // Handle both AuthError format (error/message) and axum-valid format (errors array or field errors)
        let has_error_info = error_response.get("error").is_some() 
            || error_response.get("message").is_some() 
            || error_response.get("errors").is_some()
            || error_response.get("password").is_some() // axum-valid field validation
            || error_response.get("email").is_some(); // axum-valid field validation

        assert!(
            has_error_info,
            "Should contain error information for {}: {}",
            description, error_response
        );
    }
}

#[tokio::test]
#[serial]
async fn test_login_missing_required_fields() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    let test_cases = vec![
        (json!({"password": "password123"}), "missing email"),
        (json!({"email": "test@example.com"}), "missing password"),
        (json!({}), "missing both fields"),
    ];

    for (login_data, description) in test_cases {
        let response = client
            .post(&format!("{}/api/auth/login", base_url))
            .header("Content-Type", "application/json")
            .json(&login_data)
            .send()
            .await
            .expect("Failed to send login request");

        // ✅ Should return 422 for missing required fields
        assert_eq!(
            response.status(),
            422,
            "Should return 422 for {}",
            description
        );
    }
}

// ✅ POST /auth/verify

#[tokio::test]
#[serial]
async fn test_verify_email_success() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with unverified email
    let user = DbFixtures::user()
        .username("unverifieduser")
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("unverified@example.com")
        .is_primary(true)
        .is_verified(false)
        .commit(db.clone())
        .await
        .expect("Failed to create user email");

    // Create verification token
    let verification_token = "test_verification_token_123";
    let verification_id = Uuid::new_v4();

    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!(
            "INSERT INTO user_email_verification (id, email, verification_token, expires_at, created_at) 
             VALUES ('{}', '{}', '{}', NOW() + INTERVAL '1 hour', NOW())",
            verification_id, user_email.email(), verification_token
        ),
    ))
    .await
    .expect("Failed to create verification record");

    // Make verify request
    let response = client
        .get(&format!("{}/api/auth/verify", base_url))
        .query(&[("email", "unverified@example.com"), ("token", verification_token)])
        .send()
        .await
        .expect("Failed to send verify request");

    // ✅ Should return 200 OK for successful verification
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for successful verification"
    );

    let response_body: Value = response.json().await.expect("Should return JSON response");

    assert!(
        response_body.get("message").is_some(),
        "Should contain success message"
    );

    // ✅ Verify email is marked as verified in database
    let email_verified = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT is_verified FROM user_emails WHERE email = '{}'",
                user_email.email()
            ),
        ))
        .await
        .expect("Failed to query email verification status")
        .unwrap();

    let is_verified: bool = email_verified
        .try_get("", "is_verified")
        .expect("Failed to get verification status");
    assert!(is_verified, "Email should be marked as verified");

    // ✅ Verify verification token is deleted
    let token_count = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT COUNT(*) as count FROM user_email_verification WHERE email = '{}'",
                user_email.email()
            ),
        ))
        .await
        .expect("Failed to count verification tokens")
        .unwrap();

    let count: i64 = token_count
        .try_get("", "count")
        .expect("Failed to get token count");
    assert_eq!(
        count, 0,
        "Verification token should be deleted after successful verification"
    );
}

#[tokio::test]
#[serial]
async fn test_verify_email_invalid_token() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with unverified email
    let user = DbFixtures::user()
        .username("unverifieduser")
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("unverified@example.com")
        .is_primary(true)
        .is_verified(false)
        .commit(db.clone())
        .await
        .expect("Failed to create user email");

    // Create verification token
    let correct_token = "correct_token_123";
    let verification_id = Uuid::new_v4();

    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!(
            "INSERT INTO user_email_verification (id, email, verification_token, expires_at, created_at) 
             VALUES ('{}', '{}', '{}', NOW() + INTERVAL '1 hour', NOW())",
            verification_id, user_email.email(), correct_token
        ),
    ))
    .await
    .expect("Failed to create verification record");

    // Test with wrong token
    let response = client
        .get(&format!("{}/api/auth/verify", base_url))
        .query(&[("email", "unverified@example.com"), ("token", "wrong_token_456")])
        .send()
        .await
        .expect("Failed to send verify request");

    // ✅ Should return 400 for invalid token
    assert_eq!(
        response.status(),
        400,
        "Should return 400 for invalid verification token"
    );

    let error_response: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(
        error_response.get("error").is_some() || error_response.get("message").is_some(),
        "Should contain error message"
    );
}

#[tokio::test]
#[serial]
async fn test_verify_email_expired_token() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with unverified email
    let user = DbFixtures::user()
        .username("unverifieduser")
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("unverified@example.com")
        .is_primary(true)
        .is_verified(false)
        .commit(db.clone())
        .await
        .expect("Failed to create user email");

    // Create expired verification token
    let verification_token = "expired_token_123";
    let verification_id = Uuid::new_v4();

    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!(
            "INSERT INTO user_email_verification (id, email, verification_token, expires_at, created_at) 
             VALUES ('{}', '{}', '{}', NOW() - INTERVAL '1 hour', NOW() - INTERVAL '2 hours')",
            verification_id, user_email.email(), verification_token
        ),
    ))
    .await
    .expect("Failed to create verification record");

    // Make verify request with expired token
    let response = client
        .get(&format!("{}/api/auth/verify", base_url))
        .query(&[("email", "unverified@example.com"), ("token", verification_token)])
        .send()
        .await
        .expect("Failed to send verify request");

    // ✅ Should return 400 for expired token
    assert_eq!(
        response.status(),
        400,
        "Should return 400 for expired verification token"
    );

    let error_response: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(
        error_response.get("error").is_some() || error_response.get("message").is_some(),
        "Should contain error message about expired token"
    );
}

#[tokio::test]
#[serial]
async fn test_verify_email_nonexistent_email() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Make verify request for nonexistent email
    let response = client
        .get(&format!("{}/api/auth/verify", base_url))
        .query(&[("email", "nonexistent@example.com"), ("token", "any_token_123")])
        .send()
        .await
        .expect("Failed to send verify request");

    // ✅ Should return 404 for nonexistent email
    assert_eq!(
        response.status(),
        404,
        "Should return 404 for nonexistent email"
    );

    let error_response: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(
        error_response.get("error").is_some() || error_response.get("message").is_some(),
        "Should contain error message"
    );
}

#[tokio::test]
#[serial]
async fn test_verify_email_already_verified() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with already verified email
    let user = DbFixtures::user()
        .username("verifieduser")
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("already_verified@example.com")
        .is_primary(true)
        .is_verified(true) // Already verified
        .commit(db.clone())
        .await
        .expect("Failed to create user email");

    // Make verify request for already verified email
    let response = client
        .get(&format!("{}/api/auth/verify", base_url))
        .query(&[("email", "already_verified@example.com"), ("token", "any_token_123")])
        .send()
        .await
        .expect("Failed to send verify request");

    // ✅ Should return 400 for already verified email
    assert_eq!(
        response.status(),
        400,
        "Should return 400 for already verified email"
    );

    let error_response: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(
        error_response.get("error").is_some() || error_response.get("message").is_some(),
        "Should contain error message about already verified email"
    );
}

#[tokio::test]
#[serial]
async fn test_verify_email_missing_required_fields() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    let test_cases = vec![
        (vec![("token", "token123")], "missing email"),
        (vec![("email", "test@example.com")], "missing token"),
        (vec![], "missing both fields"),
    ];

    for (query_params, description) in test_cases {
        let response = client
            .get(&format!("{}/api/auth/verify", base_url))
            .query(&query_params)
            .send()
            .await
            .expect("Failed to send verify request");

        // ✅ Should return 400 for missing required fields
        assert_eq!(
            response.status(),
            400,
            "Should return 400 for {}",
            description
        );
    }
}

// 🔒 End-to-End Authentication Flow Tests
