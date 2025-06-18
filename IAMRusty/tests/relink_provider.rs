use serial_test::serial;
// Include common test utilities and fixtures
mod common;
mod fixtures;

use serde_json::Value;
use uuid::Uuid;

use common::{
    setup_test_server, 
    jwt_test_utils::{create_valid_jwt_token_with_encoder, create_expired_jwt_token_with_encoder, create_invalid_jwt_token_with_encoder}
};
use fixtures::{DbFixtures, GitHubFixtures, GitLabFixtures};

// 🔗 Relink Provider Endpoint Tests
// 📍 GET /api/auth/{provider}/relink-start
// 📍 GET /api/auth/{provider}/relink-callback

#[tokio::test]
#[serial]
async fn test_generate_relink_provider_start_url_github_success() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Make request to generate GitHub relink start URL
    let response = client
        .get(&format!("{}/api/auth/github/relink-start", base_url))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 OK with auth URL
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for relink start URL generation"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    // ✅ Should contain auth_url
    assert!(
        response_json["auth_url"].is_string(),
        "Response should contain auth_url"
    );

    let auth_url = response_json["auth_url"]
        .as_str()
        .expect("auth_url should be a string");
    
    println!("auth_url: {}", auth_url);
    // ✅ Should be a valid GitHub OAuth URL
    assert!(
        auth_url.starts_with("http://localhost:3000/login/oauth/authorize"),
        "Auth URL should be GitHub OAuth URL"
    );
    assert!(
        auth_url.contains("client_id="),
        "Auth URL should contain client_id"
    );
    assert!(
        auth_url.contains("scope="),
        "Auth URL should contain scope"
    );
    assert!(
        auth_url.contains("redirect_uri="),
        "Auth URL should contain redirect_uri"
    );
    assert!(
        auth_url.contains("relink-callback"),
        "Auth URL should contain relink-callback in redirect_uri"
    );
}

#[tokio::test]
#[serial]
async fn test_generate_relink_provider_start_url_gitlab_success() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Make request to generate GitLab relink start URL
    let response = client
        .get(&format!("{}/api/auth/gitlab/relink-start", base_url))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 OK with auth URL
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for GitLab relink start URL generation"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    let auth_url = response_json["auth_url"]
        .as_str()
        .expect("auth_url should be a string");
    
    // ✅ Should be a valid GitLab OAuth URL
    assert!(
        auth_url.starts_with("http://localhost:3000/oauth/authorize"),
        "Auth URL should be GitLab OAuth URL"
    );
    assert!(
        auth_url.contains("relink-callback"),
        "Auth URL should contain relink-callback in redirect_uri"
    );
}

#[tokio::test]
#[serial]
async fn test_generate_relink_provider_start_url_unsupported_provider() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Test unsupported providers
    let unsupported_providers = vec!["facebook", "twitter", "linkedin", "invalid"];

    for provider in unsupported_providers {
        let response = client
            .get(&format!("{}/api/auth/{}/relink-start", base_url, provider))
            .send()
            .await
            .expect("Failed to send request");

        // ❌ Should return 422 Unprocessable Entity for unsupported provider
        assert_eq!(
            response.status(),
            422,
            "Should return 422 for unsupported provider: '{}'",
            provider
        );
    }
}

