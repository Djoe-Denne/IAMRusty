// Include common test utilities and fixtures
#[path = "common/mod.rs"] 
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::{get_test_server, TestFixture, TestKafkaFixture};
use fixtures::DbFixtures;
use reqwest::Client;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;
use sea_orm::ConnectionTrait;
use infra::auth::PasswordService;

/// Create a common HTTP client for tests
fn create_test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client")
}

// 🔐 Email/Password Authentication Tests
// 📝 POST /auth/signup

#[tokio::test]
#[serial]
async fn test_signup_success() {
    // Setup test server and database
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Create signup request
    let signup_data = json!({
        "username": "alice",
        "email": "alice@example.com", 
        "password": "securePassword123"
    });

    // Make signup request
    let response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    // ✅ Should return 201 Created
    assert_eq!(response.status(), 201, "Should return 201 Created status");

    // ✅ Should return success message
    let response_body: Value = response
        .json()
        .await
        .expect("Should return JSON response");

    assert!(response_body.get("message").is_some(), "Should contain success message");
    let message = response_body["message"].as_str().unwrap();
    assert!(message.contains("created successfully"), "Should confirm user creation");
    assert!(message.contains("verification"), "Should mention email verification");

    // ✅ Verify user was created in database
    let db = fixture.db();
    let user_count = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM users WHERE username = 'alice'".to_string(),
        ))
        .await
        .expect("Failed to query users")
        .unwrap();
    
    let count: i64 = user_count.try_get("", "count").expect("Failed to get count");
    assert_eq!(count, 1, "User should be created in database");

    // ✅ Verify user email was created
    let email_count = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM user_emails WHERE email = 'alice@example.com'".to_string(),
        ))
        .await
        .expect("Failed to query user emails")
        .unwrap();
    
    let email_count_val: i64 = email_count.try_get("", "count").expect("Failed to get email count");
    assert_eq!(email_count_val, 1, "User email should be created in database");

    // ✅ Verify email verification record was created
    let verification_count = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM user_email_verification WHERE email = 'alice@example.com'".to_string(),
        ))
        .await
        .expect("Failed to query email verifications")
        .unwrap();
    
    let verification_count_val: i64 = verification_count.try_get("", "count").expect("Failed to get verification count");
    assert_eq!(verification_count_val, 1, "Email verification record should be created");

    // ✅ Verify password is hashed (not stored in plain text)
    let user_record = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT password_hash FROM users WHERE username = 'alice'".to_string(),
        ))
        .await
        .expect("Failed to query user password")
        .unwrap();
    
    let password_hash: String = user_record.try_get("", "password_hash").expect("Failed to get password hash");
    assert_ne!(password_hash, "securePassword123", "Password should be hashed, not stored in plain text");
    assert!(password_hash.len() > 50, "Password hash should be substantial length");
}

