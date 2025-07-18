//! Integration tests for user signup event processing in Telegraph
//! 
//! Tests the happy path for user_signed_up event processing where:
//! 1. Telegraph receives a UserSignedUp event
//! 2. Telegraph processes the event and sends a welcome email
//! 3. We verify the email was sent correctly

mod common;

use common::*;

#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::SmtpFixtures;
use iam_events::{IamDomainEvent, UserSignedUpEvent};
use rustycog_events::{event::BaseEvent, EventPublisher};
use serial_test::serial;
use uuid::Uuid;
use wiremock::matchers::body_string_contains;

/// Test that Telegraph correctly processes UserSignedUp events and sends welcome emails
#[tokio::test]
#[serial]
async fn test_user_signed_up_event_happy_path() {
    // Setup test infrastructure with real producer/consumer
    let (fixture, _, _, test_event_publisher) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    // Setup SMTP mock server
    let smtp_service = SmtpFixtures::service().await;
    
    // Create expected email data
    let user_id = Uuid::new_v4();
    let test_email = "test.user@example.com";
    let test_username = "testuser";
    
    let expected_email = fixtures::smtp::resources::SmtpEmail::user_signup_welcome(test_email, test_username);
    
    // Mock successful email sending sequence
    smtp_service.mock_successful_email_send(&expected_email).await;
    
    // Create a test UserSignedUp event
    let user_signed_up_event = UserSignedUpEvent {
        base: BaseEvent::new("user_signed_up".to_string(), user_id),
        user_id,
        email: test_email.to_string(),
        username: test_username.to_string(),
        email_verified: false,
    };

    let iam_event = IamDomainEvent::UserSignedUp(user_signed_up_event);
    
    // Publish the event using the test event publisher (routes directly to consumer)
    println!("🔍 Debug: Publishing event...");
    let result = test_event_publisher.publish(Box::new(iam_event.clone())).await;
    
    // Verify event was published successfully
    assert!(result.is_ok(), "Event should be published successfully: {:?}", result);
    
    // Wait for the event to be processed and email to be sent
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Verify that email was sent
    assert!(
        smtp_service.verify_email_sent("Welcome to AI For All!", test_email).await,
        "Welcome email should have been sent to the user"
    );
    
    // Verify exactly one email was sent
    assert_eq!(
        smtp_service.email_count().await,
        1,
        "Exactly one email should have been sent"
    );
    
    println!("✅ UserSignedUp event processed successfully and welcome email sent");
}

/// Test that Telegraph email processor handles UserSignedUp events with proper email content
#[tokio::test]
#[serial]
async fn test_user_signed_up_email_content_verification() {
    // Setup test infrastructure with real producer/consumer
    let (fixture, _, _, test_event_publisher) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    // Setup SMTP mock server
    let smtp_service = SmtpFixtures::service().await;

    // Create test data
    let user_id = Uuid::new_v4();
    let test_email = "content.test@example.com";
    let test_username = "content_tester";
    
    let expected_email = fixtures::smtp::resources::SmtpEmail::user_signup_welcome(test_email, test_username);
    
    // Mock successful email sending with detailed content verification
    smtp_service.mock_successful_email_send(&expected_email).await;
    
    let user_signed_up_event = UserSignedUpEvent {
        base: BaseEvent::new("user_signed_up".to_string(), user_id),
        user_id,
        email: test_email.to_string(),
        username: test_username.to_string(),
        email_verified: false,
    };

    let iam_event = IamDomainEvent::UserSignedUp(user_signed_up_event);

    // Publish the event
    let result = test_event_publisher.publish(Box::new(iam_event)).await;
    assert!(result.is_ok(), "Event should be published successfully");
    
    // Wait for event processing
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Verify the email content was sent correctly
    let sent_requests = smtp_service.received_requests().await;
    
    // Find the DATA request (contains email content)
    let data_request = sent_requests.iter()
        .find(|req| req.url.path() == "/smtp/data")
        .expect("Should have a DATA request for email content");
    
    let email_body = String::from_utf8_lossy(&data_request.body);
    
    // Verify email contains expected content
    assert!(
        email_body.contains("Welcome to AI For All!"),
        "Email should contain welcome subject"
    );
    assert!(
        email_body.contains(test_email),
        "Email should contain recipient address"
    );
    assert!(
        email_body.contains(test_username),
        "Email should contain username in content"
    );
    assert!(
        email_body.contains("noreply@telegraph.com"),
        "Email should be sent from correct address"
    );
    
    println!("✅ Email content verification successful");
}

