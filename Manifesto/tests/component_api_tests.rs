//! Component API endpoint tests for Manifesto
//!
//! Tests for component CRUD operations

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
// Component Creation Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_add_component_returns_201_with_valid_data() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .post(&format!("{}/api/projects/{}/components", base_url, project.id()))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "component_type": "taskboard"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        201,
        "Should return 201 Created for valid component creation"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    assert!(response_json["id"].is_string(), "Should return component ID");
    assert_eq!(
        response_json["component_type"], "taskboard",
        "Should return correct component type"
    );
    assert_eq!(
        response_json["status"], "pending",
        "New component should be in pending status"
    );
}

#[tokio::test]
#[serial]
async fn test_add_component_returns_409_for_duplicate() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _member, _component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("Failed to create project with component");

    let jwt_token = create_test_jwt_token(owner_id);

    // Try to add the same component type again
    let response = client
        .post(&format!("{}/api/projects/{}/components", base_url, project.id()))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "component_type": "taskboard"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        409,
        "Should return 409 Conflict for duplicate component"
    );
}

// =============================================================================
// Component Read Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_get_component_returns_200_for_existing() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("Failed to create project with component");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .get(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component.id()
        ))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for existing component"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    assert_eq!(
        response_json["id"],
        component.id().to_string(),
        "Should return correct component ID"
    );
}

#[tokio::test]
#[serial]
async fn test_list_components_returns_all_project_components() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Add multiple components
    DbFixtures::component()
        .for_project(project.id())
        .taskboard()
        .commit(db.clone())
        .await
        .expect("Failed to create taskboard component");

    DbFixtures::component()
        .for_project(project.id())
        .wiki()
        .commit(db.clone())
        .await
        .expect("Failed to create wiki component");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .get(&format!("{}/api/projects/{}/components", base_url, project.id()))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for component list"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    assert!(response_json["data"].is_array(), "Should return data array");
    let components = response_json["data"].as_array().unwrap();
    assert_eq!(components.len(), 2, "Should return all project components");
}

// =============================================================================
// Component Update Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_update_component_status_returns_200() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("Failed to create project with component");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .patch(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component.id()
        ))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "status": "configured"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for status update"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");

    assert_eq!(
        response_json["status"], "configured",
        "Component should be in configured status"
    );
}

// =============================================================================
// Component Delete Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_remove_component_returns_204() {
    let (_fixture, base_url, client) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("Failed to create project with component");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .delete(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component.id(),
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


