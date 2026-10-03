//! Black-box SQS consumer resilience checks.

mod common;

use common::*;
use iam_events::{IamDomainEvent, UserSignedUpEvent};
use rustycog::events::event::BaseEvent;
use serial_test::serial;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn malformed_sqs_payload_has_no_email_side_effect_and_does_not_stop_the_consumer() {
    let (fixture, _, _, _openfga) = setup_test_server().await.unwrap();
    let sqs = fixture.sqs();
    let smtp = fixture.smtp();

    sqs.send_message("{this-is-not-json")
        .await
        .expect("the real SQS fixture must accept the malformed producer payload");
    sleep(Duration::from_secs(2)).await;
    assert_eq!(
        smtp.email_count().await,
        0,
        "invalid payloads must not send mail"
    );

    let user_id = Uuid::new_v4();
    let email = "consumer-still-alive@example.test";
    let event = IamDomainEvent::UserSignedUp(UserSignedUpEvent {
        base: BaseEvent::new("user_signed_up".to_string(), user_id),
        user_id,
        email: email.to_string(),
        username: "consumer-resilience".to_string(),
        email_verified: false,
        verification_token: Some("resilience-token".to_string()),
        verification_url: None,
    });
    sqs.send_event(&event)
        .await
        .expect("the valid event must be accepted after the malformed one");

    for _ in 0..20 {
        if smtp
            .has_email("Welcome ! Please validate your email", email)
            .await
        {
            return;
        }
        sleep(Duration::from_secs(1)).await;
    }

    panic!("the real Telegraph consumer did not recover to process the valid event");
}
