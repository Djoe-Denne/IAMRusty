#![allow(missing_docs, clippy::future_not_send)]

mod common;

use axum_test::TestServer;
use common::{boot_app, test_config, GitHubFixtures, ALLOWED_REDIRECT, TEST_HMAC_SECRET};
use github_connect_http::SERVICE_PREFIX;
use github_connect_setup::Application;
use idp_connect_contract::{
    sign, unix_timestamp_secs, ProfileRequest, ProviderTokens, ProviderUserProfile, TokenRequest,
    SIGNATURE_HEADER, TIMESTAMP_HEADER, TOKEN_PATH,
};
use rustycog::testing::http::jwt::create_jwt_token;
use serial_test::serial;
use uuid::Uuid;

const TOKEN_ROUTE: &str = "/github-connect/v1/token";
const PROFILE_ROUTE: &str = "/github-connect/v1/profile";

fn token_body() -> String {
    serde_json::to_string(&TokenRequest {
        code: "test_auth_code".to_owned(),
        redirect_uri: ALLOWED_REDIRECT.to_owned(),
    })
    .expect("token body")
}

async fn post_signed(server: &TestServer, path: &str, body: &str) -> axum_test::TestResponse {
    let now = unix_timestamp_secs().expect("clock");
    let sig = sign(TEST_HMAC_SECRET.as_bytes(), "POST", path, now, body).expect("sign");
    server
        .post(path)
        .add_header(TIMESTAMP_HEADER, now.to_string())
        .add_header(SIGNATURE_HEADER, sig)
        .content_type("application/json")
        .bytes(axum::body::Bytes::from(body.to_owned()))
        .await
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn missing_hmac_headers_are_unauthorized() {
    let github = GitHubFixtures::service().await;
    let app = boot_app(&github.base_url()).expect("app");
    let server = TestServer::new(app.prefixed_router()).expect("test server");

    let response = server
        .post(TOKEN_ROUTE)
        .content_type("application/json")
        .bytes(axum::body::Bytes::from(token_body()))
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn valid_hmac_exchanges_token_and_profile() {
    let github = GitHubFixtures::service().await;
    github
        .setup_successful_token_exchange()
        .await
        .setup_successful_user_profile_arthur()
        .await;
    let app = boot_app(&github.base_url()).expect("app");
    let server = TestServer::new(app.prefixed_router()).expect("test server");

    let token_response = post_signed(&server, TOKEN_ROUTE, &token_body()).await;
    token_response.assert_status_ok();
    let tokens: ProviderTokens = token_response.json();
    assert_eq!(tokens.access_token, "gho_test_access_token_12345");

    let profile_body = serde_json::to_string(&ProfileRequest {
        access_token: tokens.access_token,
    })
    .expect("profile body");
    let profile_response = post_signed(&server, PROFILE_ROUTE, &profile_body).await;
    profile_response.assert_status_ok();
    let profile: ProviderUserProfile = profile_response.json();
    assert_eq!(profile.id, "12345");
    assert_eq!(profile.username, "arthur");
    assert_eq!(profile.email.as_deref(), Some("arthur@example.com"));
    assert!(profile.email_verified);
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn redirect_uri_outside_allowlist_is_not_ok() {
    let github = GitHubFixtures::service().await;
    let app = boot_app(&github.base_url()).expect("app");
    let server = TestServer::new(app.prefixed_router()).expect("test server");

    let body = serde_json::to_string(&TokenRequest {
        code: "test_auth_code".to_owned(),
        redirect_uri: "https://evil.example/cb".to_owned(),
    })
    .expect("body");
    let response = post_signed(&server, TOKEN_ROUTE, &body).await;
    assert_ne!(response.status_code(), axum::http::StatusCode::OK);
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn bearer_jwt_without_hmac_is_unauthorized() {
    let github = GitHubFixtures::service().await;
    let app = boot_app(&github.base_url()).expect("app");
    let server = TestServer::new(app.prefixed_router()).expect("test server");
    let jwt = create_jwt_token(Uuid::new_v4());

    let response = server
        .post(TOKEN_ROUTE)
        .add_header("Authorization", format!("Bearer {jwt}"))
        .content_type("application/json")
        .bytes(axum::body::Bytes::from(token_body()))
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn hmac_path_must_include_service_prefix() {
    let github = GitHubFixtures::service().await;
    github
        .setup_successful_token_exchange()
        .await
        .setup_successful_user_profile_arthur()
        .await;
    let app = boot_app(&github.base_url()).expect("app");
    let server = TestServer::new(app.prefixed_router()).expect("test server");
    let body = token_body();
    let now = unix_timestamp_secs().expect("clock");

    let unprefixed =
        sign(TEST_HMAC_SECRET.as_bytes(), "POST", TOKEN_PATH, now, &body).expect("sign");
    let denied = server
        .post(TOKEN_ROUTE)
        .add_header(TIMESTAMP_HEADER, now.to_string())
        .add_header(SIGNATURE_HEADER, unprefixed)
        .content_type("application/json")
        .bytes(axum::body::Bytes::from(body.clone()))
        .await;
    denied.assert_status(axum::http::StatusCode::UNAUTHORIZED);

    let prefixed =
        sign(TEST_HMAC_SECRET.as_bytes(), "POST", TOKEN_ROUTE, now, &body).expect("sign");
    let allowed = server
        .post(TOKEN_ROUTE)
        .add_header(TIMESTAMP_HEADER, now.to_string())
        .add_header(SIGNATURE_HEADER, prefixed)
        .content_type("application/json")
        .bytes(axum::body::Bytes::from(body))
        .await;
    allowed.assert_status_ok();

    assert_eq!(SERVICE_PREFIX, "/github-connect");
}

#[test]
fn empty_hmac_secret_fails_closed_at_boot() {
    let mut config = test_config("http://127.0.0.1:9");
    config.github.hmac_secret = "   ".to_owned();
    assert!(Application::new(config).is_err());
}

#[test]
fn fifteen_byte_hmac_secret_fails_closed_at_boot() {
    let mut config = test_config("http://127.0.0.1:9");
    config.github.hmac_secret = "123456789012345".to_owned();
    assert!(Application::new(config).is_err());
}
