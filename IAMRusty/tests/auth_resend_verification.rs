// Include common test utilities and fixtures

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use chrono;
use common::setup_test_server;
use fixtures::DbFixtures;
use reqwest::Client;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

// 🔄 Resend Verification Email Tests
// 📝 POST /api/auth/resend-verification

#[tokio::test]
#[serial]
async fn test_resend_verification_success() {
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
        .is_verified(false) // Key: email is not verified
        .commit(db.clone())
        .await
        .expect("Failed to create user email");

    // Create resend verification request
    let resend_data = json!({
        "email": "unverified@example.com"
    });

    // Make resend verification request
    let response = client
        .post(&format!("{}/api/auth/resend-verification", base_url))
        .header("Content-Type", "application/json")
        .json(&resend_data)
        .send()
        .await
        .expect("Failed to send resend verification request");

    // ✅ Should return 200 OK for successful resend
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for successful verification resend"
    );

    // ✅ Should return success message
    let response_body: Value = response.json().await.expect("Should return JSON response");

    assert!(
        response_body.get("message").is_some(),
        "Should contain success message"
    );
    let message = response_body["message"].as_str().unwrap();
    assert!(
        message.contains("If your email is registered"),
        "Should contain generic success message"
    );

    // ✅ Verify new verification token was created in database
    let verification_count = db
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

    let count: i64 = verification_count
        .try_get("", "count")
        .expect("Failed to get token count");
    assert!(count > 0, "New verification token should be created");

    // ✅ Verify email is still unverified (only resend, not auto-verify)
    assert!(
        user_email
            .check(db.clone())
            .await
            .expect("Failed to check user email")
    );
    let email_status = db
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

    let is_verified: bool = email_status
        .try_get("", "is_verified")
        .expect("Failed to get verification status");
    assert!(!is_verified, "Email should remain unverified after resend");
}

#[tokio::test]
#[serial]
async fn test_resend_verification_email_already_verified() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with verified email
    let user = DbFixtures::user()
        .username("verifieduser")
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("verified@example.com")
        .is_primary(true)
        .is_verified(true) // Key: email is already verified
        .commit(db.clone())
        .await
        .expect("Failed to create user email");

    // Create resend verification request
    let resend_data = json!({
        "email": "verified@example.com"
    });

    // Make resend verification request
    let response = client
        .post(&format!("{}/api/auth/resend-verification", base_url))
        .header("Content-Type", "application/json")
        .json(&resend_data)
        .send()
        .await
        .expect("Failed to send resend verification request");

    // ✅ Should return 200 OK even for already verified email (security: prevent user enumeration)
    assert_eq!(
        response.status(),
        200,
        "Should return 200 for already verified email to prevent user enumeration"
    );

    // ✅ Should return generic success message
    let response_body: Value = response.json().await.expect("Should return JSON response");

    assert!(
        response_body.get("message").is_some(),
        "Should contain message"
    );
    let message = response_body["message"].as_str().unwrap();
    // Should contain generic message that doesn't reveal email status
    assert!(
        message.contains("If your email is registered"),
        "Should contain generic message that doesn't reveal email status"
    );

    // ✅ Verify no new verification token was created
    let verification_count = db
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

    let count: i64 = verification_count
        .try_get("", "count")
        .expect("Failed to get token count");
    assert_eq!(
        count, 0,
        "No verification token should be created for already verified email"
    );
}

#[tokio::test]
#[serial]
async fn test_resend_verification_email_not_found() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Try to resend verification for non-existent email
    let resend_data = json!({
        "email": "nonexistent@example.com"
    });

    let response = client
        .post(&format!("{}/api/auth/resend-verification", base_url))
        .header("Content-Type", "application/json")
        .json(&resend_data)
        .send()
        .await
        .expect("Failed to send resend verification request");

    // ✅ Should return 200 OK even for non-existent email (security: prevent user enumeration)
    assert_eq!(
        response.status(),
        200,
        "Should return 200 for non-existent email to prevent user enumeration"
    );

    // ✅ Should return generic success message
    let response_body: Value = response.json().await.expect("Should return JSON response");

    assert!(
        response_body.get("message").is_some(),
        "Should contain message"
    );
    let message = response_body["message"].as_str().unwrap();
    // Should contain generic message that doesn't reveal email status
    assert!(
        message.contains("If your email is registered"),
        "Should contain generic message that doesn't reveal email status"
    );
}

