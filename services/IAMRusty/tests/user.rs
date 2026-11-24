// Include common test utilities and fixtures

mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
mod utils;

use std::sync::Arc;

use chrono::{Duration, Utc};
use common::*;
use fixtures::DbFixtures;
use serde_json::Value;
use serial_test::serial;
use uuid::Uuid;

use fixtures::db::users::UserFixture;
use iam_configuration::{load_config_part, JwtAlgorithm};
use iam_infra::repository::entity::user_emails::{
    Column as UserEmailColumn, Entity as UserEmailsEntity, Model as UserEmailRow,
};
use iam_infra::repository::entity::users::{Entity as UsersEntity, Model as UserRow};
use reqwest::Client;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    SqlErr,
};

/// Create a valid JWT token for testing using the proper JWT service
async fn create_valid_jwt_token(user_id: Uuid, config: &iam_configuration::JwtConfig) -> String {
    utils::jwt::create_valid_jwt_token_with_encoder(user_id, config)
        .await
        .expect("Failed to create valid JWT token")
}

/// Create an expired JWT token for testing using the proper JWT service
async fn create_expired_jwt_token(user_id: Uuid, config: &iam_configuration::JwtConfig) -> String {
    utils::jwt::create_expired_jwt_token_with_encoder(user_id, config)
        .await
        .expect("Failed to create expired JWT token")
}

/// Create an invalid JWT token for testing using the proper JWT service
async fn create_invalid_signature_jwt_token(
    user_id: Uuid,
    config: &iam_configuration::JwtConfig,
) -> String {
    utils::jwt::create_invalid_jwt_token_with_encoder(user_id, config)
        .await
        .expect("Failed to create invalid JWT token")
}

// 👤 STORY-012 — own profile (E1 view, E2 update, E3 rejection)
//
// The profile is read with GET /api/me and saved with PATCH /api/me. Only
// `username` and `avatar_url` are writable; a field that is not submitted keeps
// its saved value. "Unchanged" is proven on the stored rows, not only on the
// response, so a rejected request cannot leave a partial save behind.

/// Read the stored `users` row so that "unchanged" compares the whole row.
async fn load_user_row(db: &Arc<DatabaseConnection>, user_id: Uuid) -> UserRow {
    UsersEntity::find_by_id(user_id)
        .one(db.as_ref())
        .await
        .expect("Reading the users row should succeed")
        .unwrap_or_else(|| panic!("user {user_id} should still exist"))
}

/// Read every stored email address of a user, ordered by address.
async fn load_email_rows(db: &Arc<DatabaseConnection>, user_id: Uuid) -> Vec<UserEmailRow> {
    UserEmailsEntity::find()
        .filter(UserEmailColumn::UserId.eq(user_id))
        .order_by_asc(UserEmailColumn::Email)
        .all(db.as_ref())
        .await
        .expect("Reading the user_emails rows should succeed")
}

/// Sign the access token a signed-in person presents for `user_id`.
async fn access_token_for(user_id: Uuid) -> String {
    create_valid_jwt_token(
        user_id,
        &load_config_part::<iam_configuration::JwtConfig>("jwt")
            .expect("Failed to load JWT config"),
    )
    .await
}

/// Seed Arthur with his GitHub avatar and primary email address.
async fn seed_arthur(db: &Arc<DatabaseConnection>) -> UserFixture {
    let user = DbFixtures::user()
        .arthur()
        .password_hash("profile-test-password-hash")
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");
    DbFixtures::user_email()
        .arthur_secondary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create secondary email");
    user
}

/// Seed a second account (Bob) so that a username can belong to someone else.
async fn seed_bob(db: &Arc<DatabaseConnection>) -> UserFixture {
    let user = DbFixtures::user()
        .bob()
        .commit(db.clone())
        .await
        .expect("Failed to create Bob");
    DbFixtures::user_email()
        .bob_primary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create Bob's primary email");
    user
}

/// The avatar the fixture saved for `user`.
fn saved_avatar(user: &UserFixture) -> &str {
    user.avatar_url().map_or_else(
        || panic!("fixture user should have a saved avatar"),
        String::as_str,
    )
}

/// Save changes to the caller's own profile.
async fn patch_me(client: &Client, base_url: &str, token: &str, body: Value) -> reqwest::Response {
    client
        .patch(format!("{base_url}/api/me"))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .expect("Failed to send PATCH /api/me")
}

