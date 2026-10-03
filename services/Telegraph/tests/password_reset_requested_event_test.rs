//! Integration tests for `password_reset_requested` event processing.

mod common;

use common::*;

use chrono::{Duration, Utc};
use iam_events::{IamDomainEvent, PasswordResetRequestedEvent};
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_password_reset_requested_event_sends_email() {
    let (fixture, _, _, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let test_event_publisher = fixture.sqs();
    let smtp_container = fixture.smtp();

    let user_id = Uuid::new_v4();
    let test_email = "reset.user@example.com";
    let event = IamDomainEvent::PasswordResetRequested(PasswordResetRequestedEvent::new(
        user_id,
        test_email.to_string(),
        "reset-token-abc".to_string(),
        Utc::now() + Duration::hours(1),
    ));

    test_event_publisher
        .send_event(&event)
        .await
        .expect("event should be published");

    let mut has_email = false;
    for _ in 0..25 {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        if smtp_container
            .has_email("Password Reset Requested", test_email)
            .await
        {
            has_email = true;
            break;
        }
    }

    assert!(
        has_email,
        "password reset email should have been sent to the user"
    );
    assert_eq!(
        smtp_container.email_count().await,
        1,
        "exactly one email should have been sent"
    );
}
