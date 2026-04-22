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
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
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
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
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
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
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
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
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
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
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
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
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

// =============================================================================
// Generic vs Specific Component Permission Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_user_with_generic_component_read_can_access_any_component() {
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Create multiple components
    let component1 = DbFixtures::component()
        .for_project(project.id())
        .taskboard()
        .commit(db.clone())
        .await
        .expect("Failed to create taskboard component");

    let component2 = DbFixtures::component()
        .for_project(project.id())
        .wiki()
        .commit(db.clone())
        .await
        .expect("Failed to create wiki component");

    // Add a member with generic "component" read permission (via direct membership)
    let member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create member");

    let jwt_token = create_test_jwt_token(member_id);

    // Should be able to read component1
    let response1 = client
        .get(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component1.id()
        ))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response1.status(),
        200,
        "User with generic component read should access component1"
    );

    // Should also be able to read component2
    let response2 = client
        .get(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component2.id()
        ))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response2.status(),
        200,
        "User with generic component read should access component2"
    );
}

#[tokio::test]
#[serial]
async fn test_user_with_generic_component_permission_can_list_all_components() {
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Create multiple components
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

    // Add a member with generic permissions
    let member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create member");

    let jwt_token = create_test_jwt_token(member_id);

    // Should be able to list all components
    let response = client
        .get(&format!("{}/api/projects/{}/components", base_url, project.id()))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        200,
        "User with generic component permission should list all components"
    );

    let response_json: Value = response.json().await.expect("Should return JSON response");
    let components = response_json["data"].as_array().unwrap();
    assert_eq!(components.len(), 2, "Should see all components in the project");
}

#[tokio::test]
#[serial]
async fn test_owner_can_modify_any_component() {
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Create a component
    let component = DbFixtures::component()
        .for_project(project.id())
        .taskboard()
        .commit(db.clone())
        .await
        .expect("Failed to create taskboard component");

    let jwt_token = create_test_jwt_token(owner_id);

    // Owner should be able to update component status
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
        "Owner with generic admin permission should modify any component"
    );
}

#[tokio::test]
#[serial]
async fn test_member_without_component_permission_cannot_modify_component() {
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("Failed to create project with component");

    // Add a member with only read permission (no admin)
    let member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create member");

    // The route guard `with_permission_on(Permission::Admin, "project")` will
    // ask OpenFGA whether `member_id` may admin this component. We're testing
    // the denial path, so the fake must answer no for that exact tuple. Wipe
    // the permissive `mock_check_any(true)` mounted by `setup_test_server`
    // first — wiremock matches in registration order.
    openfga.reset().await;
    openfga
        .mock_check_deny(
            Subject::new(member_id),
            Permission::Admin,
            ResourceRef::new("project", component.id()),
        )
        .await;

    let jwt_token = create_test_jwt_token(member_id);

    // Member with read permission should not be able to update component status
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
        403,
        "Member with only read permission should not modify component"
    );
}

#[tokio::test]
#[serial]
async fn test_member_without_component_permission_cannot_delete_component() {
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("Failed to create project with component");

    // Add a member with only read permission
    let member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create member");

    // We're testing the denial path on DELETE. The route guard
    // `with_permission_on(Permission::Admin, "project")` will issue a Check
    // for `(member_id, Admin, project:<component_id>)`. Reset the permissive
    // default and mount a deny for that exact tuple.
    openfga.reset().await;
    openfga
        .mock_check_deny(
            Subject::new(member_id),
            Permission::Admin,
            ResourceRef::new("project", component.id()),
        )
        .await;

    let jwt_token = create_test_jwt_token(member_id);

    // Member with read permission should not be able to delete component
    let response = client
        .delete(&format!(
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
        403,
        "Member with only read permission should not delete component"
    );
}

// =============================================================================
// Specific Component Permission Enforcement Tests
// =============================================================================
// These tests verify that component-specific permissions (granted via the
// /permissions/component/{component_id} API) are properly enforced.

#[tokio::test]
#[serial]
async fn test_granted_specific_component_permission_allows_access() {
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Create two components
    let component1 = DbFixtures::component()
        .for_project(project.id())
        .taskboard()
        .commit(db.clone())
        .await
        .expect("Failed to create taskboard component");

    let component2 = DbFixtures::component()
        .for_project(project.id())
        .wiki()
        .commit(db.clone())
        .await
        .expect("Failed to create wiki component");

    // Add a member with minimal permissions (just project read so they can authenticate)
    let member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create member");

    let owner_token = create_test_jwt_token(owner_id);
    let member_token = create_test_jwt_token(member_id);

    // Grant the member specific write permission on component1 only (not component2)
    let grant_response = client
        .post(&format!(
            "{}/api/projects/{}/members/{}/permissions/component/{}",
            base_url,
            project.id(),
            member_id,
            component1.id()
        ))
        .header("Authorization", format!("Bearer {}", owner_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "permission": "write"
        }))
        .send()
        .await
        .expect("Failed to grant specific component permission");

    assert_eq!(
        grant_response.status(),
        200,
        "Owner should be able to grant specific component permission"
    );

    // Member should be able to read component1 (has specific permission)
    let response1 = client
        .get(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component1.id()
        ))
        .header("Authorization", format!("Bearer {}", member_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response1.status(),
        200,
        "Member with specific component permission should access that component"
    );

    // Member should also be able to read component2 (has generic component read from membership)
    let response2 = client
        .get(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component2.id()
        ))
        .header("Authorization", format!("Bearer {}", member_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response2.status(),
        200,
        "Member should also access component2 via generic read permission"
    );
}