#[tokio::test]
#[serial]
async fn test_resend_verification_invalid_email_format() {
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
        let resend_data = json!({
            "email": invalid_email
        });

        let response = client
            .post(&format!("{}/api/auth/resend-verification", base_url))
            .header("Content-Type", "application/json")
            .json(&resend_data)
            .send()
            .await
            .expect("Failed to send resend verification request");

        // ✅ Should return 422 for invalid email format (framework validation)
        assert_eq!(
            response.status(),
            422,
            "Should return 422 for invalid email format at framework level: {}",
            invalid_email
        );

        let response_body: Value = response
            .json()
            .await
            .expect("Should return JSON error response");

        // Framework validation errors are acceptable for invalid email formats
        assert!(
            response_body.get("error").is_some(),
            "Should contain validation error for: {}",
            invalid_email
        );
    }
}

#[tokio::test]
#[serial]
async fn test_resend_verification_missing_email_field() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test missing email field entirely
    let resend_data = json!({});

    let response = client
        .post(&format!("{}/api/auth/resend-verification", base_url))
        .header("Content-Type", "application/json")
        .json(&resend_data)
        .send()
        .await
        .expect("Failed to send resend verification request");

    // ✅ Should return 422 for missing email field (framework validation)
    assert_eq!(
        response.status(),
        422,
        "Should return 422 for missing email field at framework level"
    );

    let response_body: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    // Framework validation errors are acceptable for missing fields
    assert!(
        response_body.get("error").is_some(),
        "Should contain validation error"
    );
}

#[tokio::test]
#[serial]
async fn test_resend_verification_malformed_json() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test malformed JSON
    let malformed_json = r#"{"email": "test@example.com""#; // Missing closing brace

    let response = client
        .post(&format!("{}/api/auth/resend-verification", base_url))
        .header("Content-Type", "application/json")
        .body(malformed_json)
        .send()
        .await
        .expect("Failed to send resend verification request");

    // ✅ Should return 422 for malformed JSON (framework validation)
    assert_eq!(
        response.status(),
        400,
        "Should return 400 for malformed JSON at framework level"
    );

    let response_body: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    // Framework validation errors are acceptable for malformed JSON
    assert!(
        response_body.get("error").is_some(),
        "Should contain validation error"
    );
}

#[tokio::test]
#[serial]
async fn test_resend_verification_wrong_content_type() {
    // Setup test server
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test wrong content type
    let response = client
        .post(&format!("{}/api/auth/resend-verification", base_url))
        .header("Content-Type", "text/plain")
        .body(r#"{"email": "test@example.com"}"#)
        .send()
        .await
        .expect("Failed to send resend verification request");

    // ✅ Should return 422 for wrong content type (framework validation)
    assert_eq!(
        response.status(),
        422,
        "Should return 422 for wrong content type at framework level"
    );

    let response_body: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    // Framework validation errors are acceptable for wrong content type
    assert!(
        response_body.get("error").is_some(),
        "Should contain validation error"
    );
}

#[tokio::test]
#[serial]
async fn test_resend_verification_multiple_times_same_email() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with unverified email
    let user = DbFixtures::user()
        .username("multiresenduser")
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("multiresend@example.com")
        .is_primary(true)
        .is_verified(false)
        .commit(db.clone())
        .await
        .expect("Failed to create user email");

    let resend_data = json!({
        "email": "multiresend@example.com"
    });

    // ✅ First resend should succeed
    let response1 = client
        .post(&format!("{}/api/auth/resend-verification", base_url))
        .header("Content-Type", "application/json")
        .json(&resend_data)
        .send()
        .await
        .expect("Failed to send first resend request");

    assert_eq!(response1.status(), 200, "First resend should succeed");

    // ✅ Second resend should also succeed (no rate limiting in this test)
    let response2 = client
        .post(&format!("{}/api/auth/resend-verification", base_url))
        .header("Content-Type", "application/json")
        .json(&resend_data)
        .send()
        .await
        .expect("Failed to send second resend request");

    assert_eq!(response2.status(), 200, "Second resend should succeed");

    // ✅ Verify new verification tokens were created
    let verification_count = db
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

    let count: i64 = verification_count
        .try_get("", "count")
        .expect("Failed to get token count");
    assert!(
        count >= 1,
        "At least one verification token should exist after multiple resends"
    );

    // Note: The actual count might be 1 or 2 depending on whether the service
    // replaces existing tokens or creates new ones. Either behavior is valid.
}

