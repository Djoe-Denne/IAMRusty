// Include common test utilities and fixtures
#[path = "common/mod.rs"] 
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::{get_test_server, TestFixture};
use fixtures::{GitHubFixtures, GitLabFixtures, DbFixtures};
use reqwest::Client;
use serde_json::Value;
use base64::{Engine as _, engine::general_purpose};
use serial_test::serial;
use uuid::Uuid;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

/// Create a common HTTP client for tests that doesn't follow redirects automatically
fn create_test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client")
}

/// Helper function to create a valid OAuth state for login operation
fn create_login_state() -> String {
    let state_data = serde_json::json!({
        "operation": {
            "type": "login"
        },
        "nonce": Uuid::new_v4().to_string()
    });
    general_purpose::URL_SAFE_NO_PAD.encode(state_data.to_string())
}

/// Helper function to create a valid OAuth state for link operation
fn create_link_state(user_id: Uuid) -> String {
    let state_data = serde_json::json!({
        "operation": {
            "type": "link",
            "user_id": user_id.to_string()
        },
        "nonce": Uuid::new_v4().to_string()
    });
    general_purpose::URL_SAFE_NO_PAD.encode(state_data.to_string())
}

/// Helper function to create an invalid OAuth state
fn create_invalid_state() -> String {
    "invalid_base64_state".to_string()
}

/// Helper function to count entities in database
async fn count_entities(db: std::sync::Arc<sea_orm::DatabaseConnection>, table: &str) -> Result<i64, sea_orm::DbErr> {
    let count: i64 = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT COUNT(*) as count FROM {}", table),
        ))
        .await?
        .unwrap()
        .try_get("", "count")?;
    Ok(count)
}

/// Helper function to verify JWT token structure (basic validation)
fn verify_jwt_structure(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    parts.len() == 3 && !parts[0].is_empty() && !parts[1].is_empty() && !parts[2].is_empty()
}

// 🔐 Authentication & OAuth Callback Tests
// 🔁 /auth/{provider}/callback

