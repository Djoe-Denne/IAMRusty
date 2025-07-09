//! Integration tests for user signup event processing in Telegraph
//! 
//! Tests the happy path for user_signed_up event processing where:
//! 1. Telegraph receives a UserSignedUp event
//! 2. Telegraph processes the event and sends a welcome email
//! 3. We verify the email was sent correctly

mod common;

use common::*;
use iam_events::{IamDomainEvent, UserSignedUpEvent};
use rustycog_events::event::BaseEvent;
use domain::IamEventHandler;
use serial_test::serial;
use uuid::Uuid;

/// Test that Telegraph correctly processes UserSignedUp events and sends welcome emails
#[tokio::test]
#[serial]
async fn test_user_signed_up_event_happy_path() {
    // Setup test infrastructure with real producer/consumer
    let (fixture, test_event_publisher) = setup_telegraph_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    // Create a test UserSignedUp event
    let user_id = Uuid::new_v4();
    let test_email = "test.user@example.com";
    let test_username = "testuser";
    
    let user_signed_up_event = UserSignedUpEvent {
        base: BaseEvent::new("user_signed_up".to_string(), user_id),
        user_id,
        email: test_email.to_string(),
        username: test_username.to_string(),
        email_verified: false,
    };

    let iam_event = IamDomainEvent::UserSignedUp(user_signed_up_event);

    println!("📧 Testing user signup event processing...");
    println!("   User ID: {}", user_id);
    println!("   Email: {}", test_email);
    println!("   Username: {}", test_username);

    // Use the fixture's services instead of creating new ones
    let mock_email_service = fixture.mock_email_service();

    // Clear any previous emails from the mock service
    mock_email_service.clear_sent_emails();
    
    // Publish the event using the test event publisher (routes directly to consumer)
    let result = test_event_publisher.publish(Box::new(iam_event)).await;
    
    // ✅ Verify event publishing was successful
    assert!(
        result.is_ok(), 
        "Event publishing should succeed, but got error: {:?}", 
        result.err()
    );

    println!("✅ Event published and processed successfully through real infrastructure");
    
    // ✅ Verify that an email was sent through the mock service
    let sent_emails = mock_email_service.sent_emails();
    assert!(
        !sent_emails.is_empty(),
        "At least one email should have been sent for user signup"
    );
    
    // ✅ Verify the welcome email was sent to the correct recipient
    assert!(
        mock_email_service.was_email_sent_to(test_email),
        "Welcome email should have been sent to the user's email address"
    );
    
    let user_emails = mock_email_service.emails_for_recipient(test_email);
    assert!(!user_emails.is_empty(), "User should have received emails");
    
    let welcome_email = &user_emails[0];
    assert!(
        welcome_email.subject.contains("Welcome"),
        "Email subject should contain 'Welcome', got: '{}'", 
        welcome_email.subject
    );
    
    assert!(
        welcome_email.text_body.contains(test_username),
        "Email body should contain the username '{}', got: '{}'", 
        test_username, welcome_email.text_body
    );

    println!("✅ Welcome email verified - sent to correct recipient with proper content");
    println!("✅ Integration test completed: event published → processed → email sent");
}

/// Test that Telegraph email processor handles UserSignedUp events with proper email content
#[tokio::test]
#[serial]
async fn test_user_signed_up_email_content_verification() {
    // Setup test infrastructure with real producer/consumer
    let (fixture, test_event_publisher) = setup_telegraph_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    // Create test data
    let user_id = Uuid::new_v4();
    let test_email = "content.test@example.com";
    let test_username = "content_tester";
    
    let user_signed_up_event = UserSignedUpEvent {
        base: BaseEvent::new("user_signed_up".to_string(), user_id),
        user_id,
        email: test_email.to_string(),
        username: test_username.to_string(),
        email_verified: false,
    };

    let iam_event = IamDomainEvent::UserSignedUp(user_signed_up_event);

    println!("🔍 Testing welcome email content verification...");
    
    // Get mock email service for verification
    let mock_email_service = fixture.mock_email_service();
    mock_email_service.clear_sent_emails();

    // Publish the event using the test event publisher
    let result = test_event_publisher.publish(Box::new(iam_event)).await;

    // ✅ Verify event publishing and processing succeeded
    assert!(result.is_ok(), "Email event publishing and processing should succeed");
    
    // ✅ Verify email content in detail
    let sent_emails = mock_email_service.sent_emails();
    assert!(!sent_emails.is_empty(), "At least one email should have been sent");
    
    let welcome_email = &sent_emails[0];
    
    // Verify subject contains welcome message
    assert!(
        welcome_email.subject.contains("Welcome"),
        "Subject should contain 'Welcome', got: '{}'", 
        welcome_email.subject
    );
    
    // Verify body contains the username
    assert!(
        welcome_email.text_body.contains(test_username),
        "Body should contain username '{}', got: '{}'", 
        test_username, welcome_email.text_body
    );
    
    // Verify email is sent to correct recipient
    assert_eq!(welcome_email.to, test_email, "Email should be sent to correct recipient");
    
    // Verify HTML version exists if provided
    if let Some(html_body) = &welcome_email.html_body {
        assert!(
            html_body.contains(test_username),
            "HTML body should also contain username '{}', got: '{}'", 
            test_username, html_body
        );
    }

    println!("✅ Welcome email content verification completed successfully");
}

