//! Integration tests for Telegraph notification error scenarios and edge cases
//!
//! Tests error handling, validation, and boundary conditions:
//! 1. Invalid input validation
//! 2. Database constraint violations
//! 3. Malformed requests
//! 4. Rate limiting and abuse prevention
//! 5. Resource exhaustion scenarios

mod common;

#[path = "fixtures/mod.rs"]
mod fixtures;

use axum::http::{header, StatusCode};
use common::*;
use serde_json::Value;
use serial_test::serial;
use uuid::Uuid;

use fixtures::db::NotificationFixtureBuilder;
use rustycog_testing::http::jwt::create_jwt_token;

/// Test parameter validation - negative page numbers
#[tokio::test]
#[serial]
async fn test_get_notifications_negative_page() {
    let (_, base_url, client, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    let response = client
        .get(format!("{}/api/notifications?page=-1", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Test parameter validation - per_page exceeds maximum limit
#[tokio::test]
#[serial]
async fn test_get_notifications_per_page_exceeds_limit() {
    let (_, base_url, client, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    let response = client
        .get(format!("{}/api/notifications?per_page=101", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// Test parameter validation - per_page is zero
#[tokio::test]
#[serial]
async fn test_get_notifications_per_page_zero() {
    let (_, base_url, client, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    let response = client
        .get(format!("{}/api/notifications?per_page=0", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// Test malformed query parameters
#[tokio::test]
#[serial]
async fn test_get_notifications_malformed_params() {
    let (_, base_url, client, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    let malformed_requests = vec![
        format!("{}/api/notifications?page=abc", base_url),
        format!("{}/api/notifications?per_page=xyz", base_url),
        format!("{}/api/notifications?unread_only=maybe", base_url),
        format!("{}/api/notifications?page=1.5", base_url),
        format!("{}/api/notifications?per_page=-5", base_url),
    ];

    for uri in malformed_requests {
        let response = client
            .get(uri.clone())
            .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
            .send()
            .await
            .expect("Failed to send request");

        assert!(
            response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::UNPROCESSABLE_ENTITY,
            "Malformed request {} should return 400 or 422, got {}",
            uri,
            response.status()
        );
    }
}

/// Test invalid UUID formats in mark-as-read endpoint
#[tokio::test]
#[serial]
async fn test_mark_notification_read_invalid_rights() {
    let (_, base_url, client, openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    let invalid_right_uuid = Uuid::new_v4();

    // We're testing the denial path. The route guard
    // `with_permission_on(Permission::Write, "notification")` on
    // `PUT /api/notifications/{id}/read` will Check
    // `(user_id, Write, notification:<invalid_right_uuid>)`. Reset the
    // harness's permissive default and mount a deny matching the exact
    // tuple the middleware will issue (wiremock matches in registration
    // order, so a deny mounted on top of the catch-all would never fire).
    openfga.reset().await;
    openfga
        .mock_check_deny(
            Subject::new(user_id),
            Permission::Write,
            ResourceRef::new("notification", invalid_right_uuid),
        )
        .await;

    let response = client
        .put(format!(
            "{}/api/notifications/{}/read",
            base_url, invalid_right_uuid
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Invalid right UUID {} should return 403",
        invalid_right_uuid
    );
}

/// Test JWT token validation edge cases
#[tokio::test]
#[serial]
async fn test_jwt_validation_edge_cases() {
    let (_, base_url, client, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let test_cases = vec![
        ("", StatusCode::UNAUTHORIZED),                         // Empty token
        ("Bearer", StatusCode::UNAUTHORIZED),                   // Missing token part
        ("Bearer ", StatusCode::UNAUTHORIZED),                  // Empty token after Bearer
        ("Basic dGVzdDp0ZXN0", StatusCode::UNAUTHORIZED),       // Wrong auth type
        ("Bearer invalid.jwt.token", StatusCode::UNAUTHORIZED), // Invalid JWT
        (
            "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
            StatusCode::UNAUTHORIZED,
        ), // Incomplete JWT
    ];

    for (auth_header, expected_status) in test_cases {
        let response = client
            .get(format!("{}/api/notifications", base_url))
            .header(header::AUTHORIZATION, auth_header)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(
            response.status(),
            expected_status,
            "Auth header '{}' should return {}",
            auth_header,
            expected_status
        );
    }
}

/// Test database connection error simulation
#[tokio::test]
#[serial]
async fn test_page_number_error_handling() {
    let (_, base_url, client, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Request an extremely large page that might cause memory issues
    let response = client
        .get(format!("{}/api/notifications?page=101", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    // Should handle gracefully - either return empty results or appropriate error
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// Test empty database scenarios
#[tokio::test]
#[serial]
async fn test_empty_database_scenarios() {
    let (_, base_url, client, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Test get notifications with no data
    let response = client
        .get(format!("{}/api/notifications", base_url))
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let result: Value = response.json().await.expect("Failed to parse response");

    assert_eq!(result["notifications"].as_array().unwrap().len(), 0);
    assert_eq!(result["total_count"].as_u64().unwrap(), 0);
    assert_eq!(result["has_more"].as_bool().unwrap(), false);

    // Test unread count with no data
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

    assert_eq!(count_result["unread_count"].as_u64().unwrap(), 0);
}

/// Test boundary conditions for pagination
#[tokio::test]
#[serial]
async fn test_pagination_boundary_conditions() {
    let (fixture, base_url, client, _openfga) = setup_test_server()
        .await
        .expect("Failed to setup Telegraph test server");
    let db = fixture.db();

    let user_id = Uuid::new_v4();
    let jwt_token = create_jwt_token(user_id);

    // Create exactly one notification
    let _notification = NotificationFixtureBuilder::new()
        .user_id(user_id)
        .title("Single notification".to_string())
        .commit(&db)
        .await
        .expect("Failed to create notification");

    // Test edge cases around the single notification
    let test_cases = vec![
        (
            format!("{}/api/notifications?page=0&per_page=1", base_url),
            1,
            false,
        ), // Exact match
        (
            format!("{}/api/notifications?page=0&per_page=2", base_url),
            1,
            false,
        ), // per_page > available
        (
            format!("{}/api/notifications?page=1&per_page=1", base_url),
            0,
            false,
        ), // page beyond data
        (
            format!("{}/api/notifications?page=0&per_page=100", base_url),
            1,
            false,
        ), // large per_page
    ];

    for (uri, expected_count, expected_has_more) in test_cases {
        let response = client
            .get(uri.clone())
            .header(header::AUTHORIZATION, format!("Bearer {}", jwt_token))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request to {} should succeed",
            uri
        );

        let result: Value = response.json().await.expect("Failed to parse response");

        assert_eq!(
            result["notifications"].as_array().unwrap().len(),
            expected_count,
            "URI {} should return {} notifications",
            uri,
            expected_count
        );

        assert_eq!(
            result["has_more"].as_bool().unwrap(),
            expected_has_more,
            "URI {} should have has_more={}",
            uri,
            expected_has_more
        );
    }
}