#[tokio::test]
#[serial]
async fn test_signup_duplicate_email_fails() {
    // Setup test server and database
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Create first user
    let signup_data = json!({
        "username": "alice",
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

    assert_eq!(response1.status(), 201, "First signup should succeed");

    // Try to create second user with same email
    let signup_data2 = json!({
        "username": "bob",
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
    assert_eq!(response2.status(), 409, "Should return 409 for duplicate email");

    let error_response: Value = response2
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(error_response.get("error").is_some() || error_response.get("message").is_some(), 
           "Should contain error message");
}

#[tokio::test]
#[serial]
async fn test_signup_invalid_email_format() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    let invalid_emails = vec![
        "invalid-email",
        "missing@domain",
        "@missing-local.com",
        "spaces in@email.com",
        "double@@domain.com",
        ""
    ];

    for invalid_email in invalid_emails {
        let signup_data = json!({
            "username": "testuser",
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
        assert_eq!(response.status(), 422, "Should return 422 for invalid email: {}", invalid_email);
    }
}

#[tokio::test]
#[serial]
async fn test_signup_weak_password_validation() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    let weak_passwords = vec![
        "", // Empty
        "123", // Too short
        "1234567", // Still too short (< 8 chars)
        "password", // Common password  
    ];

    for weak_password in weak_passwords {
        let signup_data = json!({
            "username": "testuser",
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
        assert_eq!(response.status(), 422, "Should return 422 for weak password: '{}'", weak_password);
    }
}

#[tokio::test]
#[serial]
async fn test_signup_missing_required_fields() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    let test_cases = vec![
        (json!({"email": "test@example.com", "password": "password123"}), "missing username"),
        (json!({"username": "testuser", "password": "password123"}), "missing email"),
        (json!({"username": "testuser", "email": "test@example.com"}), "missing password"),
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
        assert_eq!(response.status(), 422, "Should return 422 for {}", description);
    }
}

// 🔑 POST /auth/login

#[tokio::test]
#[serial]
async fn test_login_success_verified_user() {
    // Setup test server and database
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Create verified user using fixtures
    let password_service = PasswordService::new();
    let hashed_password = password_service
        .hash_password("originalPassword123")
        .expect("Failed to hash password");
    
    let user = DbFixtures::user()
        .username("verifieduser")
        .password_hash(hashed_password)
        .commit(fixture.db())
        .await
        .expect("Failed to create user");

    let _user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("verified@example.com")
        .is_primary(true)
        .is_verified(true)
        .commit(fixture.db())
        .await
        .expect("Failed to create user email");

    // Make login request
    let login_data = json!({
        "email": "verified@example.com",
        "password": "originalPassword123"
    });

    let response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login request");

    // ✅ Should return 200 OK for successful login
    assert_eq!(response.status(), 200, "Should return 200 OK for successful login");

    let response_body: Value = response
        .json()
        .await
        .expect("Should return JSON response");

    // ✅ Should return JWT token
    assert!(response_body.get("token").is_some(), "Should return JWT token");
    let token = response_body["token"].as_str().unwrap();
    assert!(!token.is_empty(), "Token should not be empty");
    assert!(token.starts_with("eyJ"), "Should be a JWT token (starts with eyJ)");

    // ✅ Should return user information
    assert!(response_body.get("user").is_some(), "Should return user information");
    let user_info = &response_body["user"];
    assert_eq!(user_info["username"], "verifieduser", "Should return correct username");
    assert_eq!(user_info["email"], "verified@example.com", "Should return primary email");
    assert!(user_info.get("id").is_some(), "Should return user ID");
}

#[tokio::test]
#[serial]
async fn test_login_unverified_email_fails() {
    // Setup test server and database
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Create unverified user
    let password_service = PasswordService::new();
    let hashed_password = password_service
        .hash_password("originalPassword123")
        .expect("Failed to hash password");
    
    let user = DbFixtures::user()
        .username("unverifieduser")
        .password_hash(hashed_password)
        .commit(fixture.db())
        .await
        .expect("Failed to create user");

    let _user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("unverified@example.com")
        .is_primary(true)
        .is_verified(false) // Not verified
        .commit(fixture.db())
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
    assert_eq!(response.status(), 401, "Should return 401 for unverified email");

    let error_response: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(error_response.get("error").is_some() || error_response.get("message").is_some(), 
           "Should contain error message about email verification");
}

#[tokio::test]
#[serial]
async fn test_login_invalid_credentials() {
    // Setup test server and database
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Create verified user
    let password_service = PasswordService::new();
    let hashed_password = password_service
        .hash_password("originalPassword123")
        .expect("Failed to hash password");
    
    let user = DbFixtures::user()
        .username("testuser")
        .password_hash(hashed_password)
        .commit(fixture.db())
        .await
        .expect("Failed to create user");

    let _user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("test@example.com")
        .is_primary(true)
        .is_verified(true)
        .commit(fixture.db())
        .await
        .expect("Failed to create user email");

    // Test cases for invalid credentials
    let invalid_cases = vec![
        (json!({"email": "test@example.com", "password": "wrongpassword"}), "wrong password", 401),
        (json!({"email": "nonexistent@example.com", "password": "anypassword"}), "nonexistent email", 401),
        (json!({"email": "test@example.com", "password": ""}), "empty password", 422), // Validation error
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
        assert_eq!(response.status(), expected_status, "Should return {} for {}", expected_status, description);

        let error_response: Value = response
            .json()
            .await
            .expect("Should return JSON error response");

        // Handle both AuthError format (error/message) and axum-valid format (errors array or field errors)
        let has_error_info = error_response.get("error").is_some() 
            || error_response.get("message").is_some() 
            || error_response.get("errors").is_some()
            || error_response.get("password").is_some() // axum-valid field validation
            || error_response.get("email").is_some();   // axum-valid field validation
        
        assert!(has_error_info, 
               "Should contain error information for {}: {}", description, error_response);
    }
}

#[tokio::test]
#[serial]
async fn test_login_missing_required_fields() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

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
        assert_eq!(response.status(), 422, "Should return 422 for {}", description);
    }
}

// ✅ POST /auth/verify

#[tokio::test]
#[serial]
async fn test_verify_email_success() {
    // Setup test server and database
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();
    let db = fixture.db();

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
    let verify_data = json!({
        "email": "unverified@example.com",
        "verification_token": verification_token
    });

    let response = client
        .post(&format!("{}/api/auth/verify", base_url))
        .header("Content-Type", "application/json")
        .json(&verify_data)
        .send()
        .await
        .expect("Failed to send verify request");

    // ✅ Should return 200 OK for successful verification
    assert_eq!(response.status(), 200, "Should return 200 OK for successful verification");

    let response_body: Value = response
        .json()
        .await
        .expect("Should return JSON response");

    assert!(response_body.get("message").is_some(), "Should contain success message");

    // ✅ Verify email is marked as verified in database
    let email_verified = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT is_verified FROM user_emails WHERE email = '{}'", user_email.email()),
        ))
        .await
        .expect("Failed to query email verification status")
        .unwrap();
    
    let is_verified: bool = email_verified.try_get("", "is_verified").expect("Failed to get verification status");
    assert!(is_verified, "Email should be marked as verified");

    // ✅ Verify verification token is deleted
    let token_count = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT COUNT(*) as count FROM user_email_verification WHERE email = '{}'", user_email.email()),
        ))
        .await
        .expect("Failed to count verification tokens")
        .unwrap();
    
    let count: i64 = token_count.try_get("", "count").expect("Failed to get token count");
    assert_eq!(count, 0, "Verification token should be deleted after successful verification");
}

