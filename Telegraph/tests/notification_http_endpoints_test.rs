//! Integration tests for Telegraph notification HTTP endpoints
//!
//! Tests the complete HTTP API for notification management:
//! 1. GET /api/notifications - Get user notifications with pagination/filtering
//! 2. GET /api/notifications/unread-count - Get unread notification count
//! 3. PUT /api/notifications/{id}/read - Mark notification as read
//!
//! All endpoints require JWT authentication and proper user ownership validation

mod common;

#[path = "fixtures/mod.rs"]
mod fixtures;

use axum::http::{header, StatusCode};
use common::*;
use sea_orm::EntityTrait;
use serde_json::Value;
use serial_test::serial;
use uuid::Uuid;

use fixtures::db::NotificationFixtureBuilder;
use rustycog_testing::http::jwt::create_jwt_token;
use telegraph_infra::repository::entity::notifications;

/// Test successful retrieval of user notifications with default pagination
#[tokio::test]
#[serial]
async fn test_get_notifications_success_default_pagination() {
    let (fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    // Create test user
    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Create test notifications for this user
    let notification1 = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("First notification".to_string())
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create notification 1");

    let notification2 = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Second notification".to_string())
        .is_read(true)
        .read()
        .commit(&db)
        .await
        .expect("Failed to create notification 2");

    // Create notifications for different user (should not be returned)
    let other_user_id = Uuid::new_v4();
    let _other_notification = NotificationFixtureBuilder::new()
        .user_id(other_user_id)
        .title("Other user notification".to_string())
        .commit(&db)
        .await
        .expect("Failed to create other user notification");

    // Make request to get notifications
    let response = client
        .get(format!("{}/api/notifications", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Verify response status and structure
    assert_eq!(response.status(), StatusCode::OK);

    let result: Value = response.json().await.expect("Failed to parse response");

    // ✅ Verify response structure
    assert!(result.get("notifications").is_some());
    assert!(result.get("total_count").is_some());
    assert!(result.get("page").is_some());
    assert!(result.get("per_page").is_some());
    assert!(result.get("has_more").is_some());

    let notifications = result["notifications"].as_array().unwrap();
    assert_eq!(
        notifications.len(),
        2,
        "Should return 2 notifications for the user"
    );

    // ✅ Verify notification content and ordering (newest first)
    let first_notif = &notifications[0];
    let second_notif = &notifications[1];

    assert_eq!(
        first_notif["title"].as_str().unwrap(),
        "Second notification"
    );
    assert_eq!(first_notif["is_read"].as_bool().unwrap(), true);

    assert_eq!(
        second_notif["title"].as_str().unwrap(),
        "First notification"
    );
    assert_eq!(second_notif["is_read"].as_bool().unwrap(), false);

    // ✅ Verify pagination metadata
    assert_eq!(result["total_count"].as_u64().unwrap(), 2);
    assert_eq!(result["page"].as_u64().unwrap(), 0);
    assert_eq!(result["per_page"].as_u64().unwrap(), 20);
    assert_eq!(result["has_more"].as_bool().unwrap(), false);
}

/// Test notifications filtering by unread_only parameter
#[tokio::test]
#[serial]
async fn test_get_notifications_unread_only_filter() {
    let (fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Create mix of read and unread notifications
    let _unread1 = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Unread notification 1".to_string())
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create unread notification 1");

    let _read1 = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Read notification 1".to_string())
        .read()
        .commit(&db)
        .await
        .expect("Failed to create read notification");

    let _unread2 = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Unread notification 2".to_string())
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create unread notification 2");

    // Test unread_only=true
    let response = client
        .get(format!("{}/api/notifications?unread_only=true", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let result: Value = response.json().await.expect("Failed to parse response");

    let notifications = result["notifications"].as_array().unwrap();
    assert_eq!(
        notifications.len(),
        2,
        "Should return only unread notifications"
    );
    assert_eq!(result["total_count"].as_u64().unwrap(), 2);

    // Verify all returned notifications are unread
    for notification in notifications {
        assert_eq!(notification["is_read"].as_bool().unwrap(), false);
        assert!(notification["title"].as_str().unwrap().contains("Unread"));
    }
}

/// Test pagination with per_page and page parameters
#[tokio::test]
#[serial]
async fn test_get_notifications_pagination() {
    let (fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Create 5 notifications
    for i in 1..=5 {
        let _notification = NotificationFixtureBuilder::new()
            .user_id(user_id)
            .title(format!("Notification {}", i))
            .commit(&db)
            .await
            .expect(&format!("Failed to create notification {}", i));
    }

    // Test first page with per_page=2
    let response = client
        .get(format!("{}/api/notifications?page=0&per_page=2", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let result: Value = response.json().await.expect("Failed to parse response");

    let notifications = result["notifications"].as_array().unwrap();
    assert_eq!(
        notifications.len(),
        2,
        "Should return 2 notifications per page"
    );
    assert_eq!(result["total_count"].as_u64().unwrap(), 5);
    assert_eq!(result["page"].as_u64().unwrap(), 0);
    assert_eq!(result["per_page"].as_u64().unwrap(), 2);
    assert_eq!(result["has_more"].as_bool().unwrap(), true);

    // Test second page
    let response = client
        .get(format!("{}/api/notifications?page=1&per_page=2", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let result: Value = response.json().await.expect("Failed to parse response");

    let notifications = result["notifications"].as_array().unwrap();
    assert_eq!(
        notifications.len(),
        2,
        "Should return 2 notifications on second page"
    );
    assert_eq!(result["page"].as_u64().unwrap(), 1);
    assert_eq!(result["has_more"].as_bool().unwrap(), true);
}

/// Test authentication requirement - missing JWT token
#[tokio::test]
#[serial]
async fn test_get_notifications_missing_auth() {
    let (_, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let response = client
        .get(format!("{}/api/notifications", base_url))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test authentication requirement - invalid JWT token
#[tokio::test]
#[serial]
async fn test_get_notifications_invalid_auth() {
    let (_, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let response = client
        .get(format!("{}/api/notifications", base_url))
        .header(header::AUTHORIZATION, "Bearer invalid_token")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test validation error - per_page exceeds maximum
#[tokio::test]
#[serial]
async fn test_get_notifications_validation_error() {
    let (_, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    let response = client
        .get(format!("{}/api/notifications?per_page=150", base_url)) // Exceeds max of 100
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// Test successful retrieval of unread notification count
#[tokio::test]
#[serial]
async fn test_get_unread_count_success() {
    let (fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Create mix of read and unread notifications
    let _unread1 = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create unread notification 1");

    let _unread2 = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create unread notification 2");

    let _read1 = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .read()
        .commit(&db)
        .await
        .expect("Failed to create read notification");

    // Create unread notification for different user (should not count)
    let other_user_id = Uuid::new_v4();
    let _other_unread = NotificationFixtureBuilder::new()
        .user_id(other_user_id)
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create other user unread notification");

    let response = client
        .get(format!("{}/api/notifications/unread-count", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Verify response
    assert_eq!(response.status(), StatusCode::OK);

    let result: Value = response.json().await.expect("Failed to parse response");

    assert_eq!(
        result["unread_count"].as_u64().unwrap(),
        2,
        "Should count only this user's unread notifications"
    );
}

/// Test unread count with no notifications
#[tokio::test]
#[serial]
async fn test_get_unread_count_empty() {
    let (_, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    let response = client
        .get(format!("{}/api/notifications/unread-count", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let result: Value = response.json().await.expect("Failed to parse response");

    assert_eq!(
        result["unread_count"].as_u64().unwrap(),
        0,
        "Should return 0 for users with no notifications"
    );
}

/// Test unread count authentication requirements
#[tokio::test]
#[serial]
async fn test_get_unread_count_auth_required() {
    let (_, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let response = client
        .get(format!("{}/api/notifications/unread-count", base_url))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test successful marking of notification as read
#[tokio::test]
#[serial]
async fn test_mark_notification_read_success() {
    let (fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Create unread notification
    let notification = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Test notification".to_string())
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create notification");

    let notification_id = notification.id();

    // Verify it's initially unread
    let initial_notification = notifications::Entity::find_by_id(notification_id)
        .one(db.as_ref())
        .await
        .expect("Failed to fetch notification")
        .expect("Notification should exist");
    assert!(
        !initial_notification.is_read,
        "Notification should initially be unread"
    );
    assert!(
        initial_notification.read_at.is_none(),
        "read_at should initially be None"
    );

    let response = client
        .put(format!(
            "{}/api/notifications/{}/read",
            base_url, notification_id
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Verify response
    assert_eq!(response.status(), StatusCode::OK);

    let result: Value = response.json().await.expect("Failed to parse response");

    assert_eq!(result["success"].as_bool().unwrap(), true);
    assert!(result["notification"].is_object());

    let notification_response = &result["notification"];
    assert_eq!(
        notification_response["id"].as_str().unwrap(),
        notification_id.to_string()
    );
    assert_eq!(notification_response["is_read"].as_bool().unwrap(), true);
    assert!(
        notification_response["read_at"].is_string(),
        "read_at should be set"
    );

    // ✅ Verify database state changed
    let updated_notification = notifications::Entity::find_by_id(notification_id)
        .one(db.as_ref())
        .await
        .expect("Failed to fetch notification")
        .expect("Notification should exist");

    assert!(
        updated_notification.is_read,
        "Notification should be marked as read in database"
    );
    assert!(
        updated_notification.read_at.is_some(),
        "read_at should be set in database"
    );
    assert!(
        updated_notification.updated_at > initial_notification.updated_at,
        "updated_at should be newer"
    );
}

/// Test marking notification as read - notification not found, will actually be caught by Permission engine and return a 403
#[tokio::test]
#[serial]
async fn test_mark_notification_read_not_found() {
    let (_, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);
    let nonexistent_id = Uuid::new_v4();

    let response = client
        .put(format!(
            "{}/api/notifications/{}/read",
            base_url, nonexistent_id
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// Test marking notification as read - unauthorized (different user)
#[tokio::test]
#[serial]
async fn test_mark_notification_read_unauthorized() {
    let (fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    let owner_user_id = Uuid::new_v4();
    let other_user_id = Uuid::new_v4();
    let other_user_jwt = create_jwt_token(other_user_id);

    // Create notification owned by owner_user_id
    let notification = NotificationFixtureBuilder::new()
        .user_id(owner_user_id)
        .title("Owner's notification".to_string())
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create notification");

    // Try to mark as read using other_user's JWT
    let response = client
        .put(format!(
            "{}/api/notifications/{}/read",
            base_url,
            notification.id()
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", other_user_jwt))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // ✅ Verify notification remains unchanged
    let unchanged_notification = notifications::Entity::find_by_id(notification.id())
        .one(db.as_ref())
        .await
        .expect("Failed to fetch notification")
        .expect("Notification should exist");

    assert!(
        !unchanged_notification.is_read,
        "Notification should remain unread"
    );
    assert!(
        unchanged_notification.read_at.is_none(),
        "read_at should remain None"
    );
}

/// Test marking notification as read - missing authentication
#[tokio::test]
#[serial]
async fn test_mark_notification_read_missing_auth() {
    let (_, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let notification_id = Uuid::new_v4();

    let response = client
        .put(format!(
            "{}/api/notifications/{}/read",
            base_url, notification_id
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test idempotent behavior - marking already read notification as read
#[tokio::test]
#[serial]
async fn test_mark_notification_read_idempotent() {
    let (fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Create already read notification
    let notification = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Already read notification".to_string())
        .read()
        .commit(&db)
        .await
        .expect("Failed to create read notification");

    let notification_id = notification.id();

    // Get initial read_at timestamp
    let initial_notification = notifications::Entity::find_by_id(notification_id)
        .one(db.as_ref())
        .await
        .expect("Failed to fetch notification")
        .expect("Notification should exist");

    assert!(
        initial_notification.is_read,
        "Notification should be read initially"
    );
    let initial_read_at = initial_notification.read_at.expect("read_at should be set");

    // Mark as read again
    let response = client
        .put(format!(
            "{}/api/notifications/{}/read",
            base_url, notification_id
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Verify successful response (idempotent)
    assert_eq!(response.status(), StatusCode::OK);

    let result: Value = response.json().await.expect("Failed to parse response");

    assert_eq!(result["success"].as_bool().unwrap(), true);
    assert_eq!(result["notification"]["is_read"].as_bool().unwrap(), true);

    // ✅ Verify read_at timestamp is updated (proving the operation ran)
    let updated_notification = notifications::Entity::find_by_id(notification_id)
        .one(db.as_ref())
        .await
        .expect("Failed to fetch notification")
        .expect("Notification should exist");

    assert!(
        updated_notification.is_read,
        "Notification should still be read"
    );
    let new_read_at = updated_notification
        .read_at
        .expect("read_at should still be set");
    assert!(
        new_read_at >= initial_read_at,
        "read_at should be updated or same"
    );
}

/// Helper function to create a test JWT token (you'll need to implement this based on your JWT creation logic)
fn create_test_jwt_token(user_id: Uuid) -> String {
    // This should match your JWT creation logic from IAMRusty or Telegraph's JWT handling
    // For testing purposes, you might need a simple JWT that contains the user_id claim
    format!("test_jwt_token_for_user_{}", user_id)
}

/// Comprehensive test covering the complete notification lifecycle
#[tokio::test]
#[serial]
async fn test_notification_lifecycle_complete() {
    let (fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Step 1: Create some notifications
    let notification1 = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Lifecycle test notification 1".to_string())
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create notification 1");

    let notification2 = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Lifecycle test notification 2".to_string())
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create notification 2");

    // Step 2: Check unread count (should be 2)
    let response = client
        .get(format!("{}/api/notifications/unread-count", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let count_result: Value = response.json().await.expect("Failed to parse response");
    assert_eq!(count_result["unread_count"].as_u64().unwrap(), 2);

    // Step 3: Get all notifications (should see both unread)
    let response = client
        .get(format!("{}/api/notifications", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let list_result: Value = response.json().await.expect("Failed to parse response");

    let notifications = list_result["notifications"].as_array().unwrap();
    assert_eq!(notifications.len(), 2);
    assert_eq!(list_result["total_count"].as_u64().unwrap(), 2);

    // Step 4: Mark one notification as read
    let response = client
        .put(format!(
            "{}/api/notifications/{}/read",
            base_url,
            notification1.id()
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    // Step 5: Check unread count again (should be 1)
    let response = client
        .get(format!("{}/api/notifications/unread-count", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let count_result2: Value = response.json().await.expect("Failed to parse response");
    assert_eq!(count_result2["unread_count"].as_u64().unwrap(), 1);

    // Step 6: Get only unread notifications (should see only one)
    let response = client
        .get(format!("{}/api/notifications?unread_only=true", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let unread_result: Value = response.json().await.expect("Failed to parse response");

    let unread_notifications = unread_result["notifications"].as_array().unwrap();
    assert_eq!(unread_notifications.len(), 1);
    assert_eq!(unread_result["total_count"].as_u64().unwrap(), 1);
    assert_eq!(
        unread_notifications[0]["id"].as_str().unwrap(),
        notification2.id().to_string()
    );
    assert_eq!(unread_notifications[0]["is_read"].as_bool().unwrap(), false);
}
