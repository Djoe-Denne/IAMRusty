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
use rustycog_events::{
    event::{BaseEvent, DomainEvent},
    EventPublisher,
};
use serial_test::serial;
use uuid::Uuid;
use wiremock::matchers::body_string_contains;

/// Test that Telegraph correctly processes UserSignedUp events and sends welcome emails
#[tokio::test]
#[serial]
async fn test_user_signed_up_event_happy_path() {
    // Setup test infrastructure with real producer/consumer and SMTP testcontainer
    let (fixture, _, _) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    println!("Test server setup complete");

    let test_event_publisher = fixture.sqs();
    let smtp_container = fixture.smtp();

    // Create expected email data
    let user_id = Uuid::new_v4();
    let test_email = "test.user@example.com";
    let test_username = "testuser";

    // Create a test UserSignedUp event
    let user_signed_up_event = UserSignedUpEvent {
        base: BaseEvent::new("user_signed_up".to_string(), user_id),
        user_id,
        email: test_email.to_string(),
        username: test_username.to_string(),
        email_verified: false,
        verification_token: Some("test-verification-token-123".to_string()),
        verification_url: Some("/api/auth/verify".to_string()), // Telegraph will build URL from environment variables
    };

    let iam_event = IamDomainEvent::UserSignedUp(user_signed_up_event);

    // Publish the event using the test event publisher (routes directly to consumer)
    let result = test_event_publisher.send_event(&iam_event).await;

    println!("Event published successfully: {:?}", result);

    // Verify event was published successfully
    assert!(
        result.is_ok(),
        "Event should be published successfully: {:?}",
        result
    );

    let mut has_email = false;
    for i in 0..25 {
        println!(
            "Waiting for event to be processed and email to be sent: {}",
            i
        );
        // Wait for the event to be processed and email to be sent
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        if smtp_container
            .has_email("Welcome ! Please validate your email", test_email)
            .await
        {
            has_email = true;
            break;
        }
    }

    // Verify that email was sent to MailHog
    assert!(has_email, "Welcome email should have been sent to the user");

    // Verify exactly one email was sent
    assert_eq!(
        smtp_container.email_count().await,
        1,
        "Exactly one email should have been sent"
    );

    // Get the emails for additional verification
    let emails = smtp_container
        .get_emails()
        .await
        .expect("Failed to get emails");
    let email = &emails[0];
    let verification_url = "https://oodhive.org/api/auth/verify".to_string();

    // Verify email contains expected content
    assert!(
        email.text.contains(test_email),
        "Email should contain recipient address"
    );
    assert!(
        email.text.contains(test_username),
        "Email should contain username in content: {}",
        email.text
    );
    assert!(
        email.text.contains(&verification_url),
        "Email should contain validation link"
    );
}

/// Test that Telegraph email processor handles UserSignedUp events with proper email content
#[tokio::test]
#[serial]
async fn test_user_signed_up_email_content_verification() {
    // Setup test infrastructure with real producer/consumer and SMTP testcontainer
    let (fixture, _, _) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let test_event_publisher = fixture.sqs();
    let smtp_container = fixture.smtp();

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
        verification_token: Some("test-verification-token-123".to_string()),
        verification_url: None, // Telegraph will build URL from environment variables
    };

    let iam_event = IamDomainEvent::UserSignedUp(user_signed_up_event);

    // Publish the event
    let result = test_event_publisher.send_event(&iam_event).await;
    assert!(result.is_ok(), "Event should be published successfully");

    for i in 0..10 {
        println!(
            "Waiting for event to be processed and email to be sent: {}",
            i
        );
        // Wait for the event to be processed and email to be sent
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        if smtp_container.email_count().await > 0 {
            break;
        }
    }
    // Verify the email was sent and get the content
    let emails = smtp_container
        .get_emails()
        .await
        .expect("Failed to get emails");
    assert_eq!(emails.len(), 1, "Should have exactly one email");

    let email = &emails[0];

    // Verify email contains expected content
    assert!(
        email
            .subject
            .contains("Welcome ! Please validate your email"),
        "Email should contain welcome subject"
    );
    assert!(
        email
            .to
            .iter()
            .any(|addr| addr.address.contains(test_email)),
        "Email should contain recipient address"
    );
    assert!(
        email.text.contains(test_username),
        "Email should contain username in content: {}",
        email.text
    );
    assert!(
        email.from.address.contains("noreply@telegraph.com"),
        "Email should be sent from correct address"
    );
}

/// Test error handling when processing malformed events
#[tokio::test]
#[serial]
async fn test_event_processing_error_handling() {
    // Setup test infrastructure with real producer/consumer and SMTP testcontainer
    let (fixture, _, _) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let test_event_publisher = fixture.sqs();
    let smtp_container = fixture.smtp();

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
    let login_result = test_event_publisher.send_event(&login_iam_event).await;

    // Verify event was published (should not fail)
    assert!(
        login_result.is_ok(),
        "UserLoggedIn event should be published successfully"
    );

    // Wait for potential processing
    tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;

    // Verify no email was sent for UserLoggedIn event
    assert_eq!(
        smtp_container.email_count().await,
        0,
        "No email should be sent for UserLoggedIn event"
    );
}

/// Test event type support verification using real infrastructure  
#[tokio::test]
#[serial]
async fn test_event_type_support_verification() {
    // Setup test infrastructure with real producer/consumer and SMTP testcontainer
    let (fixture, _, _) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let test_event_publisher = fixture.sqs();
    let smtp_container = fixture.smtp();

    // Test multiple event types to verify they're all processed
    let user_id = Uuid::new_v4();
    let test_email = "test@example.com";
    let test_username = "testuser";

    // 1. UserSignedUp event - should trigger welcome email
    let signup_event = iam_events::UserSignedUpEvent {
        base: BaseEvent::new("user_signed_up".to_string(), user_id),
        user_id,
        email: test_email.to_string(),
        username: test_username.to_string(),
        email_verified: false,
        verification_token: Some("test-verification-multi-token-789".to_string()),
        verification_url: None, // Telegraph will build URL from environment variables
    };

    let signup_iam_event = IamDomainEvent::UserSignedUp(signup_event);
    let result = test_event_publisher.send_event(&signup_iam_event).await;

    assert!(
        result.is_ok(),
        "UserSignedUp event should be published and processed successfully"
    );

    // Wait for processing
    for i in 0..10 {
        println!(
            "Waiting for event to be processed and email to be sent: {}",
            i
        );
        // Wait for the event to be processed and email to be sent
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        if smtp_container.email_count().await > 0 {
            break;
        }
    }

    // Verify welcome email was sent for UserSignedUp
    assert_eq!(
        smtp_container.email_count().await,
        1,
        "Welcome email should be sent for UserSignedUp event"
    );

    // Clear emails for next test
    smtp_container
        .clear_emails()
        .await
        .expect("Failed to clear emails");

    // 2. UserLoggedIn event - should be processed but no welcome email
    let login_event = iam_events::UserLoggedInEvent {
        base: BaseEvent::new("user_logged_in".to_string(), user_id),
        user_id,
        email: test_email.to_string(),
        login_method: "email_password".to_string(),
    };

    let login_iam_event = IamDomainEvent::UserLoggedIn(login_event);
    let result = test_event_publisher.send_event(&login_iam_event).await;
    assert!(
        result.is_ok(),
        "UserLoggedIn event should be published and processed successfully"
    );

    // Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;

    // Verify no email was sent for UserLoggedIn event
    assert_eq!(
        smtp_container.email_count().await,
        0,
        "No email should be sent for UserLoggedIn event"
    );
}
