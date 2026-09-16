//! Health and readiness under `SERVICE_PREFIX` (`/lazaret`).
//!
//! `setup_test_server` returns a base URL already nested via `create_prefixed_router`.

mod common;

use common::setup_test_server;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn health_returns_200() {
    let (_fixture, base_url, client) = setup_test_server().await.expect("serveur de test");
    assert!(
        base_url.ends_with("/lazaret"),
        "setup_test_server must expose prefixed base URL, got {base_url}"
    );
    let response = client
        .get(format!("{base_url}/health"))
        .send()
        .await
        .expect("GET /lazaret/health");
    assert_eq!(response.status(), 200, "GET /lazaret/health");
}

#[tokio::test]
#[serial]
async fn ready_returns_200() {
    let (_fixture, base_url, client) = setup_test_server().await.expect("serveur de test");
    assert!(
        base_url.ends_with("/lazaret"),
        "setup_test_server must expose prefixed base URL, got {base_url}"
    );
    let response = client
        .get(format!("{base_url}/ready"))
        .send()
        .await
        .expect("GET /lazaret/ready");
    assert_eq!(response.status(), 200, "GET /lazaret/ready");
}