#[tokio::test]
#[serial]
async fn test_specific_component_admin_does_not_apply_to_other_components() {
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    let component1 = DbFixtures::component()
        .for_project(project.id())
        .taskboard()
        .commit(db.clone())
        .await
        .expect("Failed to create taskboard component");

    let component2 = DbFixtures::component()
        .for_project(project.id())
        .wiki()
        .commit(db.clone())
        .await
        .expect("Failed to create wiki component");

    let member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create member");

    let owner_token = create_test_jwt_token(owner_id);
    let member_token = create_test_jwt_token(member_id);

    // Permission topology this test asserts on (owner administers everything;
    // member has admin on component1 only):
    //   Check(owner,  Admin, project:component1)  -> allow  (POST grant)
    //   Check(member, Admin, project:component1)  -> allow  (PATCH component1)
    //   Check(member, Admin, project:component2)  -> DENY   (PATCH component2)
    // The middleware uses the trailing UUID in the path as the resource id,
    // so the two PATCHes hit distinct cache keys and can carry distinct
    // decisions even with caching on.
    openfga.reset().await;
    openfga
        .mock_check_allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", component1.id()),
        )
        .await
        .mock_check_allow(
            Subject::new(member_id),
            Permission::Admin,
            ResourceRef::new("project", component1.id()),
        )
        .await
        .mock_check_deny(
            Subject::new(member_id),
            Permission::Admin,
            ResourceRef::new("project", component2.id()),
        )
        .await;

    // Grant elevated access only on component1.
    let grant_response = client
        .post(&format!(
            "{}/api/projects/{}/members/{}/permissions/component/{}",
            base_url,
            project.id(),
            member_id,
            component1.id()
        ))
        .header("Authorization", format!("Bearer {}", owner_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "permission": "admin"
        }))
        .send()
        .await
        .expect("Failed to grant permission");

    assert_eq!(grant_response.status(), 200, "Grant should succeed");

    let update_component1 = client
        .patch(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component1.id()
        ))
        .header("Authorization", format!("Bearer {}", member_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "status": "configured"
        }))
        .send()
        .await
        .expect("Failed to update first component");

    assert_eq!(
        update_component1.status(),
        200,
        "Specific component admin should allow updates on that component"
    );

    let update_component2 = client
        .patch(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component2.id()
        ))
        .header("Authorization", format!("Bearer {}", member_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "status": "configured"
        }))
        .send()
        .await
        .expect("Failed to update second component");

    assert_eq!(
        update_component2.status(),
        403,
        "Specific component admin should not grant admin on other components"
    );
}