#[tokio::test]
#[serial]
async fn test_verify_email_invalid_token() {
    // Setup test server and database
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();
    let db = fixture.db();

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
    let verify_data = json!({
        "email": "unverified@example.com",
        "verification_token": "wrong_token_456"
    });

    let response = client
        .post(&format!("{}/api/auth/verify", base_url))
        .header("Content-Type", "application/json")
        .json(&verify_data)
        .send()
        .await
        .expect("Failed to send verify request");

    // ✅ Should return 400 for invalid token
    assert_eq!(response.status(), 400, "Should return 400 for invalid verification token");

    let error_response: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(error_response.get("error").is_some() || error_response.get("message").is_some(), 
           "Should contain error message");
}

#[tokio::test]
#[serial]
async fn test_verify_email_expired_token() {
    // Setup test server and database
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();
    let db = fixture.db();

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
    let verify_data = json!({
        "email": "unverified@example.com",
        "verification_token": verification_token
    });

    let response = client
        .post(&format!("{}/api/auth/verify", base_url))
        .header("Content-Type", "application/json")
        .json(&verify_data)
        .send()
        .await
        .expect("Failed to send verify request");

    // ✅ Should return 400 for expired token
    assert_eq!(response.status(), 400, "Should return 400 for expired verification token");

    let error_response: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(error_response.get("error").is_some() || error_response.get("message").is_some(), 
           "Should contain error message about expired token");
}

#[tokio::test]
#[serial]
async fn test_verify_email_nonexistent_email() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Make verify request for nonexistent email
    let verify_data = json!({
        "email": "nonexistent@example.com",
        "verification_token": "any_token_123"
    });

    let response = client
        .post(&format!("{}/api/auth/verify", base_url))
        .header("Content-Type", "application/json")
        .json(&verify_data)
        .send()
        .await
        .expect("Failed to send verify request");

    // ✅ Should return 404 for nonexistent email
    assert_eq!(response.status(), 404, "Should return 404 for nonexistent email");

    let error_response: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(error_response.get("error").is_some() || error_response.get("message").is_some(), 
           "Should contain error message");
}

