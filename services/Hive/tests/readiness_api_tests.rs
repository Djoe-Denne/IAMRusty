//! Black-box readiness checks against the live Hive HTTP service.
//!
//! The descriptor deliberately disables the optional queue transport.  This
//! exercises the production `/hive/ready` route with a real PostgreSQL
//! dependency while asserting that optional infrastructure does not make the
//! service unavailable.

mod common;

use reqwest::StatusCode;
use serial_test::serial;

use common::setup_test_server;

#[tokio::test]
#[serial]
async fn ready_is_healthy_when_postgres_is_available_and_queue_is_optional() {
    let (_fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();

    let response = client
        .get(format!("{server_url}/ready"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "ready");
    assert!(body["checks"].is_object());
}
