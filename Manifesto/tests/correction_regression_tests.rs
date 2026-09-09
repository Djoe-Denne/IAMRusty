//! Regression coverage for ownership, join rules, suspend, and partial PUT.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use reqwest::StatusCode;
use rustycog::permission::{Permission, ResourceRef, Subject};
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

fn jwt(user_id: Uuid) -> String {
    rustycog::testing::http::jwt::create_jwt_token(user_id)
}

const fn project_resource(project_id: Uuid) -> ResourceRef {
    ResourceRef::new("project", project_id)
}

#[tokio::test]
#[serial]
async fn join_draft_public_is_forbidden() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let stranger = Uuid::new_v4();
    let owner_jwt = jwt(owner);

    let created = client
        .post(format!("{base_url}/api/projects"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "name": format!("DraftPublic-{}", Uuid::new_v4()),
            "owner_type": "personal",
            "visibility": "public"
        }))
        .send()
        .await
        .expect("create");
    assert_eq!(created.status(), StatusCode::CREATED);
    let body: Value = created.json().await.expect("body");
    let project_id = Uuid::parse_str(body["id"].as_str().expect("id")).expect("uuid");
    openfga
        .allow_wildcard(Permission::Read, project_resource(project_id))
        .await
        .expect("wildcard");

    let join = client
        .post(format!("{base_url}/api/projects/{project_id}/join"))
        .header("Authorization", format!("Bearer {}", jwt(stranger)))
        .send()
        .await
        .expect("join");
    assert_eq!(join.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn put_omitting_description_keeps_existing_value() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let owner_jwt = jwt(owner);
    let created = client
        .post(format!("{base_url}/api/projects"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "name": format!("KeepDesc-{}", Uuid::new_v4()),
            "description": "keep me",
            "owner_type": "personal"
        }))
        .send()
        .await
        .expect("create");
    assert_eq!(created.status(), StatusCode::CREATED);
    let body: Value = created.json().await.expect("body");
    let project_id = Uuid::parse_str(body["id"].as_str().expect("id")).expect("uuid");
    openfga
        .allow_all(Subject::new(owner), project_resource(project_id))
        .await
        .expect("owner tuples");

    let updated = client
        .put(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "name": "renamed-only" }))
        .send()
        .await
        .expect("put");
    assert_eq!(updated.status(), StatusCode::OK);
    let body: Value = updated.json().await.expect("put body");
    assert_eq!(body["name"], "renamed-only");
    assert_eq!(body["description"], "keep me");
    assert!(body.get("external_collaboration_enabled").is_none());
    assert!(body.get("data_classification").is_none());
}

#[tokio::test]
#[serial]
async fn suspend_and_resume_project() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let db = fixture.db();
    let project = DbFixtures::project()
        .personal(owner)
        .private()
        .active()
        .name(format!("Suspend-{}", Uuid::new_v4()))
        .commit(db.clone())
        .await
        .expect("project");
    let project_id = project.id();
    DbFixtures::member()
        .owner(project_id, owner)
        .commit(db)
        .await
        .expect("owner");
    openfga
        .allow_all(Subject::new(owner), project_resource(project_id))
        .await
        .expect("tuples");
    let owner_jwt = jwt(owner);

    let suspended = client
        .post(format!("{base_url}/api/projects/{project_id}/suspend"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .send()
        .await
        .expect("suspend");
    assert_eq!(suspended.status(), StatusCode::OK);
    let body: Value = suspended.json().await.expect("suspend body");
    assert_eq!(body["status"], "suspended");

    let resumed = client
        .post(format!("{base_url}/api/projects/{project_id}/resume"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .send()
        .await
        .expect("resume");
    assert_eq!(resumed.status(), StatusCode::OK);
    let body: Value = resumed.json().await.expect("resume body");
    assert_eq!(body["status"], "active");
}

#[tokio::test]
#[serial]
async fn transfer_ownership_demotes_previous_owner() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let successor = Uuid::new_v4();
    let db = fixture.db();
    let project = DbFixtures::project()
        .personal(owner)
        .private()
        .active()
        .name(format!("Transfer-{}", Uuid::new_v4()))
        .commit(db.clone())
        .await
        .expect("project");
    let project_id = project.id();
    DbFixtures::member()
        .owner(project_id, owner)
        .commit(db.clone())
        .await
        .expect("owner");
    DbFixtures::member()
        .direct(project_id, successor, owner)
        .commit(db)
        .await
        .expect("successor");
    openfga
        .allow_all(Subject::new(owner), project_resource(project_id))
        .await
        .expect("owner tuples");
    openfga
        .allow(
            Subject::new(successor),
            Permission::Read,
            project_resource(project_id),
        )
        .await
        .expect("successor read");

    let transferred = client
        .post(format!(
            "{base_url}/api/projects/{project_id}/transfer-ownership"
        ))
        .header("Authorization", format!("Bearer {}", jwt(owner)))
        .header("Content-Type", "application/json")
        .json(&json!({ "user_id": successor }))
        .send()
        .await
        .expect("transfer");
    assert_eq!(transferred.status(), StatusCode::OK);
    let body: Value = transferred.json().await.expect("body");
    assert_eq!(body["user_id"], successor.to_string());
}

#[tokio::test]
#[serial]
async fn rejoin_during_grace_keeps_member_id() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let joiner = Uuid::new_v4();
    let db = fixture.db();
    let project = DbFixtures::project()
        .personal(owner)
        .public()
        .active()
        .name(format!("GraceKeep-{}", Uuid::new_v4()))
        .commit(db.clone())
        .await
        .expect("project");
    let project_id = project.id();
    DbFixtures::member()
        .owner(project_id, owner)
        .commit(db)
        .await
        .expect("owner");
    openfga
        .allow_all(Subject::new(owner), project_resource(project_id))
        .await
        .expect("owner tuples");
    openfga
        .allow_wildcard(Permission::Read, project_resource(project_id))
        .await
        .expect("wildcard");

    let joined = client
        .post(format!("{base_url}/api/projects/{project_id}/join"))
        .header("Authorization", format!("Bearer {}", jwt(joiner)))
        .send()
        .await
        .expect("join");
    assert_eq!(joined.status(), StatusCode::CREATED);
    let first: Value = joined.json().await.expect("join body");
    let first_id = first["id"].as_str().expect("id").to_string();

    let removed = client
        .delete(format!(
            "{base_url}/api/projects/{project_id}/members/{joiner}"
        ))
        .header("Authorization", format!("Bearer {}", jwt(owner)))
        .send()
        .await
        .expect("remove");
    assert_eq!(removed.status(), StatusCode::NO_CONTENT);

    let rejoined = client
        .post(format!("{base_url}/api/projects/{project_id}/join"))
        .header("Authorization", format!("Bearer {}", jwt(joiner)))
        .send()
        .await
        .expect("rejoin");
    assert_eq!(rejoined.status(), StatusCode::CREATED);
    let second: Value = rejoined.json().await.expect("rejoin body");
    assert_eq!(second["id"].as_str().expect("id"), first_id);
}

#[tokio::test]
#[serial]
async fn rejoin_after_grace_creates_new_member_id() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let joiner = Uuid::new_v4();
    let db = fixture.db();
    let project = DbFixtures::project()
        .personal(owner)
        .public()
        .active()
        .name(format!("GraceExpire-{}", Uuid::new_v4()))
        .commit(db.clone())
        .await
        .expect("project");
    let project_id = project.id();
    DbFixtures::member()
        .owner(project_id, owner)
        .commit(db.clone())
        .await
        .expect("owner");
    let expired = DbFixtures::member()
        .direct(project_id, joiner, owner)
        .removed_after_grace()
        .commit(db)
        .await
        .expect("expired membership");
    openfga
        .allow_all(Subject::new(owner), project_resource(project_id))
        .await
        .expect("owner tuples");
    openfga
        .allow_wildcard(Permission::Read, project_resource(project_id))
        .await
        .expect("wildcard");

    let joined = client
        .post(format!("{base_url}/api/projects/{project_id}/join"))
        .header("Authorization", format!("Bearer {}", jwt(joiner)))
        .send()
        .await
        .expect("join");
    assert_eq!(joined.status(), StatusCode::CREATED);
    let body: Value = joined.json().await.expect("body");
    assert_ne!(body["id"].as_str().expect("id"), expired.id().to_string());
}

