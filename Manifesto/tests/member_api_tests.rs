//! Member API endpoint tests for Manifesto
//!
//! Tests for member management and permission operations

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use rustycog_permission::{Permission, ResourceRef, Subject};
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
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Route guard: `with_permission_on(Permission::Admin, "project")`.
    // POST /members trailing UUID = project.id().
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("Failed to grant project admin");

    let jwt_token = create_test_jwt_token(owner_id);
    let new_member_id = Uuid::new_v4();

    let response = client
        .post(format!(
            "{}/api/projects/{}/members",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
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
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
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

    // Default-deny: the route guard
    // `with_permission_on(Permission::Admin, "project")` issues
    // `Check(regular_member, administer, project:<project_id>)` (the
    // trailing UUID in `POST /api/projects/{project_id}/members` is the
    // project id itself — there is no deeper UUID segment). Real OpenFGA
    // returns false because no tuple has been written for that subject,
    // so the request 403s without any explicit arrange.

    let jwt_token = create_test_jwt_token(regular_member_id);
    let new_member_id = Uuid::new_v4();

    let response = client
        .post(format!(
            "{}/api/projects/{}/members",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
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
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("Failed to create project with owner");

    // Route guard: `with_permission_on(Permission::Read, "project")`.
    // GET /members/{user_id} trailing UUID = owner_id (the user id).
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Read,
            ResourceRef::new("project", owner_id),
        )
        .await
        .expect("Failed to grant project read");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .get(format!(
            "{}/api/projects/{}/members/{}",
            base_url,
            project.id(),
            owner_id
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
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
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
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

    // Route guard: `with_permission_on(Permission::Read, "project")`.
    // GET /members trailing UUID = project.id().
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Read,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("Failed to grant project read");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .get(format!(
            "{}/api/projects/{}/members",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
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
    assert!(
        members.len() >= 4,
        "Should return all members including owner"
    );
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
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
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

    // Route guard: `with_permission_on(Permission::Admin, "project")`.
    // PUT /members/{user_id} trailing UUID = regular_member_id.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", regular_member_id),
        )
        .await
        .expect("Failed to grant project admin");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .put(format!(
            "{}/api/projects/{}/members/{}",
            base_url,
            project.id(),
            regular_member_id
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
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
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
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

    // Route guard: `with_permission_on(Permission::Admin, "project")`.
    // DELETE /members/{user_id} trailing UUID = member_to_remove_id.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", member_to_remove_id),
        )
        .await
        .expect("Failed to grant project admin");

    let jwt_token = create_test_jwt_token(owner_id);

    let response = client
        .delete(format!(
            "{}/api/projects/{}/members/{}",
            base_url,
            project.id(),
            member_to_remove_id
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
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
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
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

    // Route guard: `with_permission_on(Permission::Admin, "project")`.
    // POST /permissions/{resource} trailing UUID = regular_member_id.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", regular_member_id),
        )
        .await
        .expect("Failed to grant project admin");

    let jwt_token = create_test_jwt_token(owner_id);

    // Grant permission using path-based resource: /permissions/{resource}
    let response = client
        .post(format!(
            "{}/api/projects/{}/members/{}/permissions/component",
            base_url,
            project.id(),
            regular_member_id
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .header("Content-Type", "application/json")
        .json(&json!({
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
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
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

    // Route guard: `with_permission_on(Permission::Admin, "project")`.
    // POST + DELETE /permissions/{resource} both share the trailing UUID
    // = regular_member_id, so a single tuple covers both calls.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", regular_member_id),
        )
        .await
        .expect("Failed to grant project admin");

    let jwt_token = create_test_jwt_token(owner_id);

    // First grant a permission using path-based resource
    client
        .post(format!(
            "{}/api/projects/{}/members/{}/permissions/component",
            base_url,
            project.id(),
            regular_member_id
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "permission": "write"
        }))
        .send()
        .await
        .expect("Failed to grant permission");

    // Now revoke it using path-based resource
    let response = client
        .delete(format!(
            "{}/api/projects/{}/members/{}/permissions/component",
            base_url,
            project.id(),
            regular_member_id
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        204,
        "Should return 204 No Content for permission revoke"
    );
}

// =============================================================================
// Component-Specific Permission Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_grant_permission_on_specific_component_returns_200() {
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("Failed to create project with component");

    // Add a regular member with read-only permissions
    let regular_member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), regular_member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create regular member");

    // Route guard: `with_permission_on(Permission::Admin, "project")`.
    // POST /permissions/component/{component_id} trailing UUID = component.id().
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", component.id()),
        )
        .await
        .expect("Failed to grant project admin");

    let jwt_token = create_test_jwt_token(owner_id);

    // Grant write permission on specific component using path: /permissions/component/{component_id}
    let response = client
        .post(format!(
            "{}/api/projects/{}/members/{}/permissions/component/{}",
            base_url,
            project.id(),
            regular_member_id,
            component.id()
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "permission": "write"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for granting permission on specific component"
    );
}

#[tokio::test]
#[serial]
async fn test_non_admin_cannot_grant_permission_on_specific_component() {
    let (_fixture, base_url, client, _openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("Failed to create project with component");

    // Acting user is a regular member with only read-level project permissions.
    let acting_member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), acting_member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create acting member");

    // Target member receives the grant attempt.
    let target_member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), target_member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create target member");

    // Default-deny: the grant route is
    // `POST /api/projects/{project_id}/members/{user_id}/permissions/component/{component_id}`,
    // and the middleware uses the **trailing** UUID — `component_id` — as
    // the resource id. Real OpenFGA returns false for
    // `Check(acting_member, administer, project:<component_id>)` because
    // no tuple has been written, so the request 403s.

    let acting_token = create_test_jwt_token(acting_member_id);

    let response = client
        .post(format!(
            "{}/api/projects/{}/members/{}/permissions/component/{}",
            base_url,
            project.id(),
            target_member_id,
            component.id()
        ))
        .header("Authorization", format!("Bearer {acting_token}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "permission": "write"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        403,
        "Regular members must not grant specific-component permissions"
    );
}

#[tokio::test]
#[serial]
async fn test_grant_admin_permission_on_specific_component_returns_200() {
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "wiki")
            .await
            .expect("Failed to create project with wiki component");

    // Add a regular member
    let regular_member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), regular_member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create regular member");

    // Route guard: `with_permission_on(Permission::Admin, "project")`.
    // POST /permissions/component/{component_id} trailing UUID = component.id().
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", component.id()),
        )
        .await
        .expect("Failed to grant project admin");

    let jwt_token = create_test_jwt_token(owner_id);

    // Grant admin permission on specific component using path: /permissions/component/{component_id}
    let response = client
        .post(format!(
            "{}/api/projects/{}/members/{}/permissions/component/{}",
            base_url,
            project.id(),
            regular_member_id,
            component.id()
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "permission": "admin"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        200,
        "Should return 200 OK for granting admin permission on specific component"
    );
}

#[tokio::test]
#[serial]
async fn test_revoke_permission_on_specific_component_returns_204() {
    let (_fixture, base_url, client, openfga, _components) = setup_test_server()
        .await
        .expect("Failed to setup test server");
    let db = _fixture.db();

    let owner_id = Uuid::new_v4();
    let (project, _owner_member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("Failed to create project with component");

    // Add a regular member
    let regular_member_id = Uuid::new_v4();
    DbFixtures::member()
        .direct(project.id(), regular_member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("Failed to create regular member");

    // Route guard: `with_permission_on(Permission::Admin, "project")`.
    // Both POST + DELETE share the trailing UUID = component.id().
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", component.id()),
        )
        .await
        .expect("Failed to grant project admin");

    let jwt_token = create_test_jwt_token(owner_id);

    // First grant a permission on specific component using path
    client
        .post(format!(
            "{}/api/projects/{}/members/{}/permissions/component/{}",
            base_url,
            project.id(),
            regular_member_id,
            component.id()
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "permission": "write"
        }))
        .send()
        .await
        .expect("Failed to grant permission on specific component");

    // Now revoke it using path: /permissions/component/{component_id}
    let response = client
        .delete(format!(
            "{}/api/projects/{}/members/{}/permissions/component/{}",
            base_url,
            project.id(),
            regular_member_id,
            component.id()
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        204,
        "Should return 204 No Content for revoking permission on specific component"
    );
}
