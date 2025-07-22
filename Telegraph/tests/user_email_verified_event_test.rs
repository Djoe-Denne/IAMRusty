//! Integration tests for user email verified event processing in Telegraph
//! 
//! Tests the complete flow for user_email_verified event processing where:
//! 1. Telegraph receives a UserEmailVerified event
//! 2. Telegraph processes the event and creates a database notification
//! 3. We verify the notification was stored correctly in the database

mod common;

use common::*;
use iam_events::{IamDomainEvent, UserEmailVerifiedEvent};
use rustycog_events::event::BaseEvent;
use serial_test::serial;
use uuid::Uuid;
use sea_orm::{ConnectionTrait, EntityTrait};
use telegraph_infra::repository::entity::notifications;

/// Test that Telegraph correctly processes UserEmailVerified events and creates database notifications
#[tokio::test]
#[serial]
async fn test_user_email_verified_event_creates_database_notification() {
    // Setup test infrastructure with real producer/consumer
    let (fixture, _, _) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
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

    // Publish the event using the test event publisher (routes directly to processor)
    let result = test_event_publisher.send_event(&iam_event).await;
    
    // ✅ Verify event publishing was successful
    assert!(
        result.is_ok(), 
        "Event publishing should succeed, but got error: {:?}", 
        result.err()
    );
    
    // ✅ Verify that a notification was created in the database
    let notifications_after = db
        .query_all(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT * FROM notifications WHERE user_id = '{}'",
                user_id
            ),
        ))
        .await
        .expect("Failed to get notifications after event processing");
    
    assert!(
        notifications_after.len() > 0,
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
    assert_eq!(new_notification.try_get::<Uuid>("", "user_id").unwrap(), user_id, "Notification should be for the correct user");
    assert_eq!(new_notification.try_get::<String>("", "title").unwrap(), "Email Verified Successfully", "Notification should have correct title");
    assert!(!new_notification.try_get::<bool>("", "is_read").unwrap(), "Notification should be unread initially");
    assert_eq!(new_notification.try_get::<i16>("", "priority").unwrap(), 2, "Email verification should have medium priority");
    assert_eq!(new_notification.try_get::<String>("", "content_type").unwrap(), "application/json", "Content should be JSON");
    
    // ✅ Verify notification content JSON
    let content_json: String = new_notification.try_get::<String>("", "content").unwrap();
    let content: serde_json::Value = serde_json::from_str(&content_json).unwrap();
    
    assert_eq!(content["event_type"], "user_email_verified", "Event type should be correct");
    assert_eq!(content["user_id"], user_id.to_string(), "User ID should match");
    assert_eq!(content["email"], test_email, "Email should match");
    assert_eq!(content["action"], "email_verification_completed", "Action should be correct");
    assert!(content["message"].as_str().unwrap().contains("successfully verified"), "Message should indicate successful verification");

    println!("✅ Database notification verified - correct content and metadata");
    
    // ✅ Verify that a delivery record was created
    let deliveries = db
        .query_all(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!(
                "SELECT * FROM notification_deliveries WHERE notification_id = '{}'",
                new_notification.try_get::<i32>("", "id").unwrap()
            ),
        ))
        .await
        .expect("Failed to get delivery records");
    
    assert!(!deliveries.is_empty(), "At least one delivery record should exist");
    
    let delivery = &deliveries[0];
    assert_eq!(delivery.try_get::<i32>("", "notification_id").unwrap(), new_notification.try_get::<i32>("", "id").unwrap(), "Delivery should be linked to notification");
    assert_eq!(delivery.try_get::<String>("", "delivery_method").unwrap(), "in_app", "Delivery method should be in_app");
    assert_eq!(delivery.try_get::<String>("", "status").unwrap(), "pending", "Delivery should be pending initially");
    assert_eq!(delivery.try_get::<i32>("", "attempt_count").unwrap(), 0, "No delivery attempts should have been made yet");

}

/// Test that multiple UserEmailVerified events create separate notifications
#[tokio::test]
#[serial]
async fn test_multiple_email_verified_events_create_separate_notifications() {
    // Setup test infrastructure
    let (fixture, _, _) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
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
        assert!(result.is_ok(), "Event publishing should succeed for {}", email);
        
    }

    // ✅ Verify that 3 separate notifications were created
    let notifications = db
    .query_all(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!(
            "SELECT * FROM notifications WHERE user_id = '{}'",
            user_id
        ),
    ))
    .await
    .expect("Failed to get notifications");

    assert_eq!(notifications.len(), 3, "Should have created 3 separate notifications");
    
    // ✅ Verify all notifications have correct content
    for notification in &notifications {
        assert_eq!(notification.try_get::<Uuid>("", "user_id").unwrap(), user_id, "All notifications should be for same user");
        assert_eq!(notification.try_get::<String>("", "title").unwrap(), "Email Verified Successfully", "All should have same title");
        assert_eq!(notification.try_get::<i16>("", "priority").unwrap(), 2, "All should have medium priority");
        
        let content_json: String = notification.try_get::<String>("", "content").unwrap();
        let content: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        
        assert_eq!(content["event_type"], "user_email_verified", "Event type should match");
        assert_eq!(content["action"], "email_verification_completed", "Action should match");
    }

}

/// Test that UserEmailVerified events for different users create separate notifications
#[tokio::test]
#[serial]
async fn test_different_users_email_verified_events() {
    // Setup test infrastructure
    let (fixture, _, _) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

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
        assert!(result.is_ok(), "Event publishing should succeed for user {}", user_id);
    }

    // ✅ Verify each user has exactly one notification
    for (user_id, email) in &users {
        let user_notifications = db
            .query_all(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!(
                    "SELECT * FROM notifications WHERE user_id = '{}'",
                    user_id
                ),
            ))
            .await
            .expect("Failed to get user notifications");

        assert_eq!(user_notifications.len(), 1, "User {} should have exactly 1 notification", user_id);

        let notification = &user_notifications[0];
        assert_eq!(notification.try_get::<Uuid>("", "user_id").unwrap(), *user_id, "Notification should belong to correct user");
        
        let content_json: String = notification.try_get::<String>("", "content").unwrap();
        let content: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        
        assert_eq!(content["email"], *email, "Email in notification should match event email");
        
    }
} 