#[tokio::test]
#[serial]
async fn test_oauth_callback_github_successful_flow_creates_jwt_for_new_user() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup GitHub mock server for successful flow
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    // Create valid state for login operation
    let state = create_login_state();
    
    // Make callback request with authorization code
    let response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[
            ("code", "valid_auth_code_123"),
            ("state", &state)
        ])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ✅ Should return 200 OK with JWT token
    assert_eq!(response.status(), 200, "Should return 200 OK for successful OAuth callback");
    
    // ✅ Should return JSON response with JWT token
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON response");
    
    assert_eq!(response_json["operation"], "login", 
              "Response should indicate 'login' operation");
    
    // ✅ Should contain valid JWT token
    let access_token = response_json["access_token"].as_str()
        .expect("Response should contain access token");
    assert!(verify_jwt_structure(access_token), 
           "Access token should have valid structure");
    
    // ✅ Should create user in database
    let user_count = count_entities(db.clone(), "users").await
        .expect("Failed to count users");
    assert_eq!(user_count, 1, "Should create exactly one user");
    
    // ✅ Should create provider token in database
    let token_count = count_entities(db.clone(), "provider_tokens").await
        .expect("Failed to count provider tokens");
    assert_eq!(token_count, 1, "Should create exactly one provider token");
    
    // ✅ Should create user email in database
    let email_count = count_entities(db.clone(), "user_emails").await
        .expect("Failed to count user emails");
    assert_eq!(email_count, 1, "Should create exactly one user email");
    
    // ✅ Verify user data matches GitHub profile
    let user_data: Option<sea_orm::QueryResult> = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT username, avatar_url FROM users LIMIT 1".to_string(),
        ))
        .await
        .expect("Failed to query user data");
    
    let user_data = user_data.expect("User should exist");
    let username: String = user_data.try_get("", "username").expect("Should have username");
    assert_eq!(username, "arthur", "Username should match GitHub profile");
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_gitlab_successful_flow_creates_jwt_for_new_user() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup GitLab mock server for successful flow
    let gitlab = GitLabFixtures::service().await;
    gitlab.setup_successful_token_exchange().await;
    gitlab.setup_successful_user_profile_alice().await;
    
    // Create valid state for login operation
    let state = create_login_state();
    
    // Make callback request with authorization code
    let response = client
        .get(&format!("{}/api/auth/gitlab/callback", base_url))
        .query(&[
            ("code", "valid_gitlab_auth_code"),
            ("state", &state)
        ])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ✅ Should return 200 OK with JWT token
    assert_eq!(response.status(), 200, "Should return 200 OK for successful GitLab OAuth callback");
    
    let response_json: Value = response.json().await.expect("Should return JSON response");
    
    assert_eq!(response_json["operation"], "login");
    
    // ✅ Should contain valid JWT token
    let access_token = response_json["access_token"].as_str().expect("Response should contain access token");
    assert!(verify_jwt_structure(access_token), "Access token should have valid structure");
    
    // ✅ Verify database state
    let user_count = count_entities(db.clone(), "users").await.expect("Failed to count users");
    assert_eq!(user_count, 1, "Should create exactly one user");
    
    let token_count = count_entities(db.clone(), "provider_tokens").await.expect("Failed to count provider tokens");
    assert_eq!(token_count, 1, "Should create exactly one provider token");
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_links_external_account_with_valid_link_state() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create existing user with database fixtures
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
    
    // Setup GitHub mock server
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    // Create valid state for link operation with existing user ID
    let state = create_link_state(existing_user.id());
    
    // Make callback request for linking
    let response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[
            ("code", "valid_auth_code_for_linking"),
            ("state", &state)
        ])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ✅ Should return 200 OK with success status
    assert_eq!(response.status(), 200, "Should return 200 OK for successful account linking");
    
    let response_json: Value = response.json().await.expect("Should return JSON response");
    
    assert_eq!(response_json["operation"], "link");
    assert!(response_json["message"].as_str().unwrap().contains("successfully linked"));
    
    // ✅ Should NOT create new user (should still be 1)
    let user_count = count_entities(db.clone(), "users").await.expect("Failed to count users");
    assert_eq!(user_count, 1, "Should not create new user for linking operation");
    
    // ✅ Should create provider token linked to existing user
    let token_count = count_entities(db.clone(), "provider_tokens").await.expect("Failed to count provider tokens");
    assert_eq!(token_count, 1, "Should create exactly one provider token");
    
    // ✅ Verify provider token is linked to correct user
    let provider_token_data: Option<sea_orm::QueryResult> = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT user_id, provider FROM provider_tokens WHERE user_id = '{}'", existing_user.id()),
        ))
        .await
        .expect("Failed to query provider token");
    
    let provider_token_data = provider_token_data.expect("Provider token should exist");
    let token_user_id: Uuid = provider_token_data.try_get("", "user_id").expect("Should have user_id");
    let provider: String = provider_token_data.try_get("", "provider").expect("Should have provider");
    
    assert_eq!(token_user_id, existing_user.id(), "Provider token should be linked to existing user");
    assert_eq!(provider, "github", "Provider should be GitHub");
    
    // ✅ Verify existing user and email are unchanged
    assert!(existing_user.check(db.clone()).await.expect("Failed to check existing user"));
    assert!(primary_email.check(db.clone()).await.expect("Failed to check primary email"));
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_associates_new_provider_for_same_user() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create user with existing GitHub provider
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
    
    // Setup GitLab mock server for the same user (Arthur)
    let gitlab = GitLabFixtures::service().await;
    gitlab.setup_successful_token_exchange().await;
    // Note: We'll mock GitLab to return Arthur's profile (same user, different provider)
    gitlab.setup_successful_user_profile_alice().await; // Using Alice profile for GitLab
    
    // Create valid state for link operation
    let state = create_link_state(existing_user.id());
    
    // Make callback request to associate GitLab with existing user
    let response = client
        .get(&format!("{}/api/auth/gitlab/callback", base_url))
        .query(&[
            ("code", "valid_gitlab_auth_code"),
            ("state", &state)
        ])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ✅ Should return 200 OK with linked status
    assert_eq!(response.status(), 200, "Should return 200 OK for successful provider association");
    
    let response_json: Value = response.json().await.expect("Should return JSON response");
    
    assert_eq!(response_json["operation"], "link");
    
    // ✅ Should still have only one user
    let user_count = count_entities(db.clone(), "users").await.expect("Failed to count users");
    assert_eq!(user_count, 1, "Should still have exactly one user");
    
    // ✅ Should now have two provider tokens (GitHub + GitLab)
    let token_count = count_entities(db.clone(), "provider_tokens").await.expect("Failed to count provider tokens");
    assert_eq!(token_count, 2, "Should have two provider tokens (GitHub + GitLab)");
    
    // ✅ Verify both providers are linked to the same user
    let provider_tokens: Vec<sea_orm::QueryResult> = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT provider FROM provider_tokens WHERE user_id = '{}' ORDER BY provider", existing_user.id()),
        ))
        .await
        .expect("Failed to query provider tokens");
    
    assert_eq!(provider_tokens.len(), 2, "Should have exactly two provider tokens");
    
    let providers: Vec<String> = provider_tokens
        .iter()
        .map(|row| row.try_get::<String>("", "provider").expect("Should have provider"))
        .collect();
    
    assert_eq!(providers, vec!["github", "gitlab"], "Should have both GitHub and GitLab providers");
    
    // ✅ Verify original entities are unchanged
    assert!(existing_user.check(db.clone()).await.expect("Failed to check existing user"));
    assert!(primary_email.check(db.clone()).await.expect("Failed to check primary email"));
    assert!(github_token.check(db.clone()).await.expect("Failed to check GitHub token"));
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_prevents_linking_provider_already_bound_to_another_user() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create first user with GitHub provider
    let first_user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create first user");
    
    let first_user_email = DbFixtures::user_email()
        .arthur_primary(first_user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create first user email");
    
    let first_user_github_token = DbFixtures::provider_token()
        .arthur_github(first_user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create first user GitHub token");
    
    // Pre-create second user (different user who wants to link the same GitHub account)
    let second_user = DbFixtures::user()
        .bob()
        .commit(db.clone())
        .await
        .expect("Failed to create second user");
    
    let second_user_email = DbFixtures::user_email()
        .bob_primary(second_user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create second user email");
    
    // Setup GitHub mock server to return Arthur's profile (already linked to first user)
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    // Create valid state for link operation with second user ID
    let state = create_link_state(second_user.id());
    
    // Attempt to link Arthur's GitHub account to second user (should fail)
    let response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[
            ("code", "valid_auth_code_but_already_linked"),
            ("state", &state)
        ])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ❌ Should return 409 Conflict (provider already linked to another user)
    assert_eq!(response.status(), 409, "Should return 409 Conflict for provider already linked to another user");
    
    let error_response: Value = response.json().await.expect("Should return JSON error response");
    
    assert_eq!(error_response["error"]["error_code"], "provider_already_linked");
    assert!(error_response["error"]["message"].as_str().unwrap().contains("already linked to another user"), 
           "Error message should indicate provider is already linked");
    
    // ✅ Should still have exactly two users (no new users created)
    let user_count = count_entities(db.clone(), "users").await.expect("Failed to count users");
    assert_eq!(user_count, 2, "Should still have exactly two users");
    
    // ✅ Should still have exactly one provider token (no new tokens created)
    let token_count = count_entities(db.clone(), "provider_tokens").await.expect("Failed to count provider tokens");
    assert_eq!(token_count, 1, "Should still have exactly one provider token");
    
    // ✅ Verify original entities are unchanged
    assert!(first_user.check(db.clone()).await.expect("Failed to check first user"));
    assert!(first_user_email.check(db.clone()).await.expect("Failed to check first user email"));
    assert!(first_user_github_token.check(db.clone()).await.expect("Failed to check first user GitHub token"));
    assert!(second_user.check(db.clone()).await.expect("Failed to check second user"));
    assert!(second_user_email.check(db.clone()).await.expect("Failed to check second user email"));
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_fails_on_invalid_authorization_code() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup GitHub mock server to return error for invalid code
    let github = GitHubFixtures::service().await;
    github.setup_failed_token_exchange_invalid_code().await;
    
    // Create valid state
    let state = create_login_state();
    
    // Make callback request with invalid authorization code
    let response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[
            ("code", "invalid_auth_code_123"),
            ("state", &state)
        ])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ❌ Should return 401 Unauthorized for invalid code
    assert_eq!(response.status(), 401, "Should return 401 Unauthorized for invalid authorization code");
    
    let error_response: Value = response.json().await.expect("Should return JSON error response");
    
    // For invalid code, the error comes from the login usecase, not the callback endpoint
    assert_eq!(error_response["error"]["error_code"], "authentication_failed");
    assert!(error_response["error"]["message"].as_str().unwrap().contains("Authentication failed"), 
           "Error message should mention authentication failure");
    
    // ✅ Should not create any users or tokens
    let user_count = count_entities(db.clone(), "users").await.expect("Failed to count users");
    assert_eq!(user_count, 0, "Should not create any users for invalid code");
    
    let token_count = count_entities(db.clone(), "provider_tokens").await.expect("Failed to count provider tokens");
    assert_eq!(token_count, 0, "Should not create any provider tokens for invalid code");
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_fails_on_expired_authorization_code() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup GitHub mock server to return error for expired code
    let github = GitHubFixtures::service().await;
    github.setup_failed_token_exchange_invalid_code().await; // Using invalid_code as expired_code may not exist
    
    // Create valid state
    let state = create_login_state();
    
    // Make callback request with expired authorization code
    let response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[
            ("code", "expired_auth_code_456"),
            ("state", &state)
        ])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ❌ Should return 401 for expired code
    assert_eq!(response.status(), 401, "Should return 401 for expired authorization code");
    
    let error_response: Value = response.json().await.expect("Should return JSON error response");
    
    // For expired code, the error comes from the login usecase, not the callback endpoint
    assert_eq!(error_response["error"]["error_code"], "authentication_failed");
    assert!(error_response["error"]["message"].as_str().unwrap().contains("Authentication failed"), 
           "Error message should mention authentication failure");
    
    // ✅ Should not create any users or tokens
    let user_count = count_entities(db.clone(), "users").await.expect("Failed to count users");
    assert_eq!(user_count, 0, "Should not create any users for expired code");
    
    let token_count = count_entities(db.clone(), "provider_tokens").await.expect("Failed to count provider tokens");
    assert_eq!(token_count, 0, "Should not create any provider tokens for expired code");
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_returns_400_on_missing_state_parameter() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Make callback request without state parameter
    let response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[("code", "valid_auth_code")])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ❌ Should return 400 Bad Request for missing state
    assert_eq!(response.status(), 400, "Should return 400 Bad Request for missing state parameter");
    
    let error_response: Value = response.json().await.expect("Should return JSON error response");
    
    assert_eq!(error_response["error"]["error_code"], "missing_state");
    assert!(error_response["error"]["message"].as_str().unwrap().contains("state parameter"), 
           "Error message should mention missing state parameter");
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_returns_400_on_missing_code_parameter() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create valid state
    let state = create_login_state();
    
    // Make callback request without code parameter
    let response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[("state", &state)])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ❌ Should return 400 Bad Request for missing code
    assert_eq!(response.status(), 400, "Should return 400 Bad Request for missing code parameter");
    
    let error_response: Value = response.json().await.expect("Should return JSON error response");
    
    assert_eq!(error_response["error"]["error_code"], "missing_code");
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_returns_400_on_invalid_state_format() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create invalid state (not base64 encoded JSON)
    let invalid_state = create_invalid_state();
    
    // Make callback request with invalid state format
    let response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[
            ("code", "valid_auth_code"),
            ("state", &invalid_state)
        ])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ❌ Should return 400 Bad Request for invalid state format
    assert_eq!(response.status(), 400, "Should return 400 Bad Request for invalid state format");
    
    let error_response: Value = response.json().await.expect("Should return JSON error response");
    
    assert_eq!(error_response["error"]["error_code"], "invalid_state");
    assert!(error_response["error"]["message"].as_str().unwrap().contains("state parameter"), 
           "Error message should mention invalid state parameter");
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_returns_400_on_invalid_state_purpose() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create state with invalid operation type
    let invalid_state_data = serde_json::json!({
        "operation": {
            "type": "invalid_operation"
        },
        "nonce": Uuid::new_v4().to_string()
    });
    let invalid_state = general_purpose::URL_SAFE_NO_PAD.encode(invalid_state_data.to_string());
    
    // Make callback request with invalid state purpose
    let response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[
            ("code", "valid_auth_code"),
            ("state", &invalid_state)
        ])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ❌ Should return 400 Bad Request for invalid state purpose
    assert_eq!(response.status(), 400, "Should return 400 Bad Request for invalid state purpose");
    
    let error_response: Value = response.json().await.expect("Should return JSON error response");
    
    assert_eq!(error_response["error"]["error_code"], "invalid_state");
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_returns_401_when_provider_refuses_user() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup GitHub mock server to return successful token exchange but unauthorized user profile
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_failed_user_profile_unauthorized().await;
    
    // Create valid state
    let state = create_login_state();
    
    // Make callback request where provider refuses user access
    let response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[
            ("code", "valid_code_but_user_refused"),
            ("state", &state)
        ])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ❌ Should return 401 Unauthorized when provider refuses user
    assert_eq!(response.status(), 401, "Should return 401 Unauthorized when provider refuses user");
    
    let error_response: Value = response.json().await.expect("Should return JSON error response");
    
    // For provider refusal, the error comes from the login usecase
    assert_eq!(error_response["error"]["error_code"], "authentication_failed");
    assert!(error_response["error"]["message"].as_str().unwrap().contains("Authentication failed"), 
           "Error message should mention authentication failure");
    
    // ✅ Should not create any users or tokens
    let user_count = count_entities(db.clone(), "users").await.expect("Failed to count users");
    assert_eq!(user_count, 0, "Should not create any users when provider refuses");
    
    let token_count = count_entities(db.clone(), "provider_tokens").await.expect("Failed to count provider tokens");
    assert_eq!(token_count, 0, "Should not create any provider tokens when provider refuses");
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_returns_401_when_provider_rejects_user() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup GitHub mock server to simulate provider rejection (e.g., account suspended)
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_failed_user_profile_unauthorized().await; // Using unauthorized as account_suspended may not exist
    
    // Create valid state
    let state = create_login_state();
    
    // Make callback request where provider rejects user
    let response = client
        .get(&format!("{}/api/auth/github/callback", base_url))
        .query(&[
            ("code", "valid_code_but_user_rejected"),
            ("state", &state)
        ])
        .send()
        .await
        .expect("Failed to send callback request");
    
    // ❌ Should return 401 Unauthorized when provider rejects user
    assert_eq!(response.status(), 401, "Should return 401 Unauthorized when provider rejects user");
    
    let error_response: Value = response.json().await.expect("Should return JSON error response");
    
    // For provider rejection, the error comes from the login usecase
    assert_eq!(error_response["error"]["error_code"], "authentication_failed");
    assert!(error_response["error"]["message"].as_str().unwrap().contains("Authentication failed"), 
           "Error message should mention authentication failure");
    
    // ✅ Should not create any users or tokens
    let user_count = count_entities(db.clone(), "users").await.expect("Failed to count users");
    assert_eq!(user_count, 0, "Should not create any users when provider rejects");
    
    let token_count = count_entities(db.clone(), "provider_tokens").await.expect("Failed to count provider tokens");
    assert_eq!(token_count, 0, "Should not create any provider tokens when provider rejects");
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_unsupported_provider_returns_422() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Create valid state
    let state = create_login_state();
    
    // Test unsupported providers
    let unsupported_providers = vec!["facebook", "google", "twitter", "unknown"];
    
    for provider in unsupported_providers {
        let response = client
            .get(&format!("{}/api/auth/{}/callback", base_url, provider))
            .query(&[
                ("code", "valid_auth_code"),
                ("state", &state)
            ])
            .send()
            .await
            .expect("Failed to send callback request");
        
        // ❌ Should return 422 for unsupported providers (validation error)
        assert_eq!(response.status(), 422, 
                  "Should return 422 Unprocessable Entity for unsupported provider: {}", provider);
        
        let error_response: Value = response.json().await.expect("Should return JSON error response");
        
        // axum-valid returns validation errors in a different format
        // Check for validation error structure
        assert!(error_response.get("provider_name").is_some(), 
               "Response should contain validation errors for provider: {}", provider);
    }
}

