//! Regression coverage for ownership, join rules, suspend, and partial PUT.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use std::sync::Arc;

use chrono::Utc;
use common::*;
use fixtures::DbFixtures;
use manifesto_events::PermissionGrantedEvent;
use manifesto_infra::repository::entity::{
    prelude::*, project_member_role_permissions, project_members,
};
use reqwest::StatusCode;
use rustycog::events::{DomainEvent, EventHandler};
use rustycog::outbox::entity::{Column as OutboxColumn, OutboxEvents};
use rustycog::permission::{Permission, ResourceRef, Subject};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sentinel_sync::fga_client::OpenFgaWriteClient;
use sentinel_sync::handler::SyncEventHandler;
use sentinel_sync::idempotency::{EventLedger, InMemoryEventLedger, PostgresEventLedger};
use sentinel_sync::translator::manifesto::ManifestoTranslator;
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
        .commit(db.clone())
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

    let owners = ProjectMembers::find()
        .filter(project_members::Column::ProjectId.eq(project_id))
        .filter(project_members::Column::IsOwner.eq(true))
        .filter(project_members::Column::RemovedAt.is_null())
        .all(db.as_ref())
        .await
        .expect("owners");
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].user_id, successor);

    let previous = ProjectMembers::find()
        .filter(project_members::Column::ProjectId.eq(project_id))
        .filter(project_members::Column::UserId.eq(owner))
        .one(db.as_ref())
        .await
        .expect("previous")
        .expect("previous member");
    assert!(!previous.is_owner);
    assert!(previous.removed_at.is_none());

    let previous_grants = ProjectMemberRolePermissions::find()
        .filter(project_member_role_permissions::Column::MemberId.eq(previous.id))
        .all(db.as_ref())
        .await
        .expect("grants");
    let mut saw_admin = false;
    for grant in previous_grants {
        let role = RolePermissions::find_by_id(grant.role_permission_id)
            .one(db.as_ref())
            .await
            .expect("role")
            .expect("role row");
        assert_ne!(role.permission_id, "owner");
        if role.permission_id == "admin" {
            saw_admin = true;
        }
    }
    assert!(saw_admin, "previous owner must keep admin grants");

    let project_row = Projects::find_by_id(project_id)
        .one(db.as_ref())
        .await
        .expect("project")
        .expect("project row");
    assert_eq!(project_row.owner_id, successor);

    let outbox = OutboxEvents::find()
        .filter(OutboxColumn::AggregateId.eq(project_id))
        .all(db.as_ref())
        .await
        .expect("outbox");
    assert!(outbox
        .iter()
        .any(|row| row.event_type == "project_ownership_transferred"));
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
    let permissions = second["permissions"].as_array().expect("permissions");
    assert_eq!(permissions.len(), 1);
    assert_eq!(permissions[0]["resource"], "project");
    assert_eq!(permissions[0]["permission"], "read");
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
        .commit(db.clone())
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

    let old_row = ProjectMembers::find_by_id(expired.id())
        .one(db.as_ref())
        .await
        .expect("old row")
        .expect("soft-deleted row kept");
    assert!(old_row.removed_at.is_some());
    let rows = ProjectMembers::find()
        .filter(project_members::Column::ProjectId.eq(project_id))
        .filter(project_members::Column::UserId.eq(joiner))
        .all(db.as_ref())
        .await
        .expect("member rows");
    assert_eq!(rows.len(), 2);
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

