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
use sea_orm::EntityTrait;
use telegraph_infra::repository::entity::notifications;

/// Test that Telegraph correctly processes UserEmailVerified events and creates database notifications
#[tokio::test]
#[serial]
async fn test_user_email_verified_event_creates_database_notification() {
    // Setup test infrastructure with real producer/consumer
    let (fixture, test_event_publisher) = setup_telegraph_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    // Create a test UserEmailVerified event
    let user_id = Uuid::new_v4();
    let test_email = "verified.user@example.com";
    
    let user_email_verified_event = UserEmailVerifiedEvent {
        base: BaseEvent::new("user_email_verified".to_string(), user_id),
        user_id,
        email: test_email.to_string(),
    };

    let iam_event = IamDomainEvent::UserEmailVerified(user_email_verified_event);

    println!("📧 Testing user email verified event processing...");
    println!("   User ID: {}", user_id);
    println!("   Email: {}", test_email);

    // Get the notification repository for verification
    let notification_repo = fixture.notification_repo();
    
    // Get initial notification count for this user
    let (initial_notifications, _) = notification_repo
        .get_user_notifications(user_id, 0, 10, false)
        .await
        .expect("Failed to get initial notifications");
    
    let initial_count = initial_notifications.len();
    println!("🔍 Debug: Initial notification count for user: {}", initial_count);

    // Publish the event using the test event publisher (routes directly to processor)
    println!("🔍 Debug: Publishing event...");
    let result = test_event_publisher.publish(Box::new(iam_event.clone())).await;
    
    // ✅ Verify event publishing was successful
    assert!(
        result.is_ok(), 
        "Event publishing should succeed, but got error: {:?}", 
        result.err()
    );

    println!("✅ Event published and processed successfully through real infrastructure");
    
    // ✅ Verify that a notification was created in the database
    let (notifications_after, total_count) = notification_repo
        .get_user_notifications(user_id, 0, 10, false)
        .await
        .expect("Failed to get notifications after event processing");
    
    println!("🔍 Debug: Notification count after processing: {}", notifications_after.len());
    println!("🔍 Debug: Total notification count: {}", total_count);
    
    assert!(
        notifications_after.len() > initial_count,
        "A new notification should have been created. Before: {}, After: {}",
        initial_count,
        notifications_after.len()
    );
    
    // Find the notification created for our event
    let new_notification = notifications_after
        .iter()
        .find(|notif| notif.user_id == user_id)
        .expect("Should find a notification for the test user");
    
    // ✅ Verify notification content and metadata
    assert_eq!(new_notification.user_id, user_id, "Notification should be for the correct user");
    assert_eq!(new_notification.title, "Email Verified Successfully", "Notification should have correct title");
    assert!(!new_notification.is_read, "Notification should be unread initially");
    assert_eq!(new_notification.priority, 2, "Email verification should have medium priority");
    assert_eq!(new_notification.content_type, "application/json", "Content should be JSON");
    
    // ✅ Verify notification content JSON
    let content_json = new_notification.content_as_json()
        .expect("Should be able to parse content as JSON")
        .expect("Content should be valid JSON");
    
    let content: serde_json::Value = serde_json::from_str(&content_json)
        .expect("Should be able to parse JSON content");
    
    assert_eq!(content["event_type"], "user_email_verified", "Event type should be correct");
    assert_eq!(content["user_id"], user_id.to_string(), "User ID should match");
    assert_eq!(content["email"], test_email, "Email should match");
    assert_eq!(content["action"], "email_verification_completed", "Action should be correct");
    assert!(content["message"].as_str().unwrap().contains("successfully verified"), "Message should indicate successful verification");

    println!("✅ Database notification verified - correct content and metadata");
    
    // ✅ Verify that a delivery record was created
    let deliveries = notification_repo
        .get_notification_deliveries(new_notification.id)
        .await
        .expect("Failed to get delivery records");
    
    assert!(!deliveries.is_empty(), "At least one delivery record should exist");
    
    let delivery = &deliveries[0];
    assert_eq!(delivery.notification_id, new_notification.id, "Delivery should be linked to notification");
    assert_eq!(delivery.delivery_method, "in_app", "Delivery method should be in_app");
    assert_eq!(delivery.status, "pending", "Delivery should be pending initially");
    assert_eq!(delivery.attempt_count, 0, "No delivery attempts should have been made yet");

    println!("✅ Delivery record verified - created with correct metadata");
    println!("✅ Integration test completed: event published → processed → notification stored → delivery tracked");
}