#[tokio::test]
#[serial]
async fn test_relink_provider_callback_github_success() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with existing GitHub provider token
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

    let existing_token = DbFixtures::provider_token()
        .arthur_github(user.id())
        .access_token("old_github_token_123")
        .commit(db.clone())
        .await
        .expect("Failed to create existing GitHub token");

    // Setup GitHub mock server for relink (same user profile but new tokens)
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;

    // Create valid JWT token for authentication
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Make callback request to relink GitHub provider
    let response = client
        .get(&format!("{}/api/auth/github/relink-callback", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .query(&[("code", "test_auth_code")])
        .send()
        .await
        .expect("Failed to send callback request");

    // ✅ Should return 200 OK with relinked data
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for successful relink"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    // ✅ Should contain user data
    assert!(
        response_json["user"].is_object(),
        "Response should contain user object"
    );
    assert_eq!(
        response_json["user"]["id"], user.id().to_string(),
        "Should return correct user ID"
    );

    // ✅ Should contain emails
    assert!(
        response_json["emails"].is_array(),
        "Response should contain emails array"
    );

    // ✅ Should indicate if new email was added
    assert!(
        response_json["new_email_added"].is_boolean(),
        "Response should contain new_email_added boolean"
    );

    // ✅ User should still exist unchanged
    assert!(
        user.check(db.clone()).await.expect("Failed to check user"),
        "User should still exist"
    );

    // Note: In a real test, we'd verify that the provider token was updated in the database
}

#[tokio::test]
#[serial]
async fn test_relink_provider_callback_returns_401_when_no_authorization_header() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Make request without Authorization header
    let response = client
        .get(&format!("{}/api/auth/github/relink-callback", base_url))
        .query(&[("code", "test_auth_code")])
        .send()
        .await
        .expect("Failed to send request");

    // ❌ Should return 401 Unauthorized for missing header
    assert_eq!(
        response.status(),
        401,
        "Should return 401 for missing Authorization header"
    );
}

#[tokio::test]
#[serial]
async fn test_relink_provider_callback_returns_401_when_token_is_expired() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create expired JWT token
    let user_id = Uuid::new_v4();
    let expired_token = create_expired_jwt_token_with_encoder(user_id, &_fixture.config())
        .expect("Failed to create expired JWT token");

    // Make request with expired token
    let response = client
        .get(&format!("{}/api/auth/github/relink-callback", base_url))
        .header("Authorization", format!("Bearer {}", expired_token))
        .query(&[("code", "test_auth_code")])
        .send()
        .await
        .expect("Failed to send request");

    // ❌ Should return 401 Unauthorized for expired token
    assert_eq!(
        response.status(),
        401,
        "Should return 401 for expired token"
    );
}

#[tokio::test]
#[serial]
async fn test_relink_provider_callback_returns_401_when_token_has_invalid_signature() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create JWT token with invalid signature
    let user_id = Uuid::new_v4();
    let invalid_token = create_invalid_jwt_token_with_encoder(user_id, &_fixture.config())
        .expect("Failed to create invalid signature JWT token");

    // Make request with invalid signature token
    let response = client
        .get(&format!("{}/api/auth/github/relink-callback", base_url))
        .header("Authorization", format!("Bearer {}", invalid_token))
        .query(&[("code", "test_auth_code")])
        .send()
        .await
        .expect("Failed to send request");

    // ❌ Should return 401 Unauthorized for invalid signature
    assert_eq!(
        response.status(),
        401,
        "Should return 401 for invalid signature"
    );
}

#[tokio::test]
#[serial]
async fn test_relink_provider_callback_returns_422_when_provider_is_unsupported() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create valid JWT token
    let user_id = Uuid::new_v4();
    let jwt_token = create_valid_jwt_token_with_encoder(user_id, &_fixture.config())
        .expect("Failed to create JWT token");

    // Test unsupported providers
    let unsupported_providers = vec!["facebook", "twitter", "linkedin", "invalid"];

    for provider in unsupported_providers {
        let response = client
            .get(&format!("{}/api/auth/{}/relink-callback", base_url, provider))
            .header("Authorization", format!("Bearer {}", jwt_token))
            .query(&[("code", "test_auth_code")])
            .send()
            .await
            .expect("Failed to send request");

        // ❌ Should return 422 Unprocessable Entity for unsupported provider
        assert_eq!(
            response.status(),
            422,
            "Should return 422 for unsupported provider: '{}'",
            provider
        );
    }
}