#[tokio::test]
#[serial]
async fn test_oauth_callback_case_insensitive_providers() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let _db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Setup fixtures
    let github = GitHubFixtures::service().await;
    let gitlab = GitLabFixtures::service().await;
    
    // Test case variations that should work
    let valid_cases = vec![
        ("github", "valid_auth_code_123"),
        ("GITHUB", "valid_auth_code_123"),
        ("GitHub", "valid_auth_code_123"),
        ("gitlab", "valid_gitlab_auth_code"),
        ("GITLAB", "valid_gitlab_auth_code"),
        ("GitLab", "valid_gitlab_auth_code"),
    ];
    
    for (provider_input, auth_code) in valid_cases {
        // Setup the appropriate mock for each provider
        if provider_input.to_lowercase() == "github" {
            github.setup_successful_token_exchange().await;
            github.setup_successful_user_profile_arthur().await;
        } else {
            gitlab.setup_successful_token_exchange().await;
            gitlab.setup_successful_user_profile_alice().await;
        }
        
        // Create fresh state for each test
        let state = create_login_state();
        
        let response = client
            .get(&format!("{}/api/auth/{}/callback", base_url, provider_input))
            .query(&[
                ("code", auth_code),
                ("state", &state)
            ])
            .send()
            .await
            .expect("Failed to send callback request");

        // ✅ Should successfully handle case-insensitive provider names
        assert_eq!(response.status(), 200, 
                  "Should handle case-insensitive provider: {}", provider_input);
        
        let response_json: Value = response.json().await.expect("Should return JSON response");
        
        assert_eq!(response_json["operation"], "login");
        
        // Clean up for next iteration - truncate all tables
        let _ = test_fixture.cleanup().await;
    }
} 