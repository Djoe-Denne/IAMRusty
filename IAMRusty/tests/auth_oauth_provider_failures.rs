mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;
mod utils;

use common::setup_test_server;
use fixtures::{GitHubFixtures, GitLabFixtures};
use reqwest::{Response, StatusCode};
use serial_test::serial;
use utils::{auth::AuthTestUtils, oauth::OAuthTestUtils};

async fn assert_failed_without_persistence(
    response: Response,
    db: std::sync::Arc<sea_orm::DatabaseConnection>,
) {
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"]["error_code"], "authentication_failed");
    assert_eq!(
        AuthTestUtils::count_entities(db.clone(), "users")
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        AuthTestUtils::count_entities(db, "provider_tokens")
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
#[serial]
async fn github_rate_limit_and_server_error_are_http_failures_without_account_side_effects() {
    let (fixture, base_url, client) = setup_test_server().await.unwrap();
    let db = fixture.db();
    let github = GitHubFixtures::service().await;

    github.setup_successful_token_exchange().await;
    github.setup_rate_limit_exceeded().await;
    let state = OAuthTestUtils::create_login_state();
    let rate_limited = client
        .get(format!("{base_url}/api/auth/github/callback"))
        .query(&[("code", "test_auth_code"), ("state", &state)])
        .send()
        .await
        .unwrap();
    assert_failed_without_persistence(rate_limited, db.clone()).await;

    github.reset().await;
    github.setup_successful_token_exchange().await;
    github.setup_server_error().await;
    let state = OAuthTestUtils::create_login_state();
    let unavailable = client
        .get(format!("{base_url}/api/auth/github/callback"))
        .query(&[("code", "test_auth_code"), ("state", &state)])
        .send()
        .await
        .unwrap();
    assert_failed_without_persistence(unavailable, db).await;
}

#[tokio::test]
#[serial]
async fn gitlab_forbidden_rate_limit_and_server_error_keep_the_same_public_contract() {
    let (fixture, base_url, client) = setup_test_server().await.unwrap();
    let db = fixture.db();
    let gitlab = GitLabFixtures::service().await;

    gitlab.setup_successful_token_exchange().await;
    gitlab.setup_forbidden_error().await;
    let state = OAuthTestUtils::create_login_state();
    let forbidden = client
        .get(format!("{base_url}/api/auth/gitlab/callback"))
        .query(&[("code", "test_auth_code"), ("state", &state)])
        .send()
        .await
        .unwrap();
    assert_failed_without_persistence(forbidden, db.clone()).await;

    gitlab.reset().await;
    gitlab.setup_successful_token_exchange().await;
    gitlab.setup_rate_limit_exceeded().await;
    let state = OAuthTestUtils::create_login_state();
    let rate_limited = client
        .get(format!("{base_url}/api/auth/gitlab/callback"))
        .query(&[("code", "test_auth_code"), ("state", &state)])
        .send()
        .await
        .unwrap();
    assert_failed_without_persistence(rate_limited, db.clone()).await;

    gitlab.reset().await;
    gitlab.setup_successful_token_exchange().await;
    gitlab.setup_server_error().await;
    let state = OAuthTestUtils::create_login_state();
    let unavailable = client
        .get(format!("{base_url}/api/auth/gitlab/callback"))
        .query(&[("code", "test_auth_code"), ("state", &state)])
        .send()
        .await
        .unwrap();
    assert_failed_without_persistence(unavailable, db).await;
}

#[tokio::test]
#[serial]
async fn github_rejects_invalid_authorization_codes_and_clients_without_writing_tokens() {
    let (fixture, base_url, client) = setup_test_server().await.unwrap();
    let db = fixture.db();
    let github = GitHubFixtures::service().await;

    github.setup_failed_token_exchange_invalid_code().await;
    let state = OAuthTestUtils::create_login_state();
    let invalid_code = client
        .get(format!("{base_url}/api/auth/github/callback"))
        .query(&[("code", "expired-or-replayed-code"), ("state", &state)])
        .send()
        .await
        .unwrap();
    assert_failed_without_persistence(invalid_code, db.clone()).await;

    github.reset().await;
    github.setup_failed_token_exchange_invalid_client().await;
    let state = OAuthTestUtils::create_login_state();
    let invalid_client = client
        .get(format!("{base_url}/api/auth/github/callback"))
        .query(&[("code", "test_auth_code"), ("state", &state)])
        .send()
        .await
        .unwrap();
    assert_failed_without_persistence(invalid_client, db).await;
}

#[tokio::test]
#[serial]
async fn gitlab_rejects_invalid_authorization_codes_and_clients_without_writing_tokens() {
    let (fixture, base_url, client) = setup_test_server().await.unwrap();
    let db = fixture.db();
    let gitlab = GitLabFixtures::service().await;

    gitlab.setup_failed_token_exchange_invalid_code().await;
    let state = OAuthTestUtils::create_login_state();
    let invalid_code = client
        .get(format!("{base_url}/api/auth/gitlab/callback"))
        .query(&[("code", "expired-or-replayed-code"), ("state", &state)])
        .send()
        .await
        .unwrap();
    assert_failed_without_persistence(invalid_code, db.clone()).await;

    gitlab.reset().await;
    gitlab.setup_failed_token_exchange_invalid_client().await;
    let state = OAuthTestUtils::create_login_state();
    let invalid_client = client
        .get(format!("{base_url}/api/auth/gitlab/callback"))
        .query(&[("code", "test_auth_code"), ("state", &state)])
        .send()
        .await
        .unwrap();
    assert_failed_without_persistence(invalid_client, db).await;
}