/// Test that multiple UserEmailVerified events create separate notifications
#[tokio::test]
#[serial]
async fn test_multiple_email_verified_events_create_separate_notifications() {
    // Setup test infrastructure
    let (fixture, test_event_publisher) = setup_telegraph_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let user_id = Uuid::new_v4();
    let notification_repo = fixture.notification_repo();

    // Create multiple email verified events
    let events = vec![
        ("first.email@example.com", "user_email_verified"),
        ("second.email@example.com", "user_email_verified"),
        ("third.email@example.com", "user_email_verified"),
    ];

    println!("🔍 Testing multiple email verified events...");

    for (email, event_type) in events {
        let event = UserEmailVerifiedEvent {
            base: BaseEvent::new(event_type.to_string(), user_id),
            user_id,
            email: email.to_string(),
        };

        let iam_event = IamDomainEvent::UserEmailVerified(event);
        
        let result = test_event_publisher.publish(Box::new(iam_event)).await;
        assert!(result.is_ok(), "Event publishing should succeed for {}", email);
        
        println!("✅ Processed event for email: {}", email);
    }

    // ✅ Verify that 3 separate notifications were created
    let (notifications, total_count) = notification_repo
        .get_user_notifications(user_id, 0, 10, false)
        .await
        .expect("Failed to get notifications");

    assert_eq!(notifications.len(), 3, "Should have created 3 separate notifications");
    assert_eq!(total_count, 3, "Total count should be 3");

    // ✅ Verify all notifications have correct content
    for notification in &notifications {
        assert_eq!(notification.user_id, user_id, "All notifications should be for same user");
        assert_eq!(notification.title, "Email Verified Successfully", "All should have same title");
        assert_eq!(notification.priority, 2, "All should have medium priority");
        
        let content_json = notification.content_as_json()
            .expect("Should parse JSON")
            .expect("Should have JSON content");
        let content: serde_json::Value = serde_json::from_str(&content_json)
            .expect("Should parse content");
        
        assert_eq!(content["event_type"], "user_email_verified", "Event type should match");
        assert_eq!(content["action"], "email_verification_completed", "Action should match");
    }

    println!("✅ Multiple events test completed: 3 events → 3 separate notifications");
}

/// Test that UserEmailVerified events for different users create separate notifications
#[tokio::test]
#[serial]
async fn test_different_users_email_verified_events() {
    // Setup test infrastructure
    let (fixture, test_event_publisher) = setup_telegraph_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let notification_repo = fixture.notification_repo();

    // Create events for different users
    let users = vec![
        (Uuid::new_v4(), "alice@example.com"),
        (Uuid::new_v4(), "bob@example.com"),
        (Uuid::new_v4(), "charlie@example.com"),
    ];

    println!("🔍 Testing email verified events for different users...");

    for (user_id, email) in &users {
        let event = UserEmailVerifiedEvent {
            base: BaseEvent::new("user_email_verified".to_string(), *user_id),
            user_id: *user_id,
            email: email.to_string(),
        };

        let iam_event = IamDomainEvent::UserEmailVerified(event);
        
        let result = test_event_publisher.publish(Box::new(iam_event)).await;
        assert!(result.is_ok(), "Event publishing should succeed for user {}", user_id);
        
        println!("✅ Processed event for user: {} ({})", user_id, email);
    }

    // ✅ Verify each user has exactly one notification
    for (user_id, email) in &users {
        let (user_notifications, count) = notification_repo
            .get_user_notifications(*user_id, 0, 10, false)
            .await
            .expect("Failed to get user notifications");

        assert_eq!(user_notifications.len(), 1, "User {} should have exactly 1 notification", user_id);
        assert_eq!(count, 1, "User {} total count should be 1", user_id);

        let notification = &user_notifications[0];
        assert_eq!(notification.user_id, *user_id, "Notification should belong to correct user");
        
        let content_json = notification.content_as_json()
            .expect("Should parse JSON")
            .expect("Should have JSON content");
        let content: serde_json::Value = serde_json::from_str(&content_json)
            .expect("Should parse content");
        
        assert_eq!(content["email"], *email, "Email in notification should match event email");
        
        println!("✅ Verified notification for user: {} ({})", user_id, email);
    }

    println!("✅ Different users test completed: 3 users → 3 separate user notifications");
} 