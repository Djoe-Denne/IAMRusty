//! Integration tests for user email verified event processing in Telegraph
//!
//! Tests the complete flow for `user_email_verified` event processing where:
//! 1. Telegraph receives a `UserEmailVerified` event
//! 2. Telegraph processes the event and creates a database notification
//! 3. We verify the notification was stored correctly in the database

mod common;

use common::fixtures::sqs_scenario::SqsEventScenario;
use iam_events::{IamDomainEvent, UserEmailVerifiedEvent};
use rustycog::events::event::BaseEvent;
use sea_orm::ConnectionTrait;
use serial_test::serial;
use uuid::Uuid;

/// Test that Telegraph correctly processes `UserEmailVerified` events and creates database notifications
#[tokio::test]
#[serial]
async fn test_user_email_verified_event_creates_database_notification() {
    // Setup test infrastructure with real producer/consumer
    let scenario = SqsEventScenario::start()
        .await
        .expect("Failed to start Telegraph SQS scenario");
    let fixture = &scenario.fixture;
    let db = fixture.db();
    let test_event_publisher = fixture.sqs();

    // Create a test UserEmailVerified event
    let user_id = Uuid::new_v4();
    let test_email = "verified.user@example.com";

    let user_email_verified_event = UserEmailVerifiedEvent {
        base: BaseEvent::new("user_email_verified".to_string(), user_id),
        user_id,
        email: test_email.to_string(),
    };

    let iam_event = IamDomainEvent::UserEmailVerified(user_email_verified_event);
    let event_id = rustycog::events::DomainEvent::event_id(&iam_event);

    // Publish the event using the test event publisher (routes directly to processor)
    let result = test_event_publisher.send_event(&iam_event).await;

    // ✅ Verify event publishing was successful
    assert!(
        result.is_ok(),
        "Event publishing should succeed, but got error: {:?}",
        result.err()
    );

    let phase_started = tokio::time::Instant::now();
    let mut notifications_after = vec![];
    // Wait for the event to be processed and notification to be created
    for i in 0..5 {
        if i > 0 {
            println!("Waiting for event to be processed and notification to be created: {i}");
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        notifications_after = db
            .query_all(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!("SELECT * FROM notifications WHERE user_id = '{user_id}'"),
            ))
            .await
            .expect("Failed to get notifications after event processing");
        if !notifications_after.is_empty() {
            break;
        }
    }

    if notifications_after.is_empty() {
        let snapshot = common::fixtures::sqs_failure_probe::failure_snapshot(
            &test_event_publisher.client,
            &test_event_publisher.queue_url,
            common::fixtures::sqs_failure_probe::Stage::Notification,
            event_id,
            result.as_deref().ok(),
            phase_started.elapsed(),
        )
        .await;
        eprintln!("TELEGRAPH_SQS_FAILURE {snapshot}");
    }
    assert!(
        !notifications_after.is_empty(),
        "A new notification should have been created. Before: {}, After: {}",
        0,
        notifications_after.len()
    );

    // Find the notification created for our event
    let new_notification = notifications_after
        .iter()
        .find(|notif| notif.try_get::<Uuid>("", "user_id").unwrap() == user_id)
        .expect("Should find a notification for the test user");

    // ✅ Verify notification content and metadata
    assert_eq!(
        new_notification.try_get::<Uuid>("", "user_id").unwrap(),
        user_id,
        "Notification should be for the correct user"
    );
    assert_eq!(
        new_notification.try_get::<String>("", "title").unwrap(),
        "Email Verified Successfully",
        "Notification should have correct title"
    );
    assert!(
        !new_notification.try_get::<bool>("", "is_read").unwrap(),
        "Notification should be unread initially"
    );
    assert_eq!(
        new_notification.try_get::<i16>("", "priority").unwrap(),
        1,
        "Email verification should have high priority"
    );
    assert_eq!(
        new_notification
            .try_get::<String>("", "content_type")
            .unwrap(),
        "application/text",
        "Content should be text"
    );

    // ✅ Verify notification content JSON
    let content_bytes: Vec<u8> = new_notification.try_get::<Vec<u8>>("", "content").unwrap();
    let content = String::from_utf8(content_bytes).unwrap();
    assert_eq!(content, "Email Verified Successfully\n\nYour email address verified.user@example.com has been successfully verified.\n\nYou can now access all features of your Telegraph account.\n\nThank you!");

    // ✅ Verify that a delivery record was created
    let deliveries = db
        .query_all(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT * FROM notification_deliveries WHERE notification_id = '{}'",
                new_notification.try_get::<Uuid>("", "id").unwrap()
            ),
        ))
        .await
        .expect("Failed to get delivery records");

    assert!(
        !deliveries.is_empty(),
        "At least one delivery record should exist"
    );

    let delivery = &deliveries[0];
    assert_eq!(
        delivery.try_get::<Uuid>("", "notification_id").unwrap(),
        new_notification.try_get::<Uuid>("", "id").unwrap(),
        "Delivery should be linked to notification"
    );
    assert_eq!(
        delivery.try_get::<String>("", "delivery_method").unwrap(),
        "notification",
        "Delivery method should be notification"
    );
    assert_eq!(
        delivery.try_get::<String>("", "status").unwrap(),
        "pending",
        "Delivery should be pending initially"
    );
    assert_eq!(
        delivery.try_get::<i16>("", "attempt_count").unwrap(),
        0,
        "No delivery attempts should have been made yet"
    );
    scenario.stop().await.unwrap();
}