/// Open the caller's own profile.
async fn get_me(client: &Client, base_url: &str, token: &str) -> Value {
    let response = client
        .get(format!("{base_url}/api/me"))
        .bearer_auth(token)
        .send()
        .await
        .expect("Failed to send GET /api/me");
    assert_eq!(
        response.status(),
        200,
        "Opening the profile should return 200 OK"
    );
    response.json().await.expect("The profile should be JSON")
}

/// Assert a rejection: its HTTP status, its machine-readable code and the
/// explanation of what needs correcting.
async fn expect_rejection(
    response: reqwest::Response,
    status: u16,
    error_code: &str,
    message: &str,
) {
    let actual_status = response.status().as_u16();
    let text = response
        .text()
        .await
        .expect("Should be able to read the rejection body");
    assert_eq!(
        actual_status, status,
        "Expected {status} {error_code}, got {actual_status}; body: {text}"
    );
    let body: Value = serde_json::from_str(&text).unwrap_or_else(|error| {
        panic!("The rejection should be the uniform JSON error ({error}); body: {text}")
    });
    assert_eq!(
        body["error"]["status"], status,
        "The error should carry its HTTP status; body: {text}"
    );
    assert_eq!(
        body["error"]["error_code"], error_code,
        "The error code should say what was rejected; body: {text}"
    );
    assert_eq!(
        body["error"]["message"], message,
        "The error should explain what needs correcting; body: {text}"
    );
}

// 👤 User Endpoint Tests
// 🔁 /me