#[tokio::test]
#[serial]
async fn delete_draft_project_returns_204() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let owner_jwt = jwt(owner);
    let created = client
        .post(format!("{base_url}/api/projects"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "name": format!("DeleteDraft-{}", Uuid::new_v4()),
            "owner_type": "personal"
        }))
        .send()
        .await
        .expect("create");
    assert_eq!(created.status(), StatusCode::CREATED);
    let project_id = Uuid::parse_str(
        created.json::<Value>().await.expect("body")["id"]
            .as_str()
            .expect("id"),
    )
    .expect("uuid");
    openfga
        .allow_all(Subject::new(owner), project_resource(project_id))
        .await
        .expect("tuples");

    let deleted = client
        .delete(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .send()
        .await
        .expect("delete");
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
#[serial]
async fn suspend_twice_returns_422() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let writer = Uuid::new_v4();
    let db = fixture.db();
    let project = DbFixtures::project()
        .personal(owner)
        .private()
        .active()
        .name(format!("SuspendTwice-{}", Uuid::new_v4()))
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
        .direct(project_id, writer, owner)
        .commit(db)
        .await
        .expect("writer");
    openfga
        .allow_all(Subject::new(owner), project_resource(project_id))
        .await
        .expect("owner tuples");
    openfga
        .allow(
            Subject::new(writer),
            Permission::Write,
            project_resource(project_id),
        )
        .await
        .expect("writer write");
    openfga
        .allow(
            Subject::new(writer),
            Permission::Read,
            project_resource(project_id),
        )
        .await
        .expect("writer read");

    let owner_jwt = jwt(owner);
    let suspended = client
        .post(format!("{base_url}/api/projects/{project_id}/suspend"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .send()
        .await
        .expect("suspend");
    assert_eq!(suspended.status(), StatusCode::OK);

    let again = client
        .post(format!("{base_url}/api/projects/{project_id}/suspend"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .send()
        .await
        .expect("suspend again");
    assert_eq!(again.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let writer_get = client
        .get(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {}", jwt(writer)))
        .send()
        .await
        .expect("writer get");
    assert_eq!(writer_get.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn removed_member_is_denied_immediately() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let writer = Uuid::new_v4();
    let db = fixture.db();
    let project = DbFixtures::project()
        .personal(owner)
        .private()
        .active()
        .name(format!("CutOff-{}", Uuid::new_v4()))
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
        .direct(project_id, writer, owner)
        .commit(db)
        .await
        .expect("writer");
    openfga
        .allow_all(Subject::new(owner), project_resource(project_id))
        .await
        .expect("owner tuples");
    openfga
        .allow(
            Subject::new(writer),
            Permission::Write,
            project_resource(project_id),
        )
        .await
        .expect("stale write tuple");

    let removed = client
        .delete(format!(
            "{base_url}/api/projects/{project_id}/members/{writer}"
        ))
        .header("Authorization", format!("Bearer {}", jwt(owner)))
        .send()
        .await
        .expect("remove");
    assert_eq!(removed.status(), StatusCode::NO_CONTENT);

    let put = client
        .put(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {}", jwt(writer)))
        .header("Content-Type", "application/json")
        .json(&json!({ "name": "should-fail" }))
        .send()
        .await
        .expect("put");
    assert_eq!(put.status(), StatusCode::FORBIDDEN);
}

#[test]
fn openspecs_declare_security_and_error_codes() {
    let spec = include_str!("../openspecs.yaml");
    assert!(spec.starts_with("openapi: 3.0.3"));
    assert!(spec.contains("paths:"));
    assert!(spec.contains("bearerAuth"));
    assert!(spec.contains("security:\n  - bearerAuth: []"));
    assert!(spec.contains("/api/projects/{projectId}/join"));
    assert!(spec.contains("Unprocessable"));
    assert!(spec.contains("409:"));
    assert!(spec.contains("403:"));
    assert!(spec.contains("422:"));
    assert!(spec.contains("public active project"));
}

async fn grant_owner_tuples(openfga: &TestOpenFga, user_id: Uuid, project_id: Uuid) {
    openfga
        .allow_all(Subject::new(user_id), project_resource(project_id))
        .await
        .expect("owner tuples");
}

#[tokio::test]
#[serial]
async fn delete_published_project_returns_204() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let owner_jwt = jwt(owner);
    let created = client
        .post(format!("{base_url}/api/projects"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "name": format!("DeletePublished-{}", Uuid::new_v4()),
            "owner_type": "personal"
        }))
        .send()
        .await
        .expect("create");
    assert_eq!(created.status(), StatusCode::CREATED);
    let project_id = Uuid::parse_str(
        created.json::<Value>().await.expect("body")["id"]
            .as_str()
            .expect("id"),
    )
    .expect("uuid");
    grant_owner_tuples(&openfga, owner, project_id).await;

    let added = client
        .post(format!("{base_url}/api/projects/{project_id}/components"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "component_type": "taskboard" }))
        .send()
        .await
        .expect("add component");
    assert_eq!(added.status(), StatusCode::CREATED);
    let component_id = Uuid::parse_str(
        added.json::<Value>().await.expect("body")["id"]
            .as_str()
            .expect("id"),
    )
    .expect("uuid");
    for status in ["configured", "active"] {
        let patch = client
            .patch(format!(
                "{base_url}/api/projects/{project_id}/components/{component_id}"
            ))
            .header("Authorization", format!("Bearer {owner_jwt}"))
            .header("Content-Type", "application/json")
            .json(&json!({ "status": status }))
            .send()
            .await
            .expect("patch");
        assert_eq!(patch.status(), StatusCode::OK);
    }
    let published = client
        .post(format!("{base_url}/api/projects/{project_id}/publish"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .send()
        .await
        .expect("publish");
    assert_eq!(published.status(), StatusCode::OK);

    let deleted = client
        .delete(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .send()
        .await
        .expect("delete");
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
#[serial]
async fn concurrent_puts_one_conflicts() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let owner_jwt = jwt(owner);
    let created = client
        .post(format!("{base_url}/api/projects"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "name": format!("ConcurrentPut-{}", Uuid::new_v4()),
            "owner_type": "personal"
        }))
        .send()
        .await
        .expect("create");
    assert_eq!(created.status(), StatusCode::CREATED);
    let project_id = Uuid::parse_str(
        created.json::<Value>().await.expect("body")["id"]
            .as_str()
            .expect("id"),
    )
    .expect("uuid");
    grant_owner_tuples(&openfga, owner, project_id).await;

    let first = client
        .put(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "name": "concurrent-a" }))
        .send();
    let second = client
        .put(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "name": "concurrent-b" }))
        .send();
    let (first, second) = tokio::join!(first, second);
    let statuses = [
        first.expect("first").status(),
        second.expect("second").status(),
    ];
    let oks = statuses.iter().filter(|s| **s == StatusCode::OK).count();
    let conflicts = statuses
        .iter()
        .filter(|s| **s == StatusCode::CONFLICT)
        .count();
    assert!(oks >= 1);
    assert_eq!(oks + conflicts, 2);
}

#[tokio::test]
#[serial]
async fn concurrent_remove_and_grant_do_not_500() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let member = Uuid::new_v4();
    let db = fixture.db();
    let project = DbFixtures::project()
        .personal(owner)
        .private()
        .active()
        .name(format!("ConcurrentMember-{}", Uuid::new_v4()))
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
        .commit(db)
        .await
        .expect("member");
    grant_owner_tuples(&openfga, owner, project_id).await;

    let remove = client
        .delete(format!(
            "{base_url}/api/projects/{project_id}/members/{member}"
        ))
        .header("Authorization", format!("Bearer {}", jwt(owner)))
        .send();
    let grant = client
        .post(format!(
            "{base_url}/api/projects/{project_id}/members/{member}/permissions/component"
        ))
        .header("Authorization", format!("Bearer {}", jwt(owner)))
        .header("Content-Type", "application/json")
        .json(&json!({ "permission": "read" }))
        .send();
    let (remove, grant) = tokio::join!(remove, grant);
    let remove_status = remove.expect("remove").status();
    let grant_status = grant.expect("grant").status();
    assert_ne!(remove_status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_ne!(grant_status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        remove_status == StatusCode::NO_CONTENT
            || grant_status == StatusCode::OK
            || grant_status == StatusCode::FORBIDDEN
            || grant_status == StatusCode::NOT_FOUND
            || grant_status == StatusCode::CONFLICT
    );
}

#[tokio::test]
#[serial]
async fn suspended_matrix_hides_project_from_writer() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let writer = Uuid::new_v4();
    let db = fixture.db();
    let project = DbFixtures::project()
        .personal(owner)
        .private()
        .active()
        .name(format!("SuspendedMatrix-{}", Uuid::new_v4()))
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
        .direct(project_id, writer, owner)
        .with_permission("write")
        .commit(db)
        .await
        .expect("writer");
    grant_owner_tuples(&openfga, owner, project_id).await;
    openfga
        .allow(
            Subject::new(writer),
            Permission::Read,
            project_resource(project_id),
        )
        .await
        .expect("writer read");
    openfga
        .allow(
            Subject::new(writer),
            Permission::Write,
            project_resource(project_id),
        )
        .await
        .expect("writer write");

    let owner_jwt = jwt(owner);
    let writer_jwt = jwt(writer);
    let suspended = client
        .post(format!("{base_url}/api/projects/{project_id}/suspend"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .send()
        .await
        .expect("suspend");
    assert_eq!(suspended.status(), StatusCode::OK);

    let listed = client
        .get(format!("{base_url}/api/projects"))
        .header("Authorization", format!("Bearer {writer_jwt}"))
        .send()
        .await
        .expect("list");
    assert_eq!(listed.status(), StatusCode::OK);
    let body: Value = listed.json().await.expect("list body");
    let found = body["data"]
        .as_array()
        .expect("data")
        .iter()
        .any(|row| row["id"] == project_id.to_string());
    assert!(!found);

    let details = client
        .get(format!("{base_url}/api/projects/{project_id}/details"))
        .header("Authorization", format!("Bearer {writer_jwt}"))
        .send()
        .await
        .expect("details");
    assert_eq!(details.status(), StatusCode::FORBIDDEN);

    let members = client
        .get(format!("{base_url}/api/projects/{project_id}/members"))
        .header("Authorization", format!("Bearer {writer_jwt}"))
        .send()
        .await
        .expect("members");
    assert_eq!(members.status(), StatusCode::FORBIDDEN);

    let owner_get = client
        .get(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .send()
        .await
        .expect("owner get");
    assert_eq!(owner_get.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn creator_removed_after_transfer_is_denied() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let successor = Uuid::new_v4();
    let db = fixture.db();
    let project = DbFixtures::project()
        .personal(owner)
        .private()
        .active()
        .name(format!("CreatorCut-{}", Uuid::new_v4()))
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
    grant_owner_tuples(&openfga, owner, project_id).await;
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
    grant_owner_tuples(&openfga, successor, project_id).await;

    let removed = client
        .delete(format!(
            "{base_url}/api/projects/{project_id}/members/{owner}"
        ))
        .header("Authorization", format!("Bearer {}", jwt(successor)))
        .send()
        .await
        .expect("remove creator");
    assert_eq!(removed.status(), StatusCode::NO_CONTENT);

    let get = client
        .get(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {}", jwt(owner)))
        .send()
        .await
        .expect("creator get");
    assert_eq!(get.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn generic_component_grant_lists_all_instances() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let member = Uuid::new_v4();
    let db = fixture.db();
    let project = DbFixtures::project()
        .personal(owner)
        .private()
        .active()
        .name(format!("GenericComp-{}", Uuid::new_v4()))
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
        .commit(db)
        .await
        .expect("member");
    grant_owner_tuples(&openfga, owner, project_id).await;
    openfga
        .allow(
            Subject::new(member),
            Permission::Read,
            project_resource(project_id),
        )
        .await
        .expect("member read");

    let owner_jwt = jwt(owner);
    let mut ids = Vec::new();
    for component_type in ["taskboard", "wiki"] {
        let created = client
            .post(format!("{base_url}/api/projects/{project_id}/components"))
            .header("Authorization", format!("Bearer {owner_jwt}"))
            .header("Content-Type", "application/json")
            .json(&json!({ "component_type": component_type }))
            .send()
            .await
            .expect("add");
        assert_eq!(created.status(), StatusCode::CREATED);
        ids.push(
            created.json::<Value>().await.expect("body")["id"]
                .as_str()
                .expect("id")
                .to_string(),
        );
    }

    let listed = client
        .get(format!("{base_url}/api/projects/{project_id}/components"))
        .header("Authorization", format!("Bearer {}", jwt(member)))
        .send()
        .await
        .expect("list");
    assert_eq!(listed.status(), StatusCode::OK);
    let body: Value = listed.json().await.expect("list body");
    let listed_ids: Vec<String> = body["data"]
        .as_array()
        .expect("data")
        .iter()
        .filter_map(|c| c["id"].as_str().map(str::to_string))
        .collect();
    for id in ids {
        assert!(listed_ids.contains(&id));
    }
}

#[tokio::test]
#[serial]
async fn postgres_ledger_survives_restart_and_keeps_monotonic_order() {
    let fixture = {
        let (_fixture, _base_url, _client, _openfga, _components) =
            setup_test_server().await.expect("setup");
        _fixture
    };
    let database_url = fixture
        .database
        .as_ref()
        .expect("database")
        .database_url
        .clone();
    let project_id = Uuid::new_v4();
    let first = PostgresEventLedger::connect(&database_url)
        .await
        .expect("ledger");
    assert!(first
        .begin_visibility_change(project_id, 3)
        .await
        .expect("begin 3"));
    first
        .complete_visibility_change(project_id, 3)
        .await
        .expect("complete 3");
    drop(first);

    let restarted = PostgresEventLedger::connect(&database_url)
        .await
        .expect("restart");
    assert!(!restarted
        .begin_visibility_change(project_id, 2)
        .await
        .expect("older"));
    assert!(restarted
        .begin_visibility_change(project_id, 4)
        .await
        .expect("gap"));
}

#[tokio::test]
#[serial]
async fn sentinel_applies_v2_grant_from_flat_payload() {
    let (_fixture, _base_url, _client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let project_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    let ledger = Arc::new(InMemoryEventLedger::new());
    let handler = SyncEventHandler::new(
        vec![Arc::new(ManifestoTranslator::new())],
        ledger,
        OpenFgaWriteClient::new(openfga.client_config()).expect("fga client"),
    );
    let granted = PermissionGrantedEvent::new(
        project_id,
        Uuid::new_v4(),
        user_id,
        "project".into(),
        "read".into(),
        user_id,
        Utc::now(),
    );
    let mut payload = serde_json::to_value(&granted).expect("payload");
    if let Some(object) = payload.as_object_mut() {
        object.insert("event_id".into(), json!(event_id));
        object.insert("event_type".into(), json!("permission_granted"));
        object.insert("aggregate_id".into(), json!(project_id));
    }
    #[derive(Debug)]
    struct FlatEvent {
        event_id: Uuid,
        project_id: Uuid,
        payload: Value,
    }
    impl DomainEvent for FlatEvent {
        fn event_type(&self) -> &str {
            "permission_granted"
        }
        fn event_id(&self) -> Uuid {
            self.event_id
        }
        fn aggregate_id(&self) -> Uuid {
            self.project_id
        }
        fn occurred_at(&self) -> chrono::DateTime<Utc> {
            Utc::now()
        }
        fn version(&self) -> u32 {
            2
        }
        fn to_json(&self) -> Result<String, rustycog::core::error::ServiceError> {
            Ok(self.payload.to_string())
        }
        fn metadata(&self) -> std::collections::HashMap<String, String> {
            std::collections::HashMap::new()
        }
    }
    handler
        .handle_event(Box::new(FlatEvent {
            event_id,
            project_id,
            payload,
        }))
        .await
        .expect("apply grant");
    let tuples = openfga
        .read_tuples(
            Some(&format!("user:{user_id}")),
            Some("viewer"),
            Some(&format!("project:{project_id}")),
        )
        .await
        .expect("read");
    assert!(!tuples.is_empty());
}

#[tokio::test]
#[serial]
async fn reconcile_manifesto_writes_missing_and_deletes_stale_without_touching_org() {
    let (fixture, _base_url, _client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner = Uuid::new_v4();
    let stale_user = Uuid::new_v4();
    let org_id = Uuid::new_v4();
    let db = fixture.db();
    let project = DbFixtures::project()
        .personal(owner)
        .public()
        .active()
        .name(format!("Reconcile-{}", Uuid::new_v4()))
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
        .write_tuple(
            &format!("user:{stale_user}"),
            "viewer",
            &format!("project:{project_id}"),
        )
        .await
        .expect("stale manifesto tuple");
    openfga
        .write_tuple(
            &format!("user:{owner}"),
            "member",
            &format!("organization:{org_id}"),
        )
        .await
        .expect("org tuple must survive");

    let database_url = fixture
        .database
        .as_ref()
        .expect("database")
        .database_url
        .clone();
    let fga = OpenFgaWriteClient::new(openfga.client_config()).expect("write client");
    sentinel_sync::reconcile::reconcile_manifesto(&database_url, &fga)
        .await
        .expect("reconcile");

    let stale = openfga
        .read_tuples(
            Some(&format!("user:{stale_user}")),
            Some("viewer"),
            Some(&format!("project:{project_id}")),
        )
        .await
        .expect("stale");
    assert!(stale.is_empty(), "stale manifesto tuple must be deleted");

    let owner_tuples = openfga
        .read_tuples(
            Some(&format!("user:{owner}")),
            Some("owner"),
            Some(&format!("project:{project_id}")),
        )
        .await
        .expect("owner");
    assert!(
        !owner_tuples.is_empty(),
        "missing owner tuple must be written"
    );

    let wildcard = openfga
        .read_tuples(
            Some("user:*"),
            Some("viewer"),
            Some(&format!("project:{project_id}")),
        )
        .await
        .expect("wildcard");
    assert!(!wildcard.is_empty(), "public active project needs wildcard");

    let org = openfga
        .read_tuples(
            Some(&format!("user:{owner}")),
            Some("member"),
            Some(&format!("organization:{org_id}")),
        )
        .await
        .expect("org");
    assert!(!org.is_empty(), "non-manifesto types must stay");
}