/// Test error handling when processing malformed events
#[tokio::test]
#[serial]
async fn test_event_processing_error_handling() {
    // Setup test infrastructure with real producer/consumer
    let (fixture, test_event_publisher) = setup_telegraph_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    println!("⚠️ Testing error handling for event processing...");

    // Get mock email service for verification
    let mock_email_service = fixture.mock_email_service();
    mock_email_service.clear_sent_emails();

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

    // ✅ Login events should be published and processed successfully
    assert!(
        login_result.is_ok(),
        "Login event publishing and processing should succeed, got error: {:?}",
        login_result.err()
    );

    // ✅ Verify that no welcome emails were sent for login events
    let sent_emails = mock_email_service.sent_emails();
    let welcome_emails: Vec<_> = sent_emails.iter()
        .filter(|email| email.subject.contains("Welcome"))
        .collect();
    
    assert!(
        welcome_emails.is_empty(),
        "No welcome emails should be sent for login events"
    );

    println!("✅ Login event handled correctly (no welcome email sent)");
}

/// Test event type support verification using real infrastructure  
#[tokio::test]
#[serial]
async fn test_event_type_support_verification() {
    // Setup test infrastructure with real producer/consumer
    let (fixture, test_event_publisher) = setup_telegraph_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    println!("🔧 Testing event type support verification through real infrastructure...");

    // ✅ Test that the event consumer supports the expected event types
    let event_consumer = fixture.event_consumer();
    assert!(
        event_consumer.supports_event_type("user_signed_up"),
        "Event consumer should support user_signed_up events"
    );
    assert!(
        event_consumer.supports_event_type("user_logged_in"),
        "Event consumer should support user_logged_in events"
    );
    println!("✅ Event consumer supports expected IAM event types");

    // ✅ Test end-to-end event processing for different event types
    let mock_email_service = fixture.mock_email_service();
    mock_email_service.clear_sent_emails();

    // Test multiple event types to verify they're all processed
    let user_id = Uuid::new_v4();
    
    // 1. UserSignedUp event - should trigger welcome email
    let signup_event = iam_events::UserSignedUpEvent {
        base: BaseEvent::new("user_signed_up".to_string(), user_id),
        user_id,
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        email_verified: false,
    };
    
    let signup_iam_event = IamDomainEvent::UserSignedUp(signup_event);
    let result = test_event_publisher.publish(Box::new(signup_iam_event)).await;
    assert!(result.is_ok(), "UserSignedUp event should be published and processed successfully");
    
    // 2. UserLoggedIn event - should be processed but no welcome email
    let login_event = iam_events::UserLoggedInEvent {
        base: BaseEvent::new("user_logged_in".to_string(), user_id),
        user_id,
        email: "test@example.com".to_string(),
        login_method: "email_password".to_string(),
    };
    
    let login_iam_event = IamDomainEvent::UserLoggedIn(login_event);
    let result = test_event_publisher.publish(Box::new(login_iam_event)).await;
    assert!(result.is_ok(), "UserLoggedIn event should be published and processed successfully");
    
    // ✅ Verify that only signup events trigger welcome emails
    let sent_emails = mock_email_service.sent_emails();
    let welcome_emails: Vec<_> = sent_emails.iter()
        .filter(|email| email.subject.contains("Welcome"))
        .collect();
    
    assert_eq!(
        welcome_emails.len(), 1,
        "Only one welcome email should be sent (for signup event)"
    );

    println!("✅ End-to-end event publishing and processing verified for multiple event types");
    println!("✅ Event type support verification completed successfully");
} 