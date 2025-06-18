use serial_test::serial;
// Include common test utilities and fixtures
mod common;
mod fixtures;

use serde_json::Value;
use uuid::Uuid;

use common::{setup_test_server, jwt_test_utils::{create_valid_jwt_token_with_encoder, create_expired_jwt_token_with_encoder, create_invalid_jwt_token_with_encoder}};
use fixtures::DbFixtures;

// 🔐 Internal Provider Token Revoke Endpoint Tests
// 🗑️ DELETE /internal/{provider}/revoke

#[tokio::test]
#[serial]
async fn test_revoke_provider_token_github_success() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user in database
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    // Create GitHub provider token for the user
    let provider_token = DbFixtures::provider_token()
        .arthur_github(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create provider token");

    // Create valid JWT token for authentication
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Verify token exists before revoke
    assert!(
        provider_token
            .check(db.clone())
            .await
            .expect("Failed to check provider token"),
        "Provider token should exist before revoke"
    );

    // Make request to revoke provider token endpoint
    let response = client
        .delete(&format!("{}/internal/github/revoke", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 OK with success message
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for successful revoke"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    // ✅ Should contain success message
    assert!(
        response_json["message"].is_string(),
        "Response should contain success message"
    );
    
    let message = response_json["message"]
        .as_str()
        .expect("message should be a string");
    assert!(
        message.contains("github") && message.contains("revoked successfully"),
        "Message should indicate GitHub token was revoked"
    );

    // ✅ Should no longer exist in database
    assert!(
        !provider_token
            .check(db.clone())
            .await
            .expect("Failed to check provider token"),
        "Provider token should no longer exist after revoke"
    );

    // ✅ User should still exist
    assert!(
        user.check(db.clone())
            .await
            .expect("Failed to check user"),
        "User should still exist after token revoke"
    );
}

#[tokio::test]
#[serial]
async fn test_revoke_provider_token_gitlab_success() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user in database
    let user = DbFixtures::user()
        .bob()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    // Create GitLab provider token for the user
    let provider_token = DbFixtures::provider_token()
        .gitlab(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create provider token");

    // Create valid JWT token for authentication
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Make request to revoke GitLab provider token
    let response = client
        .delete(&format!("{}/internal/gitlab/revoke", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 OK with success message
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for successful GitLab revoke"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");
    let message = response_json["message"]
        .as_str()
        .expect("message should be a string");
    assert!(
        message.contains("gitlab") && message.contains("revoked successfully"),
        "Message should indicate GitLab token was revoked"
    );

    // ✅ Should no longer exist in database
    assert!(
        !provider_token
            .check(db.clone())
            .await
            .expect("Failed to check provider token"),
        "GitLab provider token should no longer exist after revoke"
    );
}

#[tokio::test]
#[serial]
async fn test_revoke_provider_token_returns_401_when_no_authorization_header() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Make request without Authorization header
    let response = client
        .delete(&format!("{}/internal/github/revoke", base_url))
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
async fn test_revoke_provider_token_returns_401_when_token_is_expired() {
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
        .delete(&format!("{}/internal/github/revoke", base_url))
        .header("Authorization", format!("Bearer {}", expired_token))
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
async fn test_revoke_provider_token_returns_401_when_token_has_invalid_signature() {
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
        .delete(&format!("{}/internal/github/revoke", base_url))
        .header("Authorization", format!("Bearer {}", invalid_token))
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
async fn test_revoke_provider_token_returns_422_when_provider_is_unsupported() {
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
            .delete(&format!("{}/internal/{}/revoke", base_url, provider))
            .header("Authorization", format!("Bearer {}", jwt_token))
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
async fn test_revoke_provider_token_returns_404_when_no_token_for_provider() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user in database but NO provider token
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    // Create valid JWT token for authentication
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // Make request to revoke GitHub token when user has no GitHub token
    let response = client
        .delete(&format!("{}/internal/github/revoke", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ❌ Should return 404 Not Found when no token available
    assert_eq!(
        response.status(),
        404,
        "Should return 404 when no token available for provider"
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
        response_json["error"]["error_code"], "no_token_for_provider",
        "Should return no_token_for_provider error code"
    );
}

#[tokio::test]
#[serial]
async fn test_revoke_provider_token_returns_401_when_user_not_found() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Create valid JWT token for non-existent user
    let non_existent_user_id = Uuid::new_v4();
    let jwt_token = create_valid_jwt_token_with_encoder(non_existent_user_id, &_fixture.config())
        .expect("Failed to create JWT token");

    // Make request to revoke token for non-existent user
    let response = client
        .delete(&format!("{}/internal/github/revoke", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ❌ Should return 401 Unauthorized when user doesn't exist (istio handles this)
    assert_eq!(
        response.status(),
        401,
        "Should return 401 when user not found (handled by istio)"
    );

    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON error response");

    assert!(
        response_json["error"].is_object(),
        "Should return error object"
    );
    // Note: When user is not found, istio returns 401, not 404 with specific error codes
}

#[tokio::test]
#[serial]
async fn test_revoke_provider_token_idempotent_on_already_revoked() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user in database
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    // Create GitHub provider token for the user
    let provider_token = DbFixtures::provider_token()
        .arthur_github(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to create provider token");

    // Create valid JWT token for authentication
    let jwt_token = create_valid_jwt_token_with_encoder(user.id(), &_fixture.config())
        .expect("Failed to create JWT token");

    // First revoke - should succeed
    let response1 = client
        .delete(&format!("{}/internal/github/revoke", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send first request");

    assert_eq!(response1.status(), 200, "First revoke should succeed");

    // Verify token is removed
    assert!(
        !provider_token
            .check(db.clone())
            .await
            .expect("Failed to check provider token"),
        "Provider token should be removed after first revoke"
    );

    // Second revoke - should return 404 (no token to revoke)
    let response2 = client
        .delete(&format!("{}/internal/github/revoke", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send second request");

    assert_eq!(
        response2.status(), 404,
        "Second revoke should return 404 - no token to revoke"
    );

    let response2_json: Value = response2
        .json()
        .await
        .expect("Should return JSON error response");

    assert_eq!(
        response2_json["error"]["error_code"], "no_token_for_provider",
        "Should return no_token_for_provider error code for second revoke"
    );
}

#[tokio::test]
#[serial]
async fn test_revoke_provider_token_different_users_different_tokens() {
    // Setup test environment
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create two users with GitHub tokens
    let user1 = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user1");

    let user2 = DbFixtures::user()
        .bob()
        .commit(db.clone())
        .await
        .expect("Failed to create user2");

    let token1 = DbFixtures::provider_token()
        .arthur_github(user1.id())
        .commit(db.clone())
        .await
        .expect("Failed to create token1");

    let token2 = DbFixtures::provider_token()
        .bob_github(user2.id())
        .commit(db.clone())
        .await
        .expect("Failed to create token2");

    // Revoke user1's token
    let jwt_token1 = create_valid_jwt_token_with_encoder(user1.id(), &_fixture.config())
        .expect("Failed to create JWT token for user1");
    let response1 = client
        .delete(&format!("{}/internal/github/revoke", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token1))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response1.status(), 200, "User1 revoke should succeed");

    // ✅ User1's token should be removed
    assert!(
        !token1
            .check(db.clone())
            .await
            .expect("Failed to check token1"),
        "User1's token should be removed"
    );

    // ✅ User2's token should still exist
    assert!(
        token2
            .check(db.clone())
            .await
            .expect("Failed to check token2"),
        "User2's token should still exist"
    );

    // User2 can still revoke their own token
    let jwt_token2 = create_valid_jwt_token_with_encoder(user2.id(), &_fixture.config())
        .expect("Failed to create JWT token for user2");
    let response2 = client
        .delete(&format!("{}/internal/github/revoke", base_url))
        .header("Authorization", format!("Bearer {}", jwt_token2))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response2.status(), 200, "User2 revoke should succeed");

    // ✅ User2's token should now be removed
    assert!(
        !token2
            .check(db.clone())
            .await
            .expect("Failed to check token2"),
        "User2's token should now be removed"
    );
}