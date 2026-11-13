//! Coverage90 QA: real HTTP/PG/OpenFGA membership transitions, not mocked use cases.
//! Oracles: docs/functional/projet.md (membership/admin boundary), persisted
//! grants and pagination contract. Queue stays disabled in the standard harness.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::DbFixtures;
use reqwest::{Client, StatusCode};
use rustycog::permission::{Permission, ResourceRef, Subject};
use rustycog::testing::http::jwt::create_jwt_token;
use serde_json::{json, Value};
use serial_test::serial;
use std::collections::BTreeSet;
use uuid::Uuid;

async fn authorize_owner(openfga: &common::TestOpenFga, owner: Uuid, project: Uuid) {
    for permission in [Permission::Admin, Permission::Read] {
        openfga
            .allow(
                Subject::new(owner),
                permission,
                ResourceRef::new("project", project),
            )
            .await
            .expect("grant owner route permission in real OpenFGA");
    }
}

async fn add_member(client: &Client, url: &str, token: &str, user: Uuid) -> Value {
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(&json!({"user_id": user, "permission": "read", "resource": "project"}))
        .send()
        .await
        .expect("POST member");
    assert_eq!(response.status(), StatusCode::CREATED);
    response.json().await.expect("created member JSON")
}

async fn read_member(client: &Client, url: &str, token: &str) -> Value {
    let response = client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .expect("GET member");
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.expect("persisted member JSON")
}

fn permissions(member: &Value) -> BTreeSet<(String, String)> {
    let rows = member["permissions"].as_array().expect("permissions array");
    let grants: BTreeSet<_> = rows
        .iter()
        .map(|grant| {
            (
                grant["resource"]
                    .as_str()
                    .expect("grant resource")
                    .to_owned(),
                grant["permission"]
                    .as_str()
                    .expect("grant level")
                    .to_owned(),
            )
        })
        .collect();
    assert_eq!(grants.len(), rows.len(), "duplicate member permission");
    grants
}

#[tokio::test]
#[serial]
async fn replacement_removes_old_grants_and_survives_an_independent_get() {
    let (fixture, base, client, openfga, _catalog) =
        Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let user = Uuid::new_v4();
    let (project, _) = DbFixtures::create_project_with_owner(&fixture.db(), owner)
        .await
        .unwrap();
    authorize_owner(&openfga, owner, project.id()).await;
    let token = create_jwt_token(owner);
    let members = format!("{base}/api/projects/{}/members", project.id());
    let added = add_member(&client, &members, &token, user).await;
    let member_url = format!("{members}/{user}");

    let updated = client
        .put(&member_url)
        .bearer_auth(&token)
        .json(&json!({"permissions": [
            {"resource": "project", "permission": "write"},
            {"resource": "component", "permission": "read"}
        ]}))
        .send()
        .await
        .unwrap();
    assert_eq!(updated.status(), StatusCode::OK);
    let stored = read_member(&client, &member_url, &token).await;
    assert_eq!(
        stored["id"], added["id"],
        "replacement must preserve membership identity"
    );
    assert_eq!(
        permissions(&stored),
        BTreeSet::from([
            ("project".into(), "write".into()),
            ("component".into(), "read".into()),
        ]),
        "replacement must not merge with obsolete project/read"
    );
}