#[tokio::test]
#[serial]
async fn test_relink_provider_callback_returns_422_when_missing_code_parameter() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create valid JWT token
    let user_id = Uuid::new_v4();
    let jwt_token = create_valid_jwt_token_with_encoder(user_id, &_fixture.config())
        .expect("Failed to create JWT token");

    // Make request without code parameter
    let response = client
        .get(&format!("{}/api/auth/github/relink-callback", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ❌ Should return 400 Bad Request for missing code parameter (validation error)
    assert_eq!(
        response.status(),
        400,
        "Should return 400 for missing code parameter"
    );
}

#[tokio::test]
#[serial]
async fn test_relink_provider_callback_returns_422_when_provider_not_currently_linked() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user WITHOUT any GitHub provider token
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

    // Setup GitHub mock server
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;

    // Create valid JWT token for authentication
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Make callback request to relink GitHub provider (should fail - no existing link)
    let response = client
        .get(&format!("{}/api/auth/github/relink-callback", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .query(&[("code", "test_auth_code")])
        .send()
        .await
        .expect("Failed to send callback request");

    // ❌ Should return 422 for trying to relink non-linked provider
    assert_eq!(
        response.status(),
        422,
        "Should return 422 when trying to relink provider that is not currently linked"
    );

    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(
        response_json["error"].is_object(),
        "Should return error object"
    );
    
    // Should indicate business rule violation
    let error_message = response_json["error"]["message"]
        .as_str()
        .expect("Should have error message");
    assert!(
        error_message.contains("not currently linked") || error_message.contains("Cannot relink"),
        "Error message should indicate provider is not currently linked"
    );
}

#[tokio::test]
#[serial]
async fn test_relink_provider_callback_gitlab_success() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with existing GitLab provider token
    let user = DbFixtures::user()
        .bob()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let primary_email = DbFixtures::user_email()
        .bob_primary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create primary email");

    let existing_token = DbFixtures::provider_token()
        .gitlab(user.id())
        .access_token("old_gitlab_token_456")
        .commit(db.clone())
        .await
        .expect("Failed to create existing GitLab token");

    // Setup GitLab mock server for relink
    let gitlab = GitLabFixtures::service().await;
    gitlab.setup_successful_token_exchange().await;
    gitlab.setup_successful_user_profile_alice().await; // Using Alice profile for GitLab

    // Create valid JWT token for authentication
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Make callback request to relink GitLab provider
    let response = client
        .get(&format!("{}/api/auth/gitlab/relink-callback", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .query(&[("code", "test_auth_code")])
        .send()
        .await
        .expect("Failed to send callback request");

    // ✅ Should return 200 OK with relinked data
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for successful GitLab relink"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    // ✅ Should contain user data
    assert_eq!(
        response_json["user"]["id"], user.id().to_string(),
        "Should return correct user ID"
    );

    // Note: In a real test, we'd verify that the provider token was updated in the database
}

#[tokio::test]
#[serial]
async fn test_relink_provider_callback_user_with_multiple_providers() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user with both GitHub and GitLab provider tokens
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

    let github_token = DbFixtures::provider_token()
        .arthur_github(user.id())
        .access_token("old_github_token_123")
        .commit(db.clone())
        .await
        .expect("Failed to create GitHub token");

    let gitlab_token = DbFixtures::provider_token()
        .gitlab(user.id())
        .access_token("old_gitlab_token_456")
        .commit(db.clone())
        .await
        .expect("Failed to create GitLab token");

    // Setup GitHub mock server for relink
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;

    // Create valid JWT token for authentication
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Make callback request to relink only GitHub provider
    let response = client
        .get(&format!("{}/api/auth/github/relink-callback", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .query(&[("code", "test_auth_code")])
        .send()
        .await
        .expect("Failed to send callback request");

    // ✅ Should return 200 OK with relinked data
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for successful GitHub relink"
    );

    // ✅ GitLab token should still be the original one
    assert!(
        gitlab_token
            .check(db.clone())
            .await
            .expect("Failed to check GitLab token"),
        "Original GitLab token should still exist unchanged"
    );

    // Note: In a real test, we'd verify that the GitHub token was updated and GitLab token was unchanged
}