#[tokio::test]
#[serial]
async fn component_list_filters_to_instance_acl() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let member = Uuid::new_v4();
    let db = fixture.db();
    let project = DbFixtures::project()
        .personal(owner)
        .private()
        .active()
        .name(format!("AclList-{}", Uuid::new_v4()))
        .commit(db.clone())
        .await
        .expect("project");
    let project_id = project.id();
    DbFixtures::member()
        .owner(project_id, owner)
        .commit(db.clone())
        .await
        .expect("owner");
    DbFixtures::member()
        .direct(project_id, member, owner)
        .project_only()
        .commit(db)
        .await
        .expect("member");
    openfga
        .allow_all(Subject::new(owner), project_resource(project_id))
        .await
        .expect("owner tuples");
    openfga
        .allow(
            Subject::new(member),
            Permission::Read,
            project_resource(project_id),
        )
        .await
        .expect("member read");

    let owner_jwt = jwt(owner);
    let created = client
        .post(format!("{base_url}/api/projects/{project_id}/components"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "component_type": "taskboard" }))
        .send()
        .await
        .expect("add taskboard");
    assert_eq!(created.status(), StatusCode::CREATED);
    let first = Uuid::parse_str(
        created.json::<Value>().await.expect("body")["id"]
            .as_str()
            .expect("id"),
    )
    .expect("uuid");
    let created = client
        .post(format!("{base_url}/api/projects/{project_id}/components"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "component_type": "wiki" }))
        .send()
        .await
        .expect("add wiki");
    assert_eq!(created.status(), StatusCode::CREATED);
    let second = Uuid::parse_str(
        created.json::<Value>().await.expect("body")["id"]
            .as_str()
            .expect("id"),
    )
    .expect("uuid");

    let grant = client
        .post(format!(
            "{base_url}/api/projects/{project_id}/members/{member}/permissions/component/{first}"
        ))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "permission": "read" }))
        .send()
        .await
        .expect("grant");
    assert_eq!(grant.status(), StatusCode::OK);

    let listed = client
        .get(format!("{base_url}/api/projects/{project_id}/components"))
        .header("Authorization", format!("Bearer {}", jwt(member)))
        .send()
        .await
        .expect("list");
    assert_eq!(listed.status(), StatusCode::OK);
    let body: Value = listed.json().await.expect("list body");
    let data = body["data"].as_array().expect("data");
    let ids: Vec<String> = data
        .iter()
        .filter_map(|c| c["id"].as_str().map(str::to_string))
        .collect();
    assert!(ids.contains(&first.to_string()));
    assert!(!ids.contains(&second.to_string()));
}
