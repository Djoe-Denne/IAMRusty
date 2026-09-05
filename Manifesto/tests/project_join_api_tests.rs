//! HTTP coverage for `POST /api/projects/{id}/join`.

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

fn create_test_jwt_token(user_id: Uuid) -> String {
    rustycog::testing::http::jwt::create_jwt_token(user_id)
}

fn project_resource(project_id: Uuid) -> ResourceRef {
    ResourceRef::new("project", project_id)
}

#[tokio::test]
#[serial]
async fn org_less_user_joins_public_project_and_can_read_write() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let creator = Uuid::new_v4();
    let joiner = Uuid::new_v4();
    let creator_jwt = create_test_jwt_token(creator);
    let joiner_jwt = create_test_jwt_token(joiner);

    let created = client
        .post(format!("{base_url}/api/projects"))
        .header("Authorization", format!("Bearer {creator_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "name": format!("JoinPublic-{}", Uuid::new_v4()),
            "owner_type": "personal",
            "visibility": "public"
        }))
        .send()
        .await
        .expect("create");
    assert_eq!(created.status(), StatusCode::CREATED);
    let body: Value = created.json().await.expect("create body");
    let project_id = Uuid::parse_str(body["id"].as_str().expect("id")).expect("uuid");

    openfga
        .allow_all(Subject::new(creator), project_resource(project_id))
        .await
        .expect("creator tuples");
    openfga
        .allow_wildcard(Permission::Read, project_resource(project_id))
        .await
        .expect("public wildcard");

    let join = client
        .post(format!("{base_url}/api/projects/{project_id}/join"))
        .header("Authorization", format!("Bearer {joiner_jwt}"))
        .send()
        .await
        .expect("join");
    assert_eq!(join.status(), StatusCode::CREATED);
    let member: Value = join.json().await.expect("join body");
    assert_eq!(member["user_id"], joiner.to_string());
    assert_eq!(member["source"], "invitation");

    openfga
        .allow(
            Subject::new(joiner),
            Permission::Write,
            project_resource(project_id),
        )
        .await
        .expect("synced member write tuple");

    let get = client
        .get(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {joiner_jwt}"))
        .send()
        .await
        .expect("get after join");
    assert_eq!(get.status(), StatusCode::OK);

    let put = client
        .put(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {joiner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "description": "joined" }))
        .send()
        .await
        .expect("put after join");
    assert_eq!(put.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn join_private_or_internal_is_403_and_already_member_is_409() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let stranger = Uuid::new_v4();
    let stranger_jwt = create_test_jwt_token(stranger);

    let (private_project, _member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("private project");
    let internal_project = DbFixtures::project()
        .personal(owner_id)
        .internal()
        .name(format!("JoinInternal-{}", Uuid::new_v4()))
        .commit(db.clone())
        .await
        .expect("internal project");

    let owner_jwt = create_test_jwt_token(owner_id);
    let public = client
        .post(format!("{base_url}/api/projects"))
        .header("Authorization", format!("Bearer {owner_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "name": format!("JoinDup-{}", Uuid::new_v4()),
            "owner_type": "personal",
            "visibility": "public"
        }))
        .send()
        .await
        .expect("create public");
    assert_eq!(public.status(), StatusCode::CREATED);
    let public_body: Value = public.json().await.expect("public body");
    let public_id = Uuid::parse_str(public_body["id"].as_str().expect("id")).expect("uuid");
    openfga
        .allow_all(Subject::new(owner_id), project_resource(public_id))
        .await
        .expect("owner tuples");
    openfga
        .allow_wildcard(Permission::Read, project_resource(public_id))
        .await
        .expect("wildcard");

    let private_join = client
        .post(format!(
            "{}/api/projects/{}/join",
            base_url,
            private_project.id()
        ))
        .header("Authorization", format!("Bearer {stranger_jwt}"))
        .send()
        .await
        .expect("join private");
    assert_eq!(private_join.status(), StatusCode::FORBIDDEN);

    let internal_join = client
        .post(format!(
            "{}/api/projects/{}/join",
            base_url,
            internal_project.id()
        ))
        .header("Authorization", format!("Bearer {stranger_jwt}"))
        .send()
        .await
        .expect("join internal");
    assert_eq!(internal_join.status(), StatusCode::FORBIDDEN);

    let first_join = client
        .post(format!("{base_url}/api/projects/{public_id}/join"))
        .header("Authorization", format!("Bearer {stranger_jwt}"))
        .send()
        .await
        .expect("first join");
    assert_eq!(first_join.status(), StatusCode::CREATED);

    let second_join = client
        .post(format!("{base_url}/api/projects/{public_id}/join"))
        .header("Authorization", format!("Bearer {stranger_jwt}"))
        .send()
        .await
        .expect("second join");
    assert_eq!(second_join.status(), StatusCode::CONFLICT);
}