#[tokio::test]
#[serial]
async fn mixed_replacement_cannot_partially_apply_before_an_owner_grant_is_rejected() {
    let (fixture, base, client, openfga, _catalog) =
        Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let user = Uuid::new_v4();
    let (project, _) = DbFixtures::create_project_with_owner(&fixture.db(), owner)
        .await
        .unwrap();
    authorize_owner(&openfga, owner, project.id()).await;
    let token = create_jwt_token(owner);
    let members = format!("{base}/api/projects/{}/members", project.id());
    let before = add_member(&client, &members, &token, user).await;
    let member_url = format!("{members}/{user}");

    let response = client
        .put(&member_url)
        .bearer_auth(&token)
        .json(&json!({"permissions": [
            {"resource": "project", "permission": "write"},
            {"resource": "component", "permission": "owner"}
        ]}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "ownership requires transfer, not a grant"
    );
    let after = read_member(&client, &member_url, &token).await;
    assert_eq!(after["id"], before["id"]);
    assert_eq!(
        permissions(&after),
        permissions(&before),
        "a later invalid grant must roll back the whole replacement"
    );
}

#[tokio::test]
#[serial]
async fn granting_then_revoking_one_resource_preserves_the_other_persisted_grant() {
    let (fixture, base, client, openfga, _catalog) =
        Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let user = Uuid::new_v4();
    let (project, _) = DbFixtures::create_project_with_owner(&fixture.db(), owner)
        .await
        .unwrap();
    authorize_owner(&openfga, owner, project.id()).await;
    let token = create_jwt_token(owner);
    let members = format!("{base}/api/projects/{}/members", project.id());
    let before = add_member(&client, &members, &token, user).await;
    let member_url = format!("{members}/{user}");
    let grant_url = format!("{member_url}/permissions/component");

    let grant = client
        .post(&grant_url)
        .bearer_auth(&token)
        .json(&json!({"permission": "write"}))
        .send()
        .await
        .unwrap();
    assert_eq!(grant.status(), StatusCode::OK);
    let granted = read_member(&client, &member_url, &token).await;
    assert_eq!(
        permissions(&granted),
        BTreeSet::from([
            ("project".into(), "read".into()),
            ("component".into(), "write".into()),
        ])
    );

    let revoke = client
        .delete(&grant_url)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(revoke.status(), StatusCode::NO_CONTENT);
    let after = read_member(&client, &member_url, &token).await;
    assert_eq!(after["id"], before["id"]);
    assert_eq!(
        permissions(&after),
        permissions(&before),
        "revoke must target only the requested resource"
    );
}

#[tokio::test]
#[serial]
async fn owner_cannot_be_demoted_or_removed_through_member_mutations() {
    let (fixture, base, client, openfga, _catalog) =
        Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let (project, _) = DbFixtures::create_project_with_owner(&fixture.db(), owner)
        .await
        .unwrap();
    authorize_owner(&openfga, owner, project.id()).await;
    let token = create_jwt_token(owner);
    let url = format!("{base}/api/projects/{}/members/{owner}", project.id());
    let before = read_member(&client, &url, &token).await;

    let demote = client
        .put(&url)
        .bearer_auth(&token)
        .json(&json!({"permissions": [{"resource": "project", "permission": "read"}]}))
        .send()
        .await
        .unwrap();
    assert_eq!(demote.status(), StatusCode::FORBIDDEN);
    let remove = client
        .delete(&url)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(remove.status(), StatusCode::FORBIDDEN);
    let after = read_member(&client, &url, &token).await;
    assert_eq!(after["id"], before["id"]);
    assert_eq!(permissions(&after), permissions(&before));
    assert!(
        after["removed_at"].is_null(),
        "rejected removal must not soft-delete the owner"
    );
}

#[tokio::test]
#[serial]
async fn route_admin_does_not_allow_a_read_member_to_escalate_another_member() {
    let (fixture, base, client, openfga, _catalog) =
        Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let actor = Uuid::new_v4();
    let target = Uuid::new_v4();
    let (project, _) = DbFixtures::create_project_with_owner(&fixture.db(), owner)
        .await
        .unwrap();
    authorize_owner(&openfga, owner, project.id()).await;
    let token = create_jwt_token(owner);
    let members = format!("{base}/api/projects/{}/members", project.id());
    add_member(&client, &members, &token, actor).await;
    let before = add_member(&client, &members, &token, target).await;
    // Deliberately pass the route guard; the domain authorization must still apply.
    openfga
        .allow(
            Subject::new(actor),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .unwrap();
    let target_url = format!("{members}/{target}");
    let response = client
        .put(&target_url)
        .bearer_auth(create_jwt_token(actor))
        .json(&json!({"permissions": [{"resource": "project", "permission": "admin"}]}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let after = read_member(&client, &target_url, &token).await;
    assert_eq!(
        permissions(&after),
        permissions(&before),
        "failed escalation must leave persisted grants unchanged"
    );
}

#[tokio::test]
#[serial]
async fn member_cursors_cover_exactly_one_project_without_overlap_or_phantom_pages() {
    let (fixture, base, client, openfga, _catalog) =
        Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let (project, _) = DbFixtures::create_project_with_owner(&fixture.db(), owner)
        .await
        .unwrap();
    let (other_project, _) = DbFixtures::create_project_with_owner(&fixture.db(), Uuid::new_v4())
        .await
        .unwrap();
    authorize_owner(&openfga, owner, project.id()).await;
    let token = create_jwt_token(owner);
    let members = format!("{base}/api/projects/{}/members", project.id());
    let mut expected = BTreeSet::from([owner.to_string()]);
    for _ in 0..4 {
        let user = Uuid::new_v4();
        add_member(&client, &members, &token, user).await;
        expected.insert(user.to_string());
    }
    // An unrelated row must not affect either the result or total_count.
    DbFixtures::member()
        .direct(other_project.id(), Uuid::new_v4(), owner)
        .commit(fixture.db())
        .await
        .unwrap();
    let mut seen = BTreeSet::new();
    let mut cursor = "0".to_owned();
    for (index, expected_len) in [2, 2, 1].into_iter().enumerate() {
        let response = client
            .get(&members)
            .bearer_auth(&token)
            .query(&[("limit", "2"), ("cursor", cursor.as_str())])
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let page: Value = response.json().await.unwrap();
        let rows = page["data"].as_array().expect("member page data");
        assert_eq!(rows.len(), expected_len, "page {index}");
        assert_eq!(page["pagination"]["total_count"], 5);
        for member in rows {
            let user = member["user_id"].as_str().expect("member user").to_owned();
            assert!(seen.insert(user), "cursor pages must not overlap");
        }
        assert_eq!(page["pagination"]["has_more"], index < 2);
        if index < 2 {
            let next = page["pagination"]["next_cursor"]
                .as_str()
                .expect("next page cursor");
            assert_ne!(next, cursor, "cursor must advance");
            cursor = next.to_owned();
        } else {
            assert!(page["pagination"]["next_cursor"].is_null());
        }
    }
    assert_eq!(
        seen, expected,
        "all and only the requested project's members"
    );
}