/// Test that multiple `UserEmailVerified` events create separate notifications
#[tokio::test]
#[serial]
async fn test_multiple_email_verified_events_create_separate_notifications() {
    // Setup test infrastructure
    let scenario = SqsEventScenario::start()
        .await
        .expect("Failed to start Telegraph SQS scenario");
    let fixture = &scenario.fixture;
    let db = fixture.db();
    let test_event_publisher = fixture.sqs();
    let user_id = Uuid::new_v4();

    // Create multiple email verified events
    let events = vec![
        ("first.email@example.com", "user_email_verified"),
        ("second.email@example.com", "user_email_verified"),
        ("third.email@example.com", "user_email_verified"),
    ];

    for (email, event_type) in events {
        let event = UserEmailVerifiedEvent {
            base: BaseEvent::new(event_type.to_string(), user_id),
            user_id,
            email: email.to_string(),
        };

        let iam_event = IamDomainEvent::UserEmailVerified(event);

        let result = test_event_publisher.send_event(&iam_event).await;
        assert!(
            result.is_ok(),
            "Event publishing should succeed for {email}"
        );
    }
    let mut notifications = vec![];
    for i in 0..5 {
        if i > 0 {
            println!("Waiting for event to be processed and notification to be created: {i}");
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        notifications = db
            .query_all(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!("SELECT * FROM notifications WHERE user_id = '{user_id}'"),
            ))
            .await
            .expect("Failed to get notifications after event processing");
        if notifications.len() == 3 {
            break;
        }
    }

    // ✅ Verify that 3 separate notifications were created
    assert_eq!(
        notifications.len(),
        3,
        "Should have created 3 separate notifications"
    );

    // ✅ Verify all notifications have correct content
    for notification in &notifications {
        assert_eq!(
            notification.try_get::<Uuid>("", "user_id").unwrap(),
            user_id,
            "All notifications should be for same user"
        );
        assert_eq!(
            notification.try_get::<String>("", "title").unwrap(),
            "Email Verified Successfully",
            "All should have same title"
        );
        assert_eq!(
            notification.try_get::<i16>("", "priority").unwrap(),
            1,
            "All should have high priority"
        );
        let content_bytes: Vec<u8> = notification.try_get::<Vec<u8>>("", "content").unwrap();
        let content = String::from_utf8(content_bytes).unwrap();
        assert!(
            content.contains("Email Verified Successfully\n\nYour email address"),
            "Content should contain 'Email Verified Successfully\n\nYour email address'"
        );
        assert!(
            content.contains("has been successfully verified."),
            "Content should contain 'has been successfully verified.'"
        );
        assert!(
            content.contains("You can now access all features of your Telegraph account."),
            "Content should contain 'You can now access all features of your Telegraph account.'"
        );
        assert!(
            content.contains("Thank you!"),
            "Content should contain 'Thank you!'"
        );
    }
    scenario.stop().await.unwrap();
}

/// Test that `UserEmailVerified` events for different users create separate notifications
#[tokio::test]
#[serial]
async fn test_different_users_email_verified_events() {
    // Setup test infrastructure
    let scenario = SqsEventScenario::start()
        .await
        .expect("Failed to start Telegraph SQS scenario");

    let fixture = &scenario.fixture;
    let db = fixture.db();
    let test_event_publisher = fixture.sqs();
    // Create events for different users
    let users = vec![
        (Uuid::new_v4(), "alice@example.com"),
        (Uuid::new_v4(), "bob@example.com"),
        (Uuid::new_v4(), "charlie@example.com"),
    ];

    for (user_id, email) in &users {
        let event = UserEmailVerifiedEvent {
            base: BaseEvent::new("user_email_verified".to_string(), *user_id),
            user_id: *user_id,
            email: email.to_string(),
        };

        let iam_event = IamDomainEvent::UserEmailVerified(event);

        let result = test_event_publisher.send_event(&iam_event).await;
        assert!(
            result.is_ok(),
            "Event publishing should succeed for user {user_id}"
        );
    }

    for i in 0..5 {
        if i > 0 {
            println!("Waiting for event to be processed and notification to be created: {i}");
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let notifications = db
            .query_all(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT * FROM notifications",
            ))
            .await
            .expect("Failed to get notifications after event processing");
        if notifications.len() == 3 {
            break;
        }
    }

    // ✅ Verify each user has exactly one notification
    for (user_id, email) in &users {
        let user_notifications = db
            .query_all(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!("SELECT * FROM notifications WHERE user_id = '{user_id}'"),
            ))
            .await
            .expect("Failed to get user notifications");

        assert_eq!(
            user_notifications.len(),
            1,
            "User {user_id} should have exactly 1 notification"
        );

        let notification = &user_notifications[0];
        assert_eq!(
            notification.try_get::<Uuid>("", "user_id").unwrap(),
            *user_id,
            "Notification should belong to correct user"
        );

        let content_bytes: Vec<u8> = notification.try_get::<Vec<u8>>("", "content").unwrap();
        let content = String::from_utf8(content_bytes).unwrap();
        assert_eq!(content, format!("Email Verified Successfully\n\nYour email address {email} has been successfully verified.\n\nYou can now access all features of your Telegraph account.\n\nThank you!"));
    }
    scenario.stop().await.unwrap();
}