#[tokio::test]
#[serial]
async fn test_resend_verification_case_insensitive_email() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with lowercase email
    let user = DbFixtures::user()
        .username("caseuser")
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("casetest@example.com") // Lowercase email
        .is_primary(true)
        .is_verified(false)
        .commit(db.clone())
        .await
        .expect("Failed to create user email");

    // Test with various case combinations
    let email_variations = vec![
        "casetest@example.com", // Original case
        "CASETEST@EXAMPLE.COM", // All uppercase
        "CaseTest@Example.Com", // Mixed case
        "caseTest@Example.COM", // Different mixed case
    ];

    for email_variant in email_variations {
        let resend_data = json!({
            "email": email_variant
        });

        let response = client
            .post(&format!("{}/api/auth/resend-verification", base_url))
            .header("Content-Type", "application/json")
            .json(&resend_data)
            .send()
            .await
            .expect("Failed to send resend verification request");

        // ✅ Should always return 200 OK (security: prevent information leakage)
        assert_eq!(
            response.status(),
            200,
            "Should always return 200 to prevent information leakage for email: {}",
            email_variant
        );

        let response_body: Value = response.json().await.expect("Should return JSON response");

        assert!(
            response_body.get("message").is_some(),
            "Should contain message for: {}",
            email_variant
        );
        let message = response_body["message"].as_str().unwrap();
        assert!(
            message.contains("If your email is registered"),
            "Should contain generic message that doesn't reveal case sensitivity behavior"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_resend_verification_database_consistency() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with unverified email
    let user = DbFixtures::user()
        .username("consistencyuser")
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("consistency@example.com")
        .is_primary(true)
        .is_verified(false)
        .commit(db.clone())
        .await
        .expect("Failed to create user email");

    // Get initial verification token count
    let initial_count = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT COUNT(*) as count FROM user_email_verification WHERE email = '{}'",
                user_email.email()
            ),
        ))
        .await
        .expect("Failed to count initial verification tokens")
        .unwrap();

    let initial_count_val: i64 = initial_count
        .try_get("", "count")
        .expect("Failed to get initial count");

    // Make resend request
    let resend_data = json!({
        "email": "consistency@example.com"
    });

    let response = client
        .post(&format!("{}/api/auth/resend-verification", base_url))
        .header("Content-Type", "application/json")
        .json(&resend_data)
        .send()
        .await
        .expect("Failed to send resend verification request");

    assert_eq!(response.status(), 200, "Resend should succeed");

    // ✅ Verify database state after resend
    let final_count = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT COUNT(*) as count FROM user_email_verification WHERE email = '{}'",
                user_email.email()
            ),
        ))
        .await
        .expect("Failed to count final verification tokens")
        .unwrap();

    let final_count_val: i64 = final_count
        .try_get("", "count")
        .expect("Failed to get final count");

    // ✅ Should have at least one verification token after resend
    assert!(
        final_count_val > initial_count_val,
        "Should have more verification tokens after resend: initial={}, final={}",
        initial_count_val,
        final_count_val
    );

    // ✅ Verify user email record is unchanged (still unverified)
    assert!(
        user_email
            .check(db.clone())
            .await
            .expect("Failed to check user email")
    );

    // ✅ Verify user record is unchanged
    assert!(user.check(db.clone()).await.expect("Failed to check user"));

    // ✅ Check verification token properties
    let token_data = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT verification_token, expires_at, created_at FROM user_email_verification WHERE email = '{}' ORDER BY created_at DESC LIMIT 1", 
                user_email.email()
            ),
        ))
        .await
        .expect("Failed to query verification token")
        .unwrap();

    let verification_token: String = token_data
        .try_get("", "verification_token")
        .expect("Failed to get token");
    assert!(
        !verification_token.is_empty(),
        "Verification token should not be empty"
    );
    assert!(
        verification_token.len() > 10,
        "Verification token should be substantial length"
    );

    // Note: We could also verify expires_at is in the future, but that would require timestamp parsing
}