/// Test error handling when processing malformed events
#[tokio::test]
#[serial]
async fn test_event_processing_error_handling() {
    // Setup test infrastructure with real producer/consumer
    let (fixture, _, _, test_event_publisher) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    // Setup SMTP mock server
    let smtp_service = SmtpFixtures::service().await;
    let expected_email = fixtures::smtp::resources::SmtpEmail::user_signup_welcome("test@example.com", "testuser");
    smtp_service.mock_successful_email_send(&expected_email).await;

    // Test with other event types that should be handled gracefully
    let user_id = Uuid::new_v4();
    
    // Test UserLoggedIn event (should be handled but not send welcome email)
    let login_event = iam_events::UserLoggedInEvent {
        base: BaseEvent::new("user_logged_in".to_string(), user_id),
        user_id,
        email: "test@example.com".to_string(),
        login_method: "email_password".to_string(),
    };

    let login_iam_event = IamDomainEvent::UserLoggedIn(login_event);
    let login_result = test_event_publisher.publish(Box::new(login_iam_event)).await;
    
    // Verify event was published (should not fail)
    assert!(login_result.is_ok(), "UserLoggedIn event should be published successfully");
    
    // Wait for potential processing
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    
    // Verify no email was sent for UserLoggedIn event
    assert_eq!(
        smtp_service.email_count().await,
        0,
        "No email should be sent for UserLoggedIn event"
    );
    
    println!("✅ Error handling test passed - non-signup events handled gracefully");
}

/// Test event type support verification using real infrastructure  
#[tokio::test]
#[serial]
async fn test_event_type_support_verification() {
    // Setup test infrastructure with real producer/consumer
    let (fixture, _, _, test_event_publisher) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    
    // Setup SMTP mock server
    let smtp_service = SmtpFixtures::service().await;
    
    // Test multiple event types to verify they're all processed
    let user_id = Uuid::new_v4();
    let test_email = "test@example.com";
    let test_username = "testuser";
    
    let expected_email = fixtures::smtp::resources::SmtpEmail::user_signup_welcome(test_email, test_username);
    smtp_service.mock_successful_email_send(&expected_email).await;
    
    // 1. UserSignedUp event - should trigger welcome email
    let signup_event = iam_events::UserSignedUpEvent {
        base: BaseEvent::new("user_signed_up".to_string(), user_id),
        user_id,
        email: test_email.to_string(),
        username: test_username.to_string(),
        email_verified: false,
    };
    
    let signup_iam_event = IamDomainEvent::UserSignedUp(signup_event);
    let result = test_event_publisher.publish(Box::new(signup_iam_event)).await;
  
    assert!(result.is_ok(), "UserSignedUp event should be published and processed successfully");
    
    // Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Verify welcome email was sent for UserSignedUp
    assert_eq!(
        smtp_service.email_count().await,
        1,
        "Welcome email should be sent for UserSignedUp event"
    );
    
    // Reset SMTP mock for next test
    smtp_service.reset().await;
    
    // 2. UserLoggedIn event - should be processed but no welcome email
    let login_event = iam_events::UserLoggedInEvent {
        base: BaseEvent::new("user_logged_in".to_string(), user_id),
        user_id,
        email: test_email.to_string(),
        login_method: "email_password".to_string(),
    };
    
    let login_iam_event = IamDomainEvent::UserLoggedIn(login_event);
    let result = test_event_publisher.publish(Box::new(login_iam_event)).await;
    assert!(result.is_ok(), "UserLoggedIn event should be published and processed successfully");
    
    // Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    
    // Verify no email was sent for UserLoggedIn event
    assert_eq!(
        smtp_service.email_count().await,
        0,
        "No email should be sent for UserLoggedIn event"
    );
    
    println!("✅ Event type support verification successful");
} 