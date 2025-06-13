// Username Check API Tests
#[path = "common/mod.rs"] 
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::{get_test_server, TestFixture};
use fixtures::DbFixtures;
use reqwest::Client;
use serde_json::{json, Value};
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
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    let response = client
        .get(&format!("{}/api/auth/username/check", base_url))
        .query(&[("username", "available")])
        .send()
        .await
        .expect("Failed to check username");

    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.expect("Should return JSON");
    assert_eq!(body["available"].as_bool().unwrap(), true);
}

#[tokio::test]
#[serial]
async fn test_username_taken() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();
    let db = fixture.db();

    // Create user
    let _user = DbFixtures::user()
        .username("taken")
        .commit(db.clone())
        .await
        .expect("Failed to create user");

    let response = client
        .get(&format!("{}/api/auth/username/check", base_url))
        .query(&[("username", "taken")])
        .send()
        .await
        .expect("Failed to check username");

    assert_eq!(response.status(), 200);
    
    let body: Value = response.json().await.expect("Should return JSON");
    assert_eq!(body["available"].as_bool().unwrap(), false);
}

#[tokio::test]
#[serial]
async fn test_username_validation() {
    let base_url = get_test_server().await.expect("Failed to start test server");
    let _fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let client = create_test_client();

    // Too short
    let response = client
        .get(&format!("{}/api/auth/username/check", base_url))
        .query(&[("username", "ab")])
        .send()
        .await
        .expect("Failed to check username");

    assert!(response.status() == 400 || response.status() == 422);

    // Valid length
    let response = client
        .get(&format!("{}/api/auth/username/check", base_url))
        .query(&[("username", "validuser")])
        .send()
        .await
        .expect("Failed to check username");

    assert_eq!(response.status(), 200);
} 