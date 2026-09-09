//! Outbound `notification_created` routing after inbound IAM events.

mod common;

use common::*;
use iam_events::{IamDomainEvent, UserEmailVerifiedEvent};
use rustycog::events::event::BaseEvent;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serial_test::serial;
use telegraph_infra::repository::entity::notifications;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

const OUTBOUND_QUEUE: &str = "test-telegraph-outbound";

#[tokio::test]
#[serial]
async fn user_email_verified_publishes_notification_created_to_outbound_queue() {
    let (fixture, _, _, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let sqs = fixture.sqs();
    let db = fixture.db();
    let user_id = Uuid::new_v4();
    let short_id = &user_id.to_string()[..8];
    let email = format!("notify-{short_id}@example.test");
    let event = IamDomainEvent::UserEmailVerified(UserEmailVerifiedEvent {
        base: BaseEvent::new("user_email_verified".to_string(), user_id),
        user_id,
        email: email.clone(),
    });

    sqs.send_event(&event)
        .await
        .expect("verified event should be published");

    let mut stored = false;
    for _ in 0..25 {
        sleep(Duration::from_secs(1)).await;
        let rows = notifications::Entity::find()
            .filter(notifications::Column::UserId.eq(user_id))
            .all(db.as_ref())
            .await
            .unwrap();
        if !rows.is_empty() {
            stored = true;
            break;
        }
    }
    assert!(stored, "user_email_verified should persist a notification");

    let messages = sqs
        .wait_for_messages_from_queue(OUTBOUND_QUEUE, 1, 10)
        .await
        .expect("notification_created should be published to the outbound queue");
    let payload: serde_json::Value =
        serde_json::from_str(&messages[0]).expect("SQS message should be JSON");
    assert_eq!(payload["event_type"], "notification_created");
}
