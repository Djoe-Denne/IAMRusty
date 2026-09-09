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

const fn project_resource(project_id: Uuid) -> ResourceRef {
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

    let db = _fixture.db();
    let project = DbFixtures::project()
        .personal(creator)
        .public()
        .active()
        .name(format!("JoinPublic-{}", Uuid::new_v4()))
        .commit(db.clone())
        .await
        .expect("public active project");
    let project_id = project.id();
    DbFixtures::member()
        .owner(project_id, creator)
        .commit(db)
        .await
        .expect("owner member");

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
    assert_eq!(member["source"], "direct");
    let permissions = member["permissions"].as_array().expect("permissions");
    assert!(permissions
        .iter()
        .any(|p| { p["resource"] == "project" && p["permission"] == "read" }));

    openfga
        .allow(
            Subject::new(joiner),
            Permission::Read,
            project_resource(project_id),
        )
        .await
        .expect("synced member read tuple");

    let get = client
        .get(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {joiner_jwt}"))
        .send()
        .await
        .expect("get after join");
    assert_eq!(get.status(), StatusCode::OK);
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

    let public = DbFixtures::project()
        .personal(owner_id)
        .public()
        .active()
        .name(format!("JoinDup-{}", Uuid::new_v4()))
        .commit(db.clone())
        .await
        .expect("public active");
    let public_id = public.id();
    DbFixtures::member()
        .owner(public_id, owner_id)
        .commit(db)
        .await
        .expect("owner");
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