#[tokio::test]
#[serial]
async fn test_verify_email_already_verified() {
    // Setup test server and database
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();
    let db = fixture.db();

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
    let verify_data = json!({
        "email": "already_verified@example.com",
        "verification_token": "any_token_123"
    });

    let response = client
        .post(&format!("{}/api/auth/verify", base_url))
        .header("Content-Type", "application/json")
        .json(&verify_data)
        .send()
        .await
        .expect("Failed to send verify request");

    // ✅ Should return 400 for already verified email
    assert_eq!(response.status(), 400, "Should return 400 for already verified email");

    let error_response: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(error_response.get("error").is_some() || error_response.get("message").is_some(), 
           "Should contain error message about already verified email");
}

#[tokio::test]
#[serial]
async fn test_verify_email_missing_required_fields() {
    // Setup test server
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    let test_cases = vec![
        (json!({"verification_token": "token123"}), "missing email"),
        (json!({"email": "test@example.com"}), "missing verification_token"),
        (json!({}), "missing both fields"),
    ];

    for (verify_data, description) in test_cases {
        let response = client
            .post(&format!("{}/api/auth/verify", base_url))
            .header("Content-Type", "application/json")
            .json(&verify_data)
            .send()
            .await
            .expect("Failed to send verify request");

        // ✅ Should return 422 for missing required fields
        assert_eq!(response.status(), 422, "Should return 422 for {}", description);
    }
}

// 🔒 End-to-End Authentication Flow Tests

#[tokio::test]
#[serial]
async fn test_complete_signup_verify_login_flow() {
    // Setup test server and database
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();
    let db = fixture.db();

    // Step 1: Signup
    let signup_data = json!({
        "username": "e2euser",
        "email": "e2e@example.com",
        "password": "securePassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 201, "Signup should succeed");

    // Step 2: Get verification token from database
    let verification_record = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT verification_token FROM user_email_verification WHERE email = 'e2e@example.com'".to_string(),
        ))
        .await
        .expect("Failed to query verification token")
        .unwrap();
    
    let verification_token: String = verification_record.try_get("", "verification_token").expect("Failed to get verification token");

    // Step 3: Verify email
    let verify_data = json!({
        "email": "e2e@example.com",
        "verification_token": verification_token
    });

    let verify_response = client
        .post(&format!("{}/api/auth/verify", base_url))
        .header("Content-Type", "application/json")
        .json(&verify_data)
        .send()
        .await
        .expect("Failed to send verify request");

    assert_eq!(verify_response.status(), 200, "Email verification should succeed");

    // Step 4: Login with verified email
    let login_data = json!({
        "email": "e2e@example.com",
        "password": "securePassword123"
    });

    let login_response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), 200, "Login should succeed after verification");

    let login_body: Value = login_response
        .json()
        .await
        .expect("Should return JSON response");

    // ✅ Should return JWT token and user info
    assert!(login_body.get("token").is_some(), "Should return JWT token");
    assert!(login_body.get("user").is_some(), "Should return user information");
    
    let user_info = &login_body["user"];
    assert_eq!(user_info["username"], "e2euser", "Should return correct username");
    assert_eq!(user_info["email"], "e2e@example.com", "Should return primary email");
}

#[tokio::test]
#[serial]
async fn test_login_before_email_verification_fails() {
    // Setup test server and database
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Step 1: Signup
    let signup_data = json!({
        "username": "unverifiedlogin",
        "email": "unverified_login@example.com",
        "password": "securePassword123"
    });

    let signup_response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    assert_eq!(signup_response.status(), 201, "Signup should succeed");

    // Step 2: Try to login immediately (without verification)
    let login_data = json!({
        "email": "unverified_login@example.com",
        "password": "securePassword123"
    });

    let login_response = client
        .post(&format!("{}/api/auth/login", base_url))
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
        .await
        .expect("Failed to send login request");

    // ✅ Should fail with 401 - email not verified
    assert_eq!(login_response.status(), 401, "Login should fail for unverified email");

    let error_response: Value = login_response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(error_response.get("error").is_some() || error_response.get("message").is_some(), 
           "Should contain error message about email verification");
}
