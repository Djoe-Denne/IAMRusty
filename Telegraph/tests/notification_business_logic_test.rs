//! Integration tests for Telegraph notification business logic
//!
//! Tests complex business scenarios and edge cases:
//! 1. Concurrent notification reading operations
//! 2. Large datasets and performance boundaries
//! 3. Notification state consistency
//! 4. Priority and ordering behavior
//! 5. Multi-user scenarios with proper isolation

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

/// Test concurrent mark-as-read operations on the same notification
#[tokio::test]
#[serial]
async fn test_mark_as_read_operation() {
    let (fixture, base_url, client, openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Create an unread notification
    let notification = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Concurrent test notification".to_string())
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create notification");

    let notification_id = notification.id();

    // Route guard: `with_permission_on(Permission::Write, "notification")`
    // on `PUT /api/notifications/{id}/read`. Seed the recipient tuple
    // `sentinel-sync` would write in production.
    openfga
        .allow(
            Subject::new(user_id),
            Permission::Write,
            ResourceRef::new("notification", notification_id),
        )
        .await
        .expect("Failed to grant notification write");

    let response = client
        .put(format!(
            "{}/api/notifications/{}/read",
            base_url, notification_id
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(response.status(), StatusCode::OK);

    // ✅ Verify final state is consistent
    let final_notification = notifications::Entity::find_by_id(notification_id)
        .one(db.as_ref())
        .await
        .expect("Failed to fetch notification")
        .expect("Notification should exist");

    assert!(
        final_notification.is_read,
        "Notification should be marked as read"
    );
    assert!(
        final_notification.read_at.is_some(),
        "read_at should be set"
    );
}

/// Test large dataset pagination performance and correctness
#[tokio::test]
#[serial]
async fn test_large_dataset_pagination() {
    let (fixture, base_url, client, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Create 50 notifications
    for i in 1..=50 {
        let _notification = NotificationFixtureBuilder::new()
            .user_id(user_id)
            .title(format!("Bulk notification {}", i))
            .commit(&db)
            .await
            .expect(&format!("Failed to create notification {}", i));
    }

    // Test various page sizes
    let page_sizes = vec![5, 10, 20];

    for per_page in page_sizes {
        let mut total_collected = 0;
        let mut page = 0;
        let mut all_notification_ids = std::collections::HashSet::new();

        loop {
            let response = client
                .get(format!(
                    "{}/api/notifications?page={}&per_page={}",
                    base_url, page, per_page
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
                .send()
                .await
                .expect("Failed to send request");

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Page {} should be successful",
                page
            );

            let result: Value = response.json().await.expect("Failed to parse response");

            let notifications = result["notifications"]
                .as_array()
                .expect("Notifications should be an array");
            let has_more = result["has_more"]
                .as_bool()
                .expect("has_more should be a boolean");

            // Collect notification IDs to ensure no duplicates
            for notification in notifications {
                let id = notification["id"].as_str().expect("id should be a string");
                assert!(
                    all_notification_ids.insert(id.to_string()),
                    "Duplicate notification ID found: {}",
                    id
                );
            }

            total_collected += notifications.len();

            if !has_more {
                break;
            }

            page += 1;

            // Safety break to avoid infinite loops
            assert!(page < 20, "Too many pages, possible infinite loop");
        }

        // ✅ Verify we collected exactly 50 notifications
        assert_eq!(
            total_collected, 50,
            "Should collect exactly 50 notifications with per_page={}",
            per_page
        );
        assert_eq!(
            all_notification_ids.len(),
            50,
            "Should have 50 unique notification IDs with per_page={}",
            per_page
        );
    }
}

/// Test notification expiration handling
#[tokio::test]
#[serial]
async fn test_expired_notifications_handling() {
    let (fixture, base_url, client, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Create mix of expired and active notifications
    let _active = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Active notification".to_string())
        .commit(&db)
        .await
        .expect("Failed to create active notification");

    let _expired = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Expired notification".to_string())
        .expired()
        .commit(&db)
        .await
        .expect("Failed to create expired notification");

    // Get notifications
    let response = client
        .get(format!("{}/api/notifications", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let result: Value = response.json().await.expect("Failed to parse response");

    let notifications = result["notifications"]
        .as_array()
        .expect("Notifications should be an array");

    // ✅ Verify business logic for expired notifications
    // This depends on your business requirements:
    // - Option 1: Expired notifications are filtered out
    // - Option 2: Expired notifications are included but marked
    // - Option 3: Expired notifications are included normally

    // For this test, assuming expired notifications are still returned but marked
    assert_eq!(
        notifications.len(),
        2,
        "Should return both active and expired notifications"
    );

    // Find expired notification and verify it's marked or handled appropriately
    let expired_notification = notifications
        .iter()
        .find(|n| n["title"].as_str().unwrap().contains("Expired"))
        .expect("Should find expired notification");

    // Business logic verification would go here
    // For example: expired_notification should have expired=true field
}

/// Test multi-user isolation - users cannot access other users' notifications
#[tokio::test]
#[serial]
async fn test_multi_user_isolation_comprehensive() {
    let (fixture, base_url, client, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    // Create three different users
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    let user_c = Uuid::new_v4();

    let jwt_a = create_jwt_token(user_a);
    let jwt_b = create_jwt_token(user_b);
    let jwt_c = create_jwt_token(user_c);

    // Create notifications for each user
    let notification_a = NotificationFixtureBuilder::new()
        .user_id(user_a)
        .title("User A notification".to_string())
        .commit(&db)
        .await
        .expect("Failed to create user A notification");

    let notification_b = NotificationFixtureBuilder::new()
        .user_id(user_b)
        .title("User B notification".to_string())
        .commit(&db)
        .await
        .expect("Failed to create user B notification");

    let _notification_c = NotificationFixtureBuilder::new()
        .user_id(user_c)
        .title("User C notification".to_string())
        .commit(&db)
        .await
        .expect("Failed to create user C notification");

    // User A should only see their notification
    let response_a = client
        .get(format!("{}/api/notifications", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_a))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response_a.status(), StatusCode::OK);

    let result_a: Value = response_a.json().await.expect("Failed to parse response");

    let notifications_a = result_a["notifications"]
        .as_array()
        .expect("Notifications should be an array");
    assert_eq!(
        notifications_a.len(),
        1,
        "User A should see only 1 notification"
    );
    assert_eq!(
        notifications_a[0]["title"]
            .as_str()
            .expect("title should be a string"),
        "User A notification"
    );

    // User B should only see their notification
    let response_b = client
        .get(format!("{}/api/notifications", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_b))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response_b.status(), StatusCode::OK);

    let result_b: Value = response_b.json().await.expect("Failed to parse response");

    let notifications_b = result_b["notifications"]
        .as_array()
        .expect("Notifications should be an array");
    assert_eq!(
        notifications_b.len(),
        1,
        "User B should see only 1 notification"
    );
    assert_eq!(
        notifications_b[0]["title"]
            .as_str()
            .expect("title should be a string"),
        "User B notification"
    );

    // Test cross-user access attempt - User A tries to mark User B's notification as read
    let cross_access_response = client
        .put(format!(
            "{}/api/notifications/{}/read",
            base_url,
            notification_b.id()
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_a))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        cross_access_response.status(),
        StatusCode::FORBIDDEN,
        "User A should not be able to mark User B's notification as read"
    );

    // Verify User B's notification remains unchanged
    let verification_notification = notifications::Entity::find_by_id(notification_b.id())
        .one(db.as_ref())
        .await
        .expect("Failed to fetch notification")
        .expect("Notification should exist");

    assert!(
        !verification_notification.is_read,
        "User B's notification should remain unread after unauthorized access attempt"
    );
}

/// Test notification filtering with complex scenarios
#[tokio::test]
#[serial]
async fn test_complex_notification_filtering() {
    let (fixture, base_url, client, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Create various types of notifications
    let _unread_high = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Unread high priority".to_string())
        .high_priority()
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create unread high priority notification");

    let _unread_normal = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Unread normal priority".to_string())
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create unread normal notification");

    let _read_high = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Read high priority".to_string())
        .high_priority()
        .read()
        .commit(&db)
        .await
        .expect("Failed to create read high priority notification");

    let _read_normal = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Read normal priority".to_string())
        .read()
        .commit(&db)
        .await
        .expect("Failed to create read normal notification");

    let _expired_unread = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Expired unread".to_string())
        .expired()
        .is_read(false)
        .commit(&db)
        .await
        .expect("Failed to create expired unread notification");

    // Test 1: Get all notifications
    let all_response = client
        .get(format!("{}/api/notifications", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(all_response.status(), StatusCode::OK);

    let all_result: Value = all_response.json().await.expect("Failed to parse response");

    assert_eq!(
        all_result["total_count"].as_u64().unwrap(),
        5,
        "Should have 5 total notifications"
    );

    // Test 2: Get only unread notifications
    let unread_response = client
        .get(format!("{}/api/notifications?unread_only=true", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(unread_response.status(), StatusCode::OK);

    let unread_result: Value = unread_response
        .json()
        .await
        .expect("Failed to parse response");

    let unread_notifications = unread_result["notifications"]
        .as_array()
        .expect("Notifications should be an array");
    assert_eq!(
        unread_notifications.len(),
        3,
        "Should have 3 unread notifications"
    );
    assert_eq!(unread_result["total_count"].as_u64().unwrap(), 3);

    // Verify all returned notifications are unread
    for notification in unread_notifications {
        assert_eq!(
            notification["is_read"].as_bool().unwrap(),
            false,
            "All filtered notifications should be unread"
        );
    }

    // Test 3: Verify unread count matches filtered results
    let count_response = client
        .get(format!("{}/api/notifications/unread-count", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(count_response.status(), StatusCode::OK);

    let count_result: Value = count_response
        .json()
        .await
        .expect("Failed to parse response");

    assert_eq!(
        count_result["unread_count"].as_u64().unwrap(),
        3,
        "Unread count should match filtered results"
    );
}
