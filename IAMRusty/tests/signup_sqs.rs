// Include common test utilities and fixtures
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use serde_json::{json, Value};
use serial_test::serial;

use iam_configuration;
use iam_configuration::{clear_config_cache, load_config, QueueConfig};
use rustycog_testing::TestSqsFixture;
use std::sync::Arc;

// 🔥 SQS Integration Test
#[tokio::test]
#[serial]
#[ignore]
async fn test_signup_sqs_integration() {
    // Setup SQS testcontainer first (this sets environment variables)
    let sqs_fixture = TestSqsFixture::new()
        .await
        .expect("Failed to start SQS LocalStack testcontainer");

    println!(
        "🔧 SQS LocalStack container started on: {}",
        sqs_fixture.sqs.endpoint_url
    );
    println!("🔧 Queue URL: {}", sqs_fixture.sqs.queue_url);

    // Clear configuration cache and restart with test environment
    clear_config_cache();

    // Load configuration
    let config = load_config().expect("Should load config");

    // Setup database fixture
    let _db_fixture = TestFixture::new(Arc::new(IAMRustyTestDescriptor))
        .await
        .expect("Failed to create test fixture");

    // Wait longer for SQS LocalStack to be fully ready
    println!("⏳ Waiting for SQS LocalStack to be fully ready...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Start the regular test server (it will pick up SQS config from environment)
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");

    // Wait a moment for server to be fully ready
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Verify SQS configuration
    match &config.queue {
        QueueConfig::Sqs(sqs_config) => {
            println!(
                "🔧 SQS config - enabled: {}, endpoint: {}, region: {}",
                sqs_config.enabled,
                sqs_config.endpoint_url().unwrap_or("none".to_string()),
                sqs_config.region
            );
        }
        _ => {
            println!("🔧 Queue config is not SQS");
        }
    }

    // Create signup request
    let test_email = "sqs_test@example.com";
    let test_username = "sqs_test_user";
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
    println!("⏳ Waiting for event to be published to SQS...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let messages = collect_sqs_messages(&sqs_fixture).await;

    // Should have at least one message
    assert!(
        !messages.is_empty(),
        "Should have at least one event published to SQS"
    );

    assert_user_signed_up_sqs_event(&messages, test_email, test_username);

    println!("✅ SQS integration test completed successfully");
    println!("   - Test server started with SQS enabled");
    println!(
        "   - SQS LocalStack running on: {}",
        sqs_fixture.sqs.endpoint_url
    );
    println!("   - User signup succeeded");
    println!("   - UserSignedUp event verified in SQS queue");
    println!("   - Event structure and data validated");
}

async fn collect_sqs_messages(sqs_fixture: &TestSqsFixture) -> Vec<String> {
    println!("🔍 Checking for any messages in SQS queue...");
    let all_messages = sqs_fixture
        .sqs
        .get_all_messages(3)
        .await
        .expect("Should be able to get messages");
    print_messages(&all_messages);

    if !all_messages.is_empty() {
        return all_messages;
    }

    println!("⚠️ No messages found, waiting longer...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let retry_messages = sqs_fixture
        .sqs
        .get_all_messages(5)
        .await
        .expect("Should be able to get messages on retry");
    println!("🔄 Retry found {} messages", retry_messages.len());

    if retry_messages.is_empty() {
        print_sqs_environment();
        panic!(
            "No messages found in SQS queue after retries. This suggests the event is not being published."
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

fn print_sqs_environment() {
    println!("🔧 Environment variables:");
    for key in [
        "IAM_QUEUE__TYPE",
        "IAM_QUEUE__ENABLED",
        "IAM_QUEUE__HOST",
        "IAM_QUEUE__PORT",
        "IAM_QUEUE__SQS__ACCOUNT_ID",
        "IAM_QUEUE__SQS__DEFAULT_QUEUES",
    ] {
        if let Ok(val) = std::env::var(key) {
            println!("   {}: {}", key, val);
        }
    }
}

fn assert_user_signed_up_sqs_event(messages: &[String], test_email: &str, test_username: &str) {
    let found_signup_event = messages
        .iter()
        .enumerate()
        .filter_map(|(i, message)| parse_logged_message(i, message))
        .any(|event| verify_sqs_signup_event(&event, test_email, test_username));

    assert!(
        found_signup_event,
        "Should find a UserSignedUp event in SQS queue"
    );
}

fn parse_logged_message(index: usize, message: &str) -> Option<Value> {
    println!("📝 Message {}: {}", index + 1, message);
    serde_json::from_str::<Value>(message).ok()
}

fn verify_sqs_signup_event(event: &Value, test_email: &str, test_username: &str) -> bool {
    if event.get("event_type") != Some(&Value::String("user_signed_up".to_string())) {
        return false;
    }

    println!("🔍 Event: {:?}", event);
    assert!(event.get("event_id").is_some(), "Should have event_id");
    assert!(
        event.get("aggregate_id").is_some(),
        "Should have aggregate_id (user_id)"
    );
    assert!(
        event.get("occurred_at").is_some(),
        "Should have occurred_at"
    );
    assert!(
        event_user_id(event).is_some(),
        "Should have user_id in data or aggregate_id"
    );
    assert_signup_metadata(event, test_email, test_username);
    println!("✅ Found and verified UserSignedUp event");
    true
}

fn event_user_id(event: &Value) -> Option<String> {
    let user_id = event
        .get("data")
        .and_then(|d| d.as_str())
        .and_then(|data_str| serde_json::from_str::<Value>(data_str).ok())
        .and_then(|data_json| {
            data_json
                .get("user_id")
                .and_then(|u| u.as_str())
                .map(|s| s.to_string())
        });

    user_id.or_else(|| {
        event
            .get("aggregate_id")
            .and_then(|a| a.as_str())
            .map(|s| s.to_string())
    })
}

fn assert_signup_metadata(event: &Value, test_email: &str, test_username: &str) {
    let Some(metadata) = event.get("metadata") else {
        return;
    };

    if let Some(email) = metadata.get("email") {
        assert_eq!(
            email.as_str().unwrap(),
            test_email,
            "Email should match signup data"
        );
    }
    if let Some(username) = metadata.get("username") {
        assert_eq!(
            username.as_str().unwrap(),
            test_username,
            "Username should match signup data"
        );
    }
    if let Some(email_verified) = metadata.get("email_verified") {
        assert!(
            !metadata_bool(email_verified),
            "Email should not be verified initially"
        );
    }
}

fn metadata_bool(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::String(s) => s == "true",
        _ => false,
    }
}
