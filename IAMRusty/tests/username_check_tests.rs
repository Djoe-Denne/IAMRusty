// Username Check API Tests

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::setup_test_server;
use fixtures::DbFixtures;
use reqwest::Client;
use serde_json::Value;
use serial_test::serial;

fn create_test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client")
}

#[tokio::test]
#[serial]
async fn test_username_available() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", "available")])
        .send()
        .await
        .expect("Failed to check username");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Should return JSON");
    assert!(body["available"].as_bool().unwrap());
}

#[tokio::test]
#[serial]
async fn test_username_taken() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    // Create user
    let _user = DbFixtures::user()
        .username("taken")
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", "taken")])
        .send()
        .await
        .expect("Failed to check username");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Should return JSON");
    assert!(!body["available"].as_bool().unwrap());
}

#[tokio::test]
#[serial]
async fn test_username_validation() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Too short
    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", "ab")])
        .send()
        .await
        .expect("Failed to check username");

    assert!(response.status() == 400 || response.status() == 422);

    // Valid length
    let response = client
        .get(format!("{base_url}/api/auth/username/check"))
        .query(&[("username", "validuser")])
        .send()
        .await
        .expect("Failed to check username");

    assert_eq!(response.status(), 200);
}
