// Include common test utilities and fixtures
#[path = "common/mod.rs"] 
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::{setup_test_server, TestSqsFixture, TestFixture};
use serde_json::{json, Value};
use serial_test::serial;
use configuration;



// 🔥 SQS Integration Test
#[tokio::test]
#[serial]
#[ignore]
async fn test_signup_sqs_integration() {
    // Setup SQS testcontainer first (this sets environment variables)
    let sqs_fixture = TestSqsFixture::new().await
        .expect("Failed to start SQS LocalStack testcontainer");
    
    println!("🔧 SQS LocalStack container started on: {}", sqs_fixture.sqs.endpoint_url);
    println!("🔧 Queue URL: {}", sqs_fixture.sqs.queue_url);
    
    // Clear configuration cache and restart with test environment
    configuration::clear_config_cache();
    
    // Load configuration
    let config = configuration::load_config().expect("Should load config");
    
    // Setup database fixture
    let _db_fixture = TestFixture::new().await
        .expect("Failed to create test fixture");
    
    // Wait longer for SQS LocalStack to be fully ready
    println!("⏳ Waiting for SQS LocalStack to be fully ready...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Start the regular test server (it will pick up SQS config from environment)
    let (_fixture, base_url, client) = setup_test_server().await.expect("Failed to setup test server");
    
    // Wait a moment for server to be fully ready
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    // Verify SQS configuration
    match &config.queue {
        configuration::QueueConfig::Sqs(sqs_config) => {
            println!("🔧 SQS config - enabled: {}, endpoint: {}, region: {}", 
                     sqs_config.enabled, 
                     sqs_config.endpoint_url().unwrap_or("none".to_string()), 
                     sqs_config.region);
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
    let response_body: Value = response
        .json()
        .await
        .expect("Should return JSON response");

    assert!(response_body.get("message").is_some(), "Should contain success message");
    println!("✅ Signup request successful");
    
    // Wait more time for event to be published and processed
    println!("⏳ Waiting for event to be published to SQS...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Try to get any messages first (debugging)
    println!("🔍 Checking for any messages in SQS queue...");
    let all_messages = sqs_fixture.sqs.get_all_messages(3).await
        .expect("Should be able to get messages");
    println!("📊 Found {} total messages", all_messages.len());
    
    for (i, msg) in all_messages.iter().enumerate() {
        println!("📝 Message {}: {}", i + 1, msg);
    }
    
    // If no messages found, let's try waiting longer
    if all_messages.is_empty() {
        println!("⚠️ No messages found, waiting longer...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        let retry_messages = sqs_fixture.sqs.get_all_messages(5).await
            .expect("Should be able to get messages on retry");
        println!("🔄 Retry found {} messages", retry_messages.len());
        
        if retry_messages.is_empty() {
            // Print environment variables for debugging
            println!("🔧 Environment variables:");
            if let Ok(val) = std::env::var("IAM_QUEUE__TYPE") {
                println!("   IAM_QUEUE__TYPE: {}", val);
            }
            if let Ok(val) = std::env::var("IAM_QUEUE__ENABLED") {
                println!("   IAM_QUEUE__ENABLED: {}", val);
            }
            if let Ok(val) = std::env::var("IAM_QUEUE__HOST") {
                println!("   IAM_QUEUE__HOST: {}", val);
            }
            if let Ok(val) = std::env::var("IAM_QUEUE__PORT") {
                println!("   IAM_QUEUE__PORT: {}", val);
            }
            if let Ok(val) = std::env::var("IAM_QUEUE__SQS__ACCOUNT_ID") {
                println!("   IAM_QUEUE__SQS__ACCOUNT_ID: {}", val);
            }
            if let Ok(val) = std::env::var("IAM_QUEUE__SQS__DEFAULT_QUEUE") {
                println!("   IAM_QUEUE__SQS__DEFAULT_QUEUE: {}", val);
            }
            
            panic!("No messages found in SQS queue after retries. This suggests the event is not being published.");
        }
    }
    
    let messages = if all_messages.is_empty() {
        sqs_fixture.sqs.get_all_messages(2).await.expect("Should get messages")
    } else {
        all_messages
    };
    
    // Should have at least one message
    assert!(!messages.is_empty(), "Should have at least one event published to SQS");
    
    // ✅ Find and verify the UserSignedUp event
    let mut found_signup_event = false;
    for (i, message) in messages.iter().enumerate() {
        println!("📝 Message {}: {}", i + 1, message);
        
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(message) {
            if let Some(event_type) = event.get("event_type") {
                if event_type == "user_signed_up" {
                    found_signup_event = true;
                    
                    println!("🔍 Event: {:?}", event);
                    // Verify event structure
                    assert!(event.get("event_id").is_some(), "Should have event_id");
                    assert!(event.get("aggregate_id").is_some(), "Should have aggregate_id (user_id)");
                    assert!(event.get("occurred_at").is_some(), "Should have occurred_at");
                    
                    // Parse the data field to get the actual event data
                    let user_id = if let Some(data_str) = event.get("data").and_then(|d| d.as_str()) {
                        if let Ok(data_json) = serde_json::from_str::<serde_json::Value>(data_str) {
                            data_json.get("user_id").and_then(|u| u.as_str()).map(|s| s.to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    
                    // Verify user_id exists in data or use aggregate_id
                    let user_id_value = user_id.or_else(|| {
                        event.get("aggregate_id").and_then(|a| a.as_str()).map(|s| s.to_string())
                    });
                    assert!(user_id_value.is_some(), "Should have user_id in data or aggregate_id");
                    
                    // Verify event data matches our signup (check metadata fields)
                    if let Some(metadata) = event.get("metadata") {
                        if let Some(email) = metadata.get("email") {
                            assert_eq!(email.as_str().unwrap(), test_email, "Email should match signup data");
                        }
                        if let Some(username) = metadata.get("username") {
                            assert_eq!(username.as_str().unwrap(), test_username, "Username should match signup data");
                        }
                        if let Some(email_verified) = metadata.get("email_verified") {
                            // email_verified might be a string "false" in metadata
                            let is_verified = match email_verified {
                                serde_json::Value::Bool(b) => *b,
                                serde_json::Value::String(s) => s == "true",
                                _ => false,
                            };
                            assert_eq!(is_verified, false, "Email should not be verified initially");
                        }
                    }
                    
                    println!("✅ Found and verified UserSignedUp event");
                    break;
                }
            }
        }
    }
    
    assert!(found_signup_event, "Should find a UserSignedUp event in SQS queue");
    
    println!("✅ SQS integration test completed successfully");
    println!("   - Test server started with SQS enabled");
    println!("   - SQS LocalStack running on: {}", sqs_fixture.sqs.endpoint_url);
    println!("   - User signup succeeded");
    println!("   - UserSignedUp event verified in SQS queue");
    println!("   - Event structure and data validated");
} 