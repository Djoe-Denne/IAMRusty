//! Member API endpoint tests for Manifesto
//!
//! Tests for member management and permission operations

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

// Helper function to create a JWT token for testing
fn create_test_jwt_token(user_id: Uuid) -> String {
    rustycog_testing::http::jwt::create_jwt_token(user_id)
}

// =============================================================================
// Member Addition Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_add_member_returns_201_with_valid_permissions() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    let jwt_token = create_test_jwt_token(owner_id);
    let new_member_id = Uuid::new_v4();

    let response = client
        .post(&format!("{}/api/projects/{}/members", base_url, project.id()))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "user_id": new_member_id.to_string(),
            "permission": "read",
            "resource": "project"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        201,
        "Should return 201 Created for valid member addition"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    assert!(response_json["id"].is_string(), "Should return member ID");
    assert_eq!(
        response_json["user_id"],
        new_member_id.to_string(),
        "Should return correct user ID"
    );
}

#[tokio::test]
#[serial]
async fn test_add_member_returns_403_without_admin_permission() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Create a regular member without admin permissions
    let regular_member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), regular_member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create regular member");

    let jwt_token = create_test_jwt_token(regular_member_id);
    let new_member_id = Uuid::new_v4();

    let response = client
        .post(&format!("{}/api/projects/{}/members", base_url, project.id()))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "user_id": new_member_id.to_string(),
            "permission": "read"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        403,
        "Should return 403 Forbidden without admin permission"
    );
}

// =============================================================================
// Member Read Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_get_member_returns_200_with_permissions() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .get(&format!(
            "{}/api/projects/{}/members/{}",
            base_url,
            project.id(),
            owner_id
        ))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for existing member"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    assert_eq!(
        response_json["user_id"],
        owner_id.to_string(),
        "Should return correct user ID"
    );
    assert!(
        response_json["permissions"].is_array(),
        "Should return permissions array"
    );
}

#[tokio::test]
#[serial]
async fn test_list_members_returns_paginated_results() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Add additional members
    for _ in 0..3 {
        let member_id = Uuid::new_v4();
        DbFixtures::member()
            .direct(project.id(), member_id, owner_id)
            .commit(db.clone())
            .await
            .expect("Failed to create member");
    }

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .get(&format!("{}/api/projects/{}/members", base_url, project.id()))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for member list"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    assert!(response_json["data"].is_array(), "Should return data array");
    let members = response_json["data"].as_array().unwrap();
    assert!(members.len() >= 4, "Should return all members including owner");
    assert!(
        response_json["pagination"].is_object(),
        "Should return pagination info"
    );
}

// =============================================================================
// Member Update Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_update_member_permissions_returns_200() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Add a regular member
    let regular_member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), regular_member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create regular member");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .put(&format!(
            "{}/api/projects/{}/members/{}",
            base_url,
            project.id(),
            regular_member_id
        ))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "permissions": [
                {"resource": "project", "permission": "write"},
                {"resource": "component", "permission": "admin"}
            ]
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for permission update"
    );
}

// =============================================================================
// Member Removal Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_remove_member_returns_204() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Add a regular member to remove
    let member_to_remove_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), member_to_remove_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create member to remove");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .delete(&format!(
            "{}/api/projects/{}/members/{}",
            base_url,
            project.id(),
            member_to_remove_id
        ))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        204,
        "Should return 204 No Content for successful removal"
    );
}

// =============================================================================
// Permission Grant/Revoke Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_grant_permission_returns_200() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Add a regular member
    let regular_member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), regular_member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create regular member");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .post(&format!(
            "{}/api/projects/{}/members/{}/permissions",
            base_url,
            project.id(),
            regular_member_id
        ))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "resource": "component",
            "permission": "write"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for permission grant"
    );
}

#[tokio::test]
#[serial]
async fn test_revoke_permission_returns_204() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Add a regular member
    let regular_member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), regular_member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create regular member");

    let jwt_token = create_test_jwt_token(owner_id);

    // First grant a permission
    client
        .post(&format!(
            "{}/api/projects/{}/members/{}/permissions",
            base_url,
            project.id(),
            regular_member_id
        ))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "resource": "component",
            "permission": "write"
        }))
        .send()
        .await
        .expect("Failed to grant permission");

    // Now revoke it
    let response = client
        .delete(&format!(
            "{}/api/projects/{}/members/{}/permissions?resource=component",
            base_url,
            project.id(),
            regular_member_id
        ))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        204,
        "Should return 204 No Content for permission revoke"
    );
}