#[tokio::test]
#[serial]
async fn test_get_user_returns_correct_info_when_token_is_valid() {
    // Setup test environment
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();

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

    // Create valid JWT token using proper configuration
    let jwt_token = create_valid_jwt_token(
        user.id(),
        &load_config_part::<iam_configuration::JwtConfig>("jwt")
            .expect("Failed to load JWT config"),
    )
    .await;

    // Make request to /me endpoint
    let response = client
        .get(format!("{base_url}/api/me"))
        .header("Authorization", format!("Bearer {jwt_token}"))
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
    // STORY-012 E1: return the saved avatar, not merely a nullable string.
    assert_eq!(
        response_json["avatar_url"], "https://avatars.githubusercontent.com/u/123456",
        "Should return Arthur's saved avatar URL"
    );
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_token_is_expired() {
    // Setup test environment
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create expired JWT token
    let user_id = Uuid::new_v4();
    let expired_token = create_expired_jwt_token(
        user_id,
        &load_config_part::<iam_configuration::JwtConfig>("jwt")
            .expect("Failed to load JWT config"),
    )
    .await;

    // Make request with expired token
    let response = client
        .get(format!("{base_url}/api/me"))
        .header("Authorization", format!("Bearer {expired_token}"))
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
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
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
            .get(format!("{base_url}/api/me"))
            .header("Authorization", format!("Bearer {malformed_token}"))
            .send()
            .await
            .expect("Failed to send request");

        // Should return 401 for malformed tokens (authentication failures)
        assert_eq!(
            response.status(),
            expected_status,
            "Should return {expected_status} for malformed token: '{malformed_token}'"
        );

        if expected_status == 401 {
            // Middleware returns plain StatusCode for auth failures
            let response_text = response
                .text()
                .await
                .expect("Should be able to read response text");
            assert!(
                response_text.is_empty() || response_text.len() <= 50,
                "Should return minimal response body for malformed token: '{malformed_token}'"
            );
        } else {
            // Service returns JSON error (this branch shouldn't be used anymore)
            let response_json: Value = response
                .json()
                .await
                .expect("Should return JSON error response");

            assert!(
                response_json["error"].is_object(),
                "Should return error object for malformed token: '{malformed_token}'"
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_no_authorization_header() {
    // Setup test environment
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Make request without Authorization header
    let response = client
        .get(format!("{base_url}/api/me"))
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
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
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
            .get(format!("{base_url}/api/me"))
            .header("Authorization", invalid_header)
            .send()
            .await
            .expect("Failed to send request");

        // ❌ Should return 401 Unauthorized for invalid header format
        assert_eq!(
            response.status(),
            401,
            "Should return 401 for invalid header format: '{invalid_header}'"
        );

        // The middleware returns plain StatusCode without JSON body
        let response_text = response
            .text()
            .await
            .expect("Should be able to read response text");
        assert!(
            response_text.is_empty() || response_text.len() <= 50,
            "Should return minimal response body for invalid header: '{invalid_header}'"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_get_user_returns_401_when_user_not_found_in_database() {
    // Setup test environment
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create valid JWT token for non-existent user
    let non_existent_user_id = Uuid::new_v4();
    let jwt_token = create_valid_jwt_token(
        non_existent_user_id,
        &load_config_part::<iam_configuration::JwtConfig>("jwt")
            .expect("Failed to load JWT config"),
    )
    .await;

    // Make request with token for non-existent user
    let _response = client
        .get(format!("{base_url}/api/me"))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .send()
        .await
        .expect("Failed to send request");

    // Re-make the request to get the JSON response
    let response = client
        .get(format!("{base_url}/api/me"))
        .header("Authorization", format!("Bearer {jwt_token}"))
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
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();

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
    let jwt_token = create_valid_jwt_token(
        user.id(),
        &load_config_part::<iam_configuration::JwtConfig>("jwt")
            .expect("Failed to load JWT config"),
    )
    .await;

    // Make request to /me endpoint
    let response = client
        .get(format!("{base_url}/api/me"))
        .header("Authorization", format!("Bearer {jwt_token}"))
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
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();

    // Pre-create user without any emails
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    // Create valid JWT token
    let jwt_token = create_valid_jwt_token(
        user.id(),
        &load_config_part::<iam_configuration::JwtConfig>("jwt")
            .expect("Failed to load JWT config"),
    )
    .await;

    // Make request to /me endpoint
    let response = client
        .get(format!("{base_url}/api/me"))
        .header("Authorization", format!("Bearer {jwt_token}"))
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
    let (fixture, base_url, _client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();

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
    let jwt_token = create_valid_jwt_token(
        user.id(),
        &load_config_part::<iam_configuration::JwtConfig>("jwt")
            .expect("Failed to load JWT config"),
    )
    .await;

    // Make multiple concurrent requests with the same token
    let mut handles = vec![];

    for i in 0..5 {
        let base_url = base_url.clone();
        let token = jwt_token.clone();

        let handle = tokio::spawn(async move {
            let client2 = create_test_client();
            let response = client2
                .get(format!("{base_url}/api/me"))
                .header("Authorization", format!("Bearer {token}"))
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
        assert_eq!(status, 200, "Request {request_id} should return 200 OK");

        let response_json = response_result.expect("Should return JSON response");
        assert_eq!(
            response_json["username"], "arthur",
            "Request {request_id} should return correct username"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_get_user_security_jwt_claims_validation() {
    // Setup test environment
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();

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
    let config =
        load_config_part::<iam_configuration::JwtConfig>("jwt").expect("Failed to load JWT config");
    let jwt_algorithm_config = config
        .create_jwt_algorithm()
        .expect("Failed to create JWT algorithm");

    // Get algorithm and keys for manual token creation
    let (algorithm, encoding_key) = match jwt_algorithm_config {
        JwtAlgorithm::HS256(secret) => (
            jsonwebtoken::Algorithm::HS256,
            jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
        ),
        JwtAlgorithm::RS256(key_pair) => (
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
            .get(format!("{base_url}/api/me"))
            .header("Authorization", format!("Bearer {invalid_token}"))
            .send()
            .await
            .expect("Failed to send request");

        // ❌ Should return 401 for invalid claims (authentication failure)
        assert_eq!(
            response.status(),
            401,
            "Should return 401 for {description}"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_get_user_shows_no_avatar_when_none_is_saved() {
    // STORY-012 E1: the profile shows the saved avatar, including when none is saved.
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();

    let user = DbFixtures::user()
        .no_avatar()
        .commit(db.clone())
        .await
        .expect("Failed to create user without avatar");
    DbFixtures::user_email()
        .user_id(user.id())
        .email("no-avatar@example.com")
        .with_primary(true)
        .with_verified(true)
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");

    let jwt_token = access_token_for(user.id()).await;

    let response = client
        .get(format!("{base_url}/api/me"))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for valid token"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    assert_eq!(
        response_json["username"], "no_avatar_user",
        "Should return the saved username"
    );
    assert_eq!(response_json.get("avatar_url"), Some(&Value::Null));
    assert_eq!(
        response_json["email"], "no-avatar@example.com",
        "Should return primary email"
    );
}

// 🔁 PATCH /api/me — STORY-012 E2: save a valid change

#[tokio::test]
#[serial]
async fn test_update_profile_saves_a_new_username_and_keeps_the_other_fields() {
    // STORY-012 E2: only the username is submitted; the avatar and the email stay the same.
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();
    let user = seed_arthur(&db).await;
    let token = access_token_for(user.id()).await;
    let row_before = load_user_row(&db, user.id()).await;
    let emails_before = load_email_rows(&db, user.id()).await;

    let body = serde_json::json!({ "username": "arthur_renamed" });
    let response = patch_me(&client, &base_url, &token, body).await;

    assert_eq!(
        response.status(),
        200,
        "Saving a valid username should return 200 OK"
    );
    let body: Value = response
        .json()
        .await
        .expect("The saved profile should be JSON");
    assert_eq!(
        body["id"],
        user.id().to_string(),
        "Should be the same account"
    );
    assert_eq!(
        body["username"], "arthur_renamed",
        "The profile should show the new username"
    );
    assert_eq!(
        body["avatar_url"],
        saved_avatar(&user),
        "The untouched avatar should stay the same"
    );
    assert_eq!(
        body["email"], "arthur@example.com",
        "The untouched email should stay the same"
    );

    // Opening the profile again shows the saved values.
    let shown = get_me(&client, &base_url, &token).await;
    assert_eq!(
        shown["username"], "arthur_renamed",
        "The saved username should be shown"
    );
    assert_eq!(
        shown["avatar_url"],
        saved_avatar(&user),
        "The saved avatar should be shown"
    );
    assert_eq!(
        shown["email"], "arthur@example.com",
        "The saved email should be shown"
    );

    // Only the username column changed.
    let row_after = load_user_row(&db, user.id()).await;
    assert_eq!(
        row_after.username.as_deref(),
        Some("arthur_renamed"),
        "The new username should be saved"
    );
    assert_eq!(
        row_after.avatar_url, row_before.avatar_url,
        "The saved avatar should stay the same"
    );
    assert_eq!(
        row_after.password_hash, row_before.password_hash,
        "Credentials are not part of a profile update"
    );
    assert_eq!(
        (row_after.id, row_after.created_at),
        (row_before.id, row_before.created_at),
        "The account identity should stay the same"
    );
    assert_eq!(
        load_email_rows(&db, user.id()).await,
        emails_before,
        "No email address should be added or changed"
    );
}

#[tokio::test]
#[serial]
async fn test_update_profile_saves_a_new_avatar_and_keeps_the_other_fields() {
    // STORY-012 E2: only the avatar is submitted; the username and the email stay the same.
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();
    let user = seed_arthur(&db).await;
    let token = access_token_for(user.id()).await;
    let row_before = load_user_row(&db, user.id()).await;
    let emails_before = load_email_rows(&db, user.id()).await;
    let new_avatar = "https://avatars.example.test/arthur-new.png";

    let body = serde_json::json!({ "avatar_url": new_avatar });
    let response = patch_me(&client, &base_url, &token, body).await;

    assert_eq!(
        response.status(),
        200,
        "Saving a valid avatar should return 200 OK"
    );
    let body: Value = response
        .json()
        .await
        .expect("The saved profile should be JSON");
    assert_eq!(
        body["avatar_url"], new_avatar,
        "The profile should show the new avatar"
    );
    assert_eq!(
        body["username"], "arthur",
        "The untouched username should stay the same"
    );
    assert_eq!(
        body["email"], "arthur@example.com",
        "The untouched email should stay the same"
    );

    // Opening the profile again shows the saved values.
    let shown = get_me(&client, &base_url, &token).await;
    assert_eq!(
        shown["avatar_url"], new_avatar,
        "The saved avatar should be shown"
    );
    assert_eq!(
        shown["username"], "arthur",
        "The saved username should be shown"
    );

    // Only the avatar column changed.
    let row_after = load_user_row(&db, user.id()).await;
    assert_eq!(
        row_after.avatar_url.as_deref(),
        Some(new_avatar),
        "The new avatar should be saved"
    );
    assert_eq!(
        row_after.username, row_before.username,
        "The saved username should stay the same"
    );
    assert_eq!(
        row_after.password_hash, row_before.password_hash,
        "Credentials are not part of a profile update"
    );
    assert_eq!(
        load_email_rows(&db, user.id()).await,
        emails_before,
        "No email address should be added or changed"
    );
}

#[tokio::test]
#[serial]
async fn test_update_profile_saves_both_fields_but_never_the_email_or_another_account() {
    // STORY-012 E2 (both fields) and E1: "The email address is not editable in this flow."
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();
    let user = seed_arthur(&db).await;
    let other = seed_bob(&db).await;
    let token = access_token_for(user.id()).await;
    let row_before = load_user_row(&db, user.id()).await;
    let emails_before = load_email_rows(&db, user.id()).await;
    let other_row_before = load_user_row(&db, other.id()).await;
    let other_emails_before = load_email_rows(&db, other.id()).await;
    let new_avatar = "https://avatars.example.test/arthur-both.png";

    let body = serde_json::json!({
        "username": "arthur_renamed",
        "avatar_url": new_avatar,
    });
    let response = patch_me(&client, &base_url, &token, body).await;

    assert_eq!(
        response.status(),
        200,
        "Saving valid changes should return 200 OK"
    );
    let body: Value = response
        .json()
        .await
        .expect("The saved profile should be JSON");
    assert_eq!(
        body["id"],
        user.id().to_string(),
        "The profile is the caller's own"
    );
    assert_eq!(
        body["username"], "arthur_renamed",
        "The profile should show the new username"
    );
    assert_eq!(
        body["avatar_url"], new_avatar,
        "The profile should show the new avatar"
    );
    assert_eq!(
        body["email"], "arthur@example.com",
        "The email address is not editable in this flow"
    );

    // Opening the profile again shows the saved values and the unchanged email.
    let shown = get_me(&client, &base_url, &token).await;
    assert_eq!(
        shown["username"], "arthur_renamed",
        "The saved username should be shown"
    );
    assert_eq!(
        shown["avatar_url"], new_avatar,
        "The saved avatar should be shown"
    );
    assert_eq!(
        shown["email"], "arthur@example.com",
        "Opening the profile shows the unchanged email"
    );

    let row_after = load_user_row(&db, user.id()).await;
    assert_eq!(
        (
            row_after.username.as_deref(),
            row_after.avatar_url.as_deref()
        ),
        (Some("arthur_renamed"), Some(new_avatar)),
        "Both submitted fields should be saved"
    );
    assert_eq!(
        row_after.password_hash, row_before.password_hash,
        "Credentials are not part of a profile update"
    );
    assert_eq!(
        load_email_rows(&db, user.id()).await,
        emails_before,
        "No email address should be added or changed"
    );
    // Only the caller's profile is updated.
    assert_eq!(
        load_user_row(&db, other.id()).await,
        other_row_before,
        "Another account's profile must not change"
    );
    assert_eq!(
        load_email_rows(&db, other.id()).await,
        other_emails_before,
        "Another account's email must not change"
    );
}

#[tokio::test]
#[serial]
async fn test_update_profile_with_no_field_to_change_saves_nothing() {
    // STORY-012 E2: "Untouched fields stay the same" — nothing submitted, nothing saved.
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();
    let user = seed_arthur(&db).await;
    let token = access_token_for(user.id()).await;
    let row_before = load_user_row(&db, user.id()).await;
    let emails_before = load_email_rows(&db, user.id()).await;

    let body = serde_json::json!({});
    let response = patch_me(&client, &base_url, &token, body).await;

    assert_eq!(
        response.status(),
        200,
        "An update with nothing to change should return 200 OK"
    );
    let body: Value = response.json().await.expect("The profile should be JSON");
    assert_eq!(
        body["username"], "arthur",
        "The saved username should be shown"
    );
    assert_eq!(
        body["avatar_url"],
        saved_avatar(&user),
        "The saved avatar should be shown"
    );
    assert_eq!(
        body["email"], "arthur@example.com",
        "The saved email should be shown"
    );
    assert_eq!(
        load_user_row(&db, user.id()).await,
        row_before,
        "The saved profile should not change"
    );
    assert_eq!(
        load_email_rows(&db, user.id()).await,
        emails_before,
        "No email address should be added or changed"
    );
}

// 🔁 PATCH /api/me — STORY-012 E3: a rejected update changes nothing

#[tokio::test]
#[serial]
async fn test_update_profile_rejects_a_username_of_another_account_and_saves_nothing() {
    // STORY-012 E3: "the proposed username is already used by another account".
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();
    let user = seed_arthur(&db).await;
    let other = seed_bob(&db).await;
    let token = access_token_for(user.id()).await;
    let row_before = load_user_row(&db, user.id()).await;
    let emails_before = load_email_rows(&db, user.id()).await;
    let other_row_before = load_user_row(&db, other.id()).await;

    // The avatar sent with the taken username is valid on its own: it must not be saved either.
    let body = serde_json::json!({
        "username": "bob",
        "avatar_url": "https://avatars.example.test/must-not-save.png",
    });
    let response = patch_me(&client, &base_url, &token, body).await;

    expect_rejection(response, 409, "username_taken", "Username is already taken").await;

    // The saved profile did not change: neither the username nor the valid avatar sent with it.
    let shown = get_me(&client, &base_url, &token).await;
    assert_eq!(
        shown["username"], "arthur",
        "The saved username should not change"
    );
    assert_eq!(
        shown["avatar_url"],
        saved_avatar(&user),
        "The submitted avatar should not be saved"
    );
    assert_eq!(
        load_user_row(&db, user.id()).await,
        row_before,
        "A rejected update must not change the saved profile"
    );
    assert_eq!(
        load_email_rows(&db, user.id()).await,
        emails_before,
        "A rejected update must not change the email"
    );
    assert_eq!(
        load_user_row(&db, other.id()).await,
        other_row_before,
        "The other account must not change"
    );
}

#[tokio::test]
#[serial]
async fn test_update_profile_rejects_an_invalid_username_and_saves_nothing() {
    // STORY-012 E3: "a submitted value is invalid" — the username rule is the one applied at signup.
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();
    let user = seed_arthur(&db).await;
    let token = access_token_for(user.id()).await;
    let row_before = load_user_row(&db, user.id()).await;
    let emails_before = load_email_rows(&db, user.id()).await;

    let too_long = "a".repeat(51);
    let invalid_usernames = [
        Value::Null,
        Value::from("ab"),
        Value::from("arthur renamed"),
        Value::from("12345"),
        Value::from("arthur!"),
        Value::from(too_long),
    ];
    for invalid_username in invalid_usernames {
        // A valid avatar is sent too: it must not be saved while the username is rejected.
        let body = serde_json::json!({
            "username": invalid_username,
            "avatar_url": "https://avatars.example.test/must-not-save.png",
        });
        let response = patch_me(&client, &base_url, &token, body).await;

        expect_rejection(response, 422, "invalid_username", "Invalid username format").await;
        assert_eq!(
            load_user_row(&db, user.id()).await,
            row_before,
            "A rejected update must not change the saved profile (username '{invalid_username}')"
        );
    }

    assert_eq!(
        load_email_rows(&db, user.id()).await,
        emails_before,
        "A rejected update must not change the email"
    );
    let shown = get_me(&client, &base_url, &token).await;
    assert_eq!(
        shown["username"], "arthur",
        "The saved username should not change"
    );
    assert_eq!(
        shown["avatar_url"],
        saved_avatar(&user),
        "The submitted avatar should not be saved"
    );
}

#[tokio::test]
#[serial]
async fn test_update_profile_omits_legacy_avatar_without_revalidating_then_clears_it_explicitly() {
    let (fixture, base_url, client) = Box::pin(setup_test_server()).await.unwrap();
    let db = fixture.db();
    let legacy_avatar = "http://avatars.example.test/legacy.png";
    let user = DbFixtures::user()
        .arthur()
        .avatar_url(Some(legacy_avatar.into()))
        .commit(db.clone())
        .await
        .unwrap();
    DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await
        .unwrap();
    let token = access_token_for(user.id()).await;
    let emails = load_email_rows(&db, user.id()).await;

    let renamed = patch_me(
        &client,
        &base_url,
        &token,
        serde_json::json!({"username": "arthur_renamed"}),
    )
    .await;
    assert_eq!(
        renamed.status(),
        200,
        "Omission must preserve a legacy avatar"
    );
    assert_eq!(
        get_me(&client, &base_url, &token).await["avatar_url"],
        legacy_avatar
    );

    let cleared = patch_me(
        &client,
        &base_url,
        &token,
        serde_json::json!({"avatar_url": null}),
    )
    .await;
    assert_eq!(cleared.status(), 200, "Explicit null must clear the avatar");
    let shown = get_me(&client, &base_url, &token).await;
    assert_eq!(shown.get("avatar_url"), Some(&Value::Null));
    assert_eq!(shown["username"], "arthur_renamed");
    assert_eq!(shown["email"], "arthur@example.com");
    assert_eq!(load_user_row(&db, user.id()).await.avatar_url, None);
    assert_eq!(load_email_rows(&db, user.id()).await, emails);
}

#[tokio::test]
#[serial]
async fn test_update_profile_rejects_invalid_avatars_without_saving_the_valid_username() {
    let (fixture, base_url, client) = Box::pin(setup_test_server()).await.unwrap();
    let db = fixture.db();
    let user = seed_arthur(&db).await;
    let token = access_token_for(user.id()).await;
    let before = (
        load_user_row(&db, user.id()).await,
        load_email_rows(&db, user.id()).await,
    );
    for avatar in [
        "",
        "/relative.png",
        "http://avatars.example.test/avatar.png",
        "https://reader@avatars.example.test/avatar.png",
        "https://reader:sample@avatars.example.test/avatar.png",
        "https://",
    ] {
        let response = patch_me(
            &client,
            &base_url,
            &token,
            serde_json::json!({
                "username": "arthur_renamed", "avatar_url": avatar
            }),
        )
        .await;
        expect_rejection(
            response,
            422,
            "invalid_avatar_url",
            "Avatar must be an absolute HTTPS URL without user information",
        )
        .await;
        assert_eq!(
            (
                load_user_row(&db, user.id()).await,
                load_email_rows(&db, user.id()).await
            ),
            before
        );
    }
}

#[tokio::test]
#[serial]
async fn test_update_profile_enforces_avatar_limit_in_utf8_bytes() {
    let (fixture, base_url, client) = Box::pin(setup_test_server()).await.unwrap();
    fixture_cleanup::run(&fixture, async {
        let db = fixture.db();
        let user = seed_arthur(&db).await;
        let token = access_token_for(user.id()).await;
        let prefix = "https://avatars.example.test/é";
        let at_limit = format!("{prefix}{}", "a".repeat(2048 - prefix.len()));
        let over_limit = format!("{at_limit}a");

        let response = patch_me(
            &client,
            &base_url,
            &token,
            serde_json::json!({"avatar_url": at_limit}),
        )
        .await;
        assert_eq!(response.status(), 200, "2048 UTF-8 bytes are allowed");
        let before = load_user_row(&db, user.id()).await;
        assert_eq!(before.avatar_url.as_deref(), Some(at_limit.as_str()));
        let emails = load_email_rows(&db, user.id()).await;

        let response = patch_me(
            &client,
            &base_url,
            &token,
            serde_json::json!({"username": "arthur_renamed", "avatar_url": over_limit}),
        )
        .await;
        expect_rejection(
            response,
            422,
            "invalid_avatar_url",
            "Avatar must be an absolute HTTPS URL without user information",
        )
        .await;
        assert_eq!(load_user_row(&db, user.id()).await, before);
        assert_eq!(load_email_rows(&db, user.id()).await, emails);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_update_profile_rejects_unsupported_keys_and_leaves_both_accounts_unchanged() {
    let (fixture, base_url, client) = Box::pin(setup_test_server()).await.unwrap();
    let db = fixture.db();
    let user = seed_arthur(&db).await;
    let other = seed_bob(&db).await;
    let token = access_token_for(user.id()).await;
    let before = (
        load_user_row(&db, user.id()).await,
        load_email_rows(&db, user.id()).await,
    );
    let other_before = (
        load_user_row(&db, other.id()).await,
        load_email_rows(&db, other.id()).await,
    );

    for (key, value) in [
        ("email", Value::from("replacement@example.com")),
        ("id", Value::from(other.id().to_string())),
        ("user_id", Value::from(other.id().to_string())),
        ("unexpected", Value::Null),
    ] {
        let mut body = serde_json::json!({"username": "arthur_renamed"});
        body[key] = value;
        let response = patch_me(&client, &base_url, &token, body).await;
        expect_rejection(
            response,
            422,
            "invalid_json_data",
            "Invalid JSON data in request body",
        )
        .await;
        assert_eq!(
            (
                load_user_row(&db, user.id()).await,
                load_email_rows(&db, user.id()).await
            ),
            before
        );
        assert_eq!(
            (
                load_user_row(&db, other.id()).await,
                load_email_rows(&db, other.id()).await
            ),
            other_before
        );
    }
}

#[tokio::test]
#[serial]
async fn test_profile_username_uniqueness_is_an_invariant_on_the_database_writer() {
    let (fixture, _, _) = Box::pin(setup_test_server()).await.unwrap();
    let db = fixture.db();
    let arthur = seed_arthur(&db).await;
    let bob = seed_bob(&db).await;
    let bob_before = load_user_row(&db, bob.id()).await;
    let mut direct_update: iam_infra::repository::entity::users::ActiveModel =
        bob_before.clone().into();
    direct_update.username = Set(Some("arthur".into()));
    let result = direct_update.update(db.as_ref()).await;
    assert!(
        matches!(result, Err(ref error) if matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_)))),
        "The database writer must reject a duplicate username independently of the HTTP path"
    );
    assert_eq!(load_user_row(&db, bob.id()).await, bob_before);
    assert_eq!(
        load_user_row(&db, arthur.id()).await.username.as_deref(),
        Some("arthur")
    );
}

#[tokio::test]
#[serial]
async fn test_update_profile_concurrent_claims_have_one_winner_and_no_partial_save() {
    let (fixture, base_url, client) = Box::pin(setup_test_server()).await.unwrap();
    let db = fixture.db();
    let arthur = seed_arthur(&db).await;
    let bob = seed_bob(&db).await;
    let arthur_token = access_token_for(arthur.id()).await;
    let bob_token = access_token_for(bob.id()).await;
    let arthur_before = load_user_row(&db, arthur.id()).await;
    let bob_before = load_user_row(&db, bob.id()).await;
    let body = serde_json::json!({
        "username": "same_new_name",
        "avatar_url": "https://avatars.example.test/new.png",
    });

    let (a, b) = tokio::join!(
        patch_me(&client, &base_url, &arthur_token, body.clone()),
        patch_me(&client, &base_url, &bob_token, body),
    );
    let (rejected, winner, unchanged) = match (a.status().as_u16(), b.status().as_u16()) {
        (200, 409) => (b, arthur.id(), bob_before),
        (409, 200) => (a, bob.id(), arthur_before),
        statuses => panic!("Expected one save and one conflict, got {statuses:?}"),
    };
    expect_rejection(rejected, 409, "username_taken", "Username is already taken").await;
    assert_eq!(load_user_row(&db, unchanged.id).await, unchanged);
    let saved = load_user_row(&db, winner).await;
    assert_eq!(saved.username.as_deref(), Some("same_new_name"));
    assert_eq!(
        saved.avatar_url.as_deref(),
        Some("https://avatars.example.test/new.png")
    );
}

#[tokio::test]
#[serial]
async fn test_update_profile_keeps_the_callers_own_username_available_to_them() {
    // STORY-012 E3 rejects a username used by *another* account: keeping your own is no conflict.
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();
    let user = seed_arthur(&db).await;
    let other = seed_bob(&db).await;
    let token = access_token_for(user.id()).await;
    let other_row_before = load_user_row(&db, other.id()).await;
    let new_avatar = "https://avatars.example.test/arthur-kept.png";

    let body = serde_json::json!({ "username": "arthur", "avatar_url": new_avatar });
    let response = patch_me(&client, &base_url, &token, body).await;

    assert_eq!(
        response.status(),
        200,
        "Keeping your own username is not a conflict"
    );
    let body: Value = response
        .json()
        .await
        .expect("The saved profile should be JSON");
    assert_eq!(
        body["username"], "arthur",
        "The profile should keep the username"
    );
    assert_eq!(
        body["avatar_url"], new_avatar,
        "The valid change sent with it should be saved"
    );
    let row_after = load_user_row(&db, user.id()).await;
    assert_eq!(
        (
            row_after.username.as_deref(),
            row_after.avatar_url.as_deref()
        ),
        (Some("arthur"), Some(new_avatar)),
        "The username stays and the avatar is saved"
    );
    assert_eq!(
        load_user_row(&db, other.id()).await,
        other_row_before,
        "The other account must not change"
    );
}

// 🔁 PATCH /api/me — only a signed-in person can save their own profile

#[tokio::test]
#[serial]
async fn test_update_profile_requires_a_valid_token_and_saves_nothing_without_one() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let db = fixture.db();
    let user = seed_arthur(&db).await;
    let row_before = load_user_row(&db, user.id()).await;
    let body = serde_json::json!({ "username": "arthur_renamed" });

    // Without an Authorization header
    let response = client
        .patch(format!("{base_url}/api/me"))
        .json(&body)
        .send()
        .await
        .expect("Failed to send request without Authorization header");
    assert_eq!(
        response.status(),
        401,
        "Should return 401 for a missing Authorization header"
    );

    // With a token that is not a valid JWT
    let response = client
        .patch(format!("{base_url}/api/me"))
        .bearer_auth("not.a.jwt")
        .json(&body)
        .send()
        .await
        .expect("Failed to send request with a malformed token");
    assert_eq!(
        response.status(),
        401,
        "Should return 401 for a malformed token"
    );

    assert_eq!(
        load_user_row(&db, user.id()).await,
        row_before,
        "An unauthenticated update must not change the saved profile"
    );
}

#[tokio::test]
#[serial]
async fn test_update_profile_returns_401_when_user_not_found_in_database() {
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");
    let token = access_token_for(Uuid::new_v4()).await;

    let body = serde_json::json!({ "username": "ghost_user" });
    let response = patch_me(&client, &base_url, &token, body).await;

    // Same refusal as opening the profile of an account that does not exist.
    assert_eq!(
        response.status(),
        401,
        "Should return 401 when user not found"
    );
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON error response");
    assert_eq!(
        response_json["error"]["error_code"], "user_not_found",
        "Error code should be user_not_found"
    );
}