#[tokio::test]
#[serial]
async fn test_revoked_specific_component_permission_denies_elevated_access() {
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("Failed to create project with component");

    // Add a member with only read permission
    let member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create member");

    let owner_token = create_test_jwt_token(owner_id);
    let member_token = create_test_jwt_token(member_id);

    // Phase 1: while the grant is active, Check returns allow for the owner
    // (grantor / revoker) and the member (acting on the component). We
    // narrow the permissive default down to the exact tuples this test
    // asserts on so each assertion is backed by an explicit decision.
    openfga.reset().await;
    openfga
        .mock_check_allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", component.id()),
        )
        .await
        .mock_check_allow(
            Subject::new(member_id),
            Permission::Admin,
            ResourceRef::new("project", component.id()),
        )
        .await;

    // Grant specific admin permission on component
    client
        .post(&format!(
            "{}/api/projects/{}/members/{}/permissions/component/{}",
            base_url,
            project.id(),
            member_id,
            component.id()
        ))
        .header("Authorization", format!("Bearer {}", owner_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "permission": "admin"
        }))
        .send()
        .await
        .expect("Failed to grant permission");

    // Member should be able to update component (has admin permission)
    let update_response1 = client
        .patch(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component.id()
        ))
        .header("Authorization", format!("Bearer {}", member_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "status": "configured"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        update_response1.status(),
        200,
        "Member with specific admin permission should be able to update component"
    );

    // Now revoke the specific permission
    let revoke_response = client
        .delete(&format!(
            "{}/api/projects/{}/members/{}/permissions/component/{}",
            base_url,
            project.id(),
            member_id,
            component.id()
        ))
        .header("Authorization", format!("Bearer {}", owner_token))
        .send()
        .await
        .expect("Failed to revoke permission");

    assert_eq!(
        revoke_response.status(),
        204,
        "Owner should be able to revoke specific component permission"
    );

    // Phase 2: in production, the revoke would propagate through
    // `sentinel-sync` so the next OpenFGA Check returns false. The wiremock
    // fake doesn't observe Manifesto domain events, so we simulate that
    // propagation here — owner keeps admin (so the revoke endpoint stays
    // callable in the abstract), member is flipped to deny. The
    // `cache_ttl_seconds = 0` setting in `test.toml` is what makes this
    // re-arrangement actually visible to the next request: with the
    // production 15s TTL, `CachedPermissionChecker` would serve a stale
    // allow from Phase 1 and the assertion below would never reach the
    // refreshed mock.
    openfga.reset().await;
    openfga
        .mock_check_allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", component.id()),
        )
        .await
        .mock_check_deny(
            Subject::new(member_id),
            Permission::Admin,
            ResourceRef::new("project", component.id()),
        )
        .await;

    // Member should no longer be able to update component (only has generic read)
    let update_response2 = client
        .patch(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component.id()
        ))
        .header("Authorization", format!("Bearer {}", member_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "status": "active"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        update_response2.status(),
        403,
        "Member should not be able to update component after permission revocation"
    );
}

#[tokio::test]
#[serial]
async fn test_generic_permission_grants_access_to_all_components() {
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Create multiple components
    let component1 = DbFixtures::component()
        .for_project(project.id())
        .taskboard()
        .commit(db.clone())
        .await
        .expect("Failed to create taskboard component");

    let component2 = DbFixtures::component()
        .for_project(project.id())
        .wiki()
        .commit(db.clone())
        .await
        .expect("Failed to create wiki component");

    // Add a member with minimal permissions
    let member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create member");

    let owner_token = create_test_jwt_token(owner_id);
    let member_token = create_test_jwt_token(member_id);

    // Grant generic admin permission on "component" (not specific to any component)
    let grant_response = client
        .post(&format!(
            "{}/api/projects/{}/members/{}/permissions/component",
            base_url,
            project.id(),
            member_id
        ))
        .header("Authorization", format!("Bearer {}", owner_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "permission": "admin"
        }))
        .send()
        .await
        .expect("Failed to grant generic component permission");

    assert_eq!(
        grant_response.status(),
        200,
        "Owner should be able to grant generic component permission"
    );

    // Member should be able to update component1
    let update_response1 = client
        .patch(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component1.id()
        ))
        .header("Authorization", format!("Bearer {}", member_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "status": "configured"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        update_response1.status(),
        200,
        "Member with generic admin permission should update component1"
    );

    // Member should also be able to update component2
    let update_response2 = client
        .patch(&format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component2.id()
        ))
        .header("Authorization", format!("Bearer {}", member_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "status": "configured"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        update_response2.status(),
        200,
        "Member with generic admin permission should update component2"
    );
}