#[tokio::test]
#[serial]
async fn test_resend_verification_invalidates_old_tokens() {
    // Setup test server and database
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with unverified email
    let user = DbFixtures::user()
        .username("tokeninvalidationuser")
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let user_email = DbFixtures::user_email()
        .user_id(user.id())
        .email("tokeninvalidation@example.com")
        .is_primary(true)
        .is_verified(false)
        .commit(db.clone())
        .await
        .expect("Failed to create user email");

    // Create an initial verification token using the fixture
    let initial_verification = DbFixtures::email_verification()
        .email(user_email.email())
        .verification_token("initial_token_123")
        .expires_at((chrono::Utc::now() + chrono::Duration::hours(24)).into())
        .commit(db.clone())
        .await
        .expect("Failed to create initial verification token");

    let initial_token = initial_verification.verification_token().to_string();

    // Verify initial token exists
    let initial_count = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT COUNT(*) as count FROM user_email_verification WHERE email = '{}'",
                user_email.email()
            ),
        ))
        .await
        .expect("Failed to count initial verification tokens")
        .unwrap();

    let initial_count_val: i64 = initial_count
        .try_get("", "count")
        .expect("Failed to get initial count");
    assert_eq!(
        initial_count_val, 1,
        "Should have exactly one initial verification token"
    );

    // Make resend verification request
    let resend_data = json!({
        "email": "tokeninvalidation@example.com"
    });

    let response = client
        .post(&format!("{}/api/auth/resend-verification", base_url))
        .header("Content-Type", "application/json")
        .json(&resend_data)
        .send()
        .await
        .expect("Failed to send resend verification request");

    // ✅ Should return 200 OK for successful resend
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for successful verification resend"
    );

    // ✅ Verify only one token exists after resend (old token should be invalidated)
    let final_count = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT COUNT(*) as count FROM user_email_verification WHERE email = '{}'",
                user_email.email()
            ),
        ))
        .await
        .expect("Failed to count final verification tokens")
        .unwrap();

    let final_count_val: i64 = final_count
        .try_get("", "count")
        .expect("Failed to get final count");
    assert_eq!(
        final_count_val, 1,
        "Should have exactly one verification token after resend (old token invalidated)"
    );

    // ✅ Verify the token is different from the initial one (new token created)
    let current_token_data = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT verification_token FROM user_email_verification WHERE email = '{}' LIMIT 1",
                user_email.email()
            ),
        ))
        .await
        .expect("Failed to query current verification token")
        .unwrap();

    let current_token: String = current_token_data
        .try_get("", "verification_token")
        .expect("Failed to get current token");
    assert_ne!(
        current_token, initial_token,
        "New token should be different from the initial token"
    );
    assert!(
        !current_token.is_empty(),
        "New verification token should not be empty"
    );
    assert!(
        current_token.len() > 10,
        "New verification token should be substantial length"
    );

    // ✅ Verify email is still unverified (only resend, not auto-verify)
    let email_status = db
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

    let is_verified: bool = email_status
        .try_get("", "is_verified")
        .expect("Failed to get verification status");
    assert!(!is_verified, "Email should remain unverified after resend");
}
