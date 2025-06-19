// Include common test utilities and fixtures
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::{TestFixture, TestKafkaFixture, setup_test_server};
use configuration;
use serde_json::{Value, json};
use serial_test::serial;

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
    configuration::clear_config_cache();

    // Load configuration
    let config = configuration::load_config().expect("Should load config");

    // Setup database fixture
    let _db_fixture = TestFixture::new()
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

    // Try to get any messages first (debugging)
    println!("🔍 Checking for any messages in topic...");
    let all_messages = kafka_fixture
        .kafka
        .get_all_messages(3)
        .await
        .expect("Should be able to get messages");
    println!("📊 Found {} total messages", all_messages.len());

    for (i, msg) in all_messages.iter().enumerate() {
        println!("📝 Message {}: {}", i + 1, msg);
    }

    // If no messages found, let's try waiting longer
    if all_messages.is_empty() {
        println!("⚠️ No messages found, waiting longer...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let retry_messages = kafka_fixture
            .kafka
            .get_all_messages(5)
            .await
            .expect("Should be able to get messages on retry");
        println!("🔄 Retry found {} messages", retry_messages.len());

        if retry_messages.is_empty() {
            // Print environment variables for debugging
            println!("🔧 Environment variables:");
            if let Ok(val) = std::env::var("RUSTYCOG_KAFKA__ENABLED") {
                println!("   RUSTYCOG_KAFKA__ENABLED: {}", val);
            }
            if let Ok(val) = std::env::var("RUSTYCOG_KAFKA__HOST") {
                println!("   RUSTYCOG_KAFKA__HOST: {}", val);
            }
            if let Ok(val) = std::env::var("RUSTYCOG_KAFKA__PORT") {
                println!("   RUSTYCOG_KAFKA__PORT: {}", val);
            }

            panic!(
                "No messages found in Kafka topic after retries. This suggests the event is not being published."
            );
        }
    }

    let messages = if all_messages.is_empty() {
        kafka_fixture
            .kafka
            .get_all_messages(2)
            .await
            .expect("Should get messages")
    } else {
        all_messages
    };

    // Should have at least one message
    assert!(
        !messages.is_empty(),
        "Should have at least one event published to Kafka"
    );

    // ✅ Find and verify the UserSignedUp event
    let mut found_signup_event = false;
    for (i, message) in messages.iter().enumerate() {
        println!("📝 Message {}: {}", i + 1, message);

        if let Ok(event) = serde_json::from_str::<serde_json::Value>(message) {
            if let Some(event_type) = event.get("event_type") {
                if event_type == "user_signed_up" {
                    found_signup_event = true;

                    // Verify event structure
                    assert!(event.get("event_id").is_some(), "Should have event_id");
                    assert!(event.get("user_id").is_some(), "Should have user_id");
                    assert!(
                        event.get("occurred_at").is_some(),
                        "Should have occurred_at"
                    );

                    // Verify event data matches our signup
                    if let Some(email) = event.get("email") {
                        assert_eq!(
                            email.as_str().unwrap(),
                            test_email,
                            "Email should match signup data"
                        );
                    }
                    if let Some(username) = event.get("username") {
                        assert_eq!(
                            username.as_str().unwrap(),
                            test_username,
                            "Username should match signup data"
                        );
                    }
                    if let Some(email_verified) = event.get("email_verified") {
                        assert_eq!(
                            email_verified.as_bool().unwrap(),
                            false,
                            "Email should not be verified initially"
                        );
                    }

                    println!("✅ Found and verified UserSignedUp event");
                    break;
                }
            }
        }
    }

    assert!(
        found_signup_event,
        "Should find a UserSignedUp event in Kafka topic"
    );

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
