// Include common test utilities and fixtures
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use serde_json::{json, Value};
use serial_test::serial;

use iam_configuration;
use iam_configuration::{clear_config_cache, load_config};
use rustycog_testing::TestKafkaFixture;
use std::sync::Arc;

// 🔥 Kafka Integration Test
#[tokio::test]
#[serial]
#[ignore]
async fn test_signup_kafka_integration() {
    // Setup Kafka testcontainer first (this sets environment variables)
    let kafka_fixture = TestKafkaFixture::new()
        .await
        .expect("Failed to start Kafka testcontainer");

    println!(
        "🔧 Kafka container started on: {}",
        kafka_fixture.kafka.brokers
    );
    println!("🔧 Topic: {}", kafka_fixture.kafka.topic);

    // Clear configuration cache and restart with test environment
    clear_config_cache();

    // Load configuration
    let config = load_config().expect("Should load config");

    // Setup database fixture
    let _db_fixture = TestFixture::new(Arc::new(IAMRustyTestDescriptor))
        .await
        .expect("Failed to create test fixture");

    // Wait longer for Kafka to be fully ready
    println!("⏳ Waiting for Kafka to be fully ready...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Start the regular test server (it will pick up Kafka config from environment)
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Wait a moment for server to be fully ready
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Verify Kafka configuration
    println!(
        "🔧 Kafka config - enabled: {}, brokers: {}, topic: {}",
        config.kafka.enabled,
        config.kafka.brokers(),
        config.kafka.user_events_topic
    );

    // Create signup request
    let test_email = "kafka_test@example.com";
    let test_username = "kafka_test_user";
    let signup_data = json!({
        "username": test_username,
        "email": test_email,
        "password": "securePassword123"
    });

    println!("🚀 Making signup request...");

    // Make signup request
    let response = client
        .post(&format!("{}/api/auth/signup", base_url))
        .header("Content-Type", "application/json")
        .json(&signup_data)
        .send()
        .await
        .expect("Failed to send signup request");

    // ✅ Should return 201 Created
    assert_eq!(response.status(), 201, "Should return 201 Created status");

    // ✅ Verify the signup was successful
    let response_body: Value = response.json().await.expect("Should return JSON response");

    assert!(
        response_body.get("message").is_some(),
        "Should contain success message"
    );
    println!("✅ Signup request successful");

    // Wait more time for event to be published and processed
    println!("⏳ Waiting for event to be published to Kafka...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let messages = collect_kafka_messages(&kafka_fixture).await;

    // Should have at least one message
    assert!(
        !messages.is_empty(),
        "Should have at least one event published to Kafka"
    );

    assert_user_signed_up_kafka_event(&messages, test_email, test_username);

    println!("✅ Kafka integration test completed successfully");
    println!("   - Test server started with Kafka enabled");
    println!(
        "   - Kafka container running on: {}",
        kafka_fixture.kafka.brokers
    );
    println!("   - User signup succeeded");
    println!("   - UserSignedUp event verified in Kafka topic");
    println!("   - Event structure and data validated");
}

async fn collect_kafka_messages(kafka_fixture: &TestKafkaFixture) -> Vec<String> {
    println!("🔍 Checking for any messages in topic...");
    let all_messages = kafka_fixture
        .kafka
        .get_all_messages(3)
        .await
        .expect("Should be able to get messages");
    print_messages(&all_messages);

    if !all_messages.is_empty() {
        return all_messages;
    }

    println!("⚠️ No messages found, waiting longer...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let retry_messages = kafka_fixture
        .kafka
        .get_all_messages(5)
        .await
        .expect("Should be able to get messages on retry");
    println!("🔄 Retry found {} messages", retry_messages.len());

    if retry_messages.is_empty() {
        print_kafka_environment();
        panic!(
            "No messages found in Kafka topic after retries. This suggests the event is not being published."
        );
    }

    retry_messages
}

fn print_messages(messages: &[String]) {
    println!("📊 Found {} total messages", messages.len());
    for (i, msg) in messages.iter().enumerate() {
        println!("📝 Message {}: {}", i + 1, msg);
    }
}

fn print_kafka_environment() {
    println!("🔧 Environment variables:");
    for key in [
        "RUSTYCOG_KAFKA__ENABLED",
        "RUSTYCOG_KAFKA__HOST",
        "RUSTYCOG_KAFKA__PORT",
    ] {
        if let Ok(val) = std::env::var(key) {
            println!("   {}: {}", key, val);
        }
    }
}

fn assert_user_signed_up_kafka_event(messages: &[String], test_email: &str, test_username: &str) {
    let found_signup_event = messages
        .iter()
        .enumerate()
        .filter_map(|(i, message)| parse_logged_message(i, message))
        .any(|event| verify_kafka_signup_event(&event, test_email, test_username));

    assert!(
        found_signup_event,
        "Should find a UserSignedUp event in Kafka topic"
    );
}

fn parse_logged_message(index: usize, message: &str) -> Option<Value> {
    println!("📝 Message {}: {}", index + 1, message);
    serde_json::from_str::<Value>(message).ok()
}

fn verify_kafka_signup_event(event: &Value, test_email: &str, test_username: &str) -> bool {
    if event.get("event_type") != Some(&Value::String("user_signed_up".to_string())) {
        return false;
    }

    assert!(event.get("event_id").is_some(), "Should have event_id");
    assert!(event.get("user_id").is_some(), "Should have user_id");
    assert!(
        event.get("occurred_at").is_some(),
        "Should have occurred_at"
    );
    assert_optional_string(event, "email", test_email, "Email should match signup data");
    assert_optional_string(
        event,
        "username",
        test_username,
        "Username should match signup data",
    );
    if let Some(email_verified) = event.get("email_verified") {
        assert!(
            !email_verified.as_bool().unwrap(),
            "Email should not be verified initially"
        );
    }

    println!("✅ Found and verified UserSignedUp event");
    true
}

fn assert_optional_string(event: &Value, key: &str, expected: &str, message: &str) {
    if let Some(value) = event.get(key) {
        assert_eq!(value.as_str().unwrap(), expected, "{}", message);
    }
}
