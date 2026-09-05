//! HTTP + real OpenFGA coverage for public-read vs publish vs visibility flips.
//!
//! Manifesto does not write tuples itself — `sentinel-sync` does. These tests
//! therefore (1) assert the store stays free of `viewer@user:*` after create /
//! publish / PUT, (2) apply the wildcard sentinel-sync would write so public
//! GET can pass middleware, and (3) prove GET/details fail-closed on a leftover
//! wildcard once the project is no longer world-readable.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use reqwest::{Client, StatusCode};
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

async fn has_public_wildcard(openfga: &TestOpenFga, project_id: Uuid) -> bool {
    let object = format!("project:{project_id}");
    let tuples = openfga
        .read_tuples(Some("user:*"), Some("viewer"), Some(object.as_str()))
        .await
        .expect("OpenFGA read should succeed");
    !tuples.is_empty()
}

async fn create_personal_project(
    client: &Client,
    base_url: &str,
    jwt_token: &str,
    visibility: &str,
) -> Value {
    let response = client
        .post(format!("{base_url}/api/projects"))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "name": format!("VisAcl-{visibility}-{}", Uuid::new_v4()),
            "owner_type": "personal",
            "visibility": visibility
        }))
        .send()
        .await
        .expect("create project request");
    assert_eq!(response.status(), StatusCode::CREATED, "create should 201");
    response.json().await.expect("create body")
}

async fn grant_creator(openfga: &TestOpenFga, user_id: Uuid, project_id: Uuid) {
    openfga
        .allow_all(Subject::new(user_id), project_resource(project_id))
        .await
        .expect("bootstrap creator tuples");
}

async fn grant_write(openfga: &TestOpenFga, user_id: Uuid, project_id: Uuid) {
    openfga
        .allow(
            Subject::new(user_id),
            Permission::Write,
            project_resource(project_id),
        )
        .await
        .expect("write-only grant");
}

async fn anonymous_get(client: &Client, base_url: &str, project_id: Uuid) -> StatusCode {
    client
        .get(format!("{base_url}/api/projects/{project_id}"))
        .send()
        .await
        .expect("anonymous GET")
        .status()
}

async fn anonymous_details(client: &Client, base_url: &str, project_id: Uuid) -> StatusCode {
    client
        .get(format!("{base_url}/api/projects/{project_id}/details"))
        .send()
        .await
        .expect("anonymous GET details")
        .status()
}

async fn anonymous_list_contains(
    client: &Client,
    base_url: &str,
    project_id: Uuid,
    name: &str,
) -> bool {
    let listed = client
        .get(format!("{base_url}/api/projects"))
        .query(&[("search", name)])
        .send()
        .await
        .expect("anonymous list");
    assert_eq!(listed.status(), StatusCode::OK);
    let list_json: Value = listed.json().await.expect("list body");
    list_json["data"]
        .as_array()
        .expect("list data")
        .iter()
        .any(|row| row["id"] == project_id.to_string())
}

async fn list_components(
    client: &Client,
    base_url: &str,
    project_id: Uuid,
    jwt: Option<&str>,
) -> StatusCode {
    let mut request = client.get(format!("{base_url}/api/projects/{project_id}/components"));
    if let Some(token) = jwt {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    request.send().await.expect("list components").status()
}

async fn get_component(
    client: &Client,
    base_url: &str,
    project_id: Uuid,
    component_id: Uuid,
    jwt: Option<&str>,
) -> StatusCode {
    let mut request = client.get(format!(
        "{base_url}/api/projects/{project_id}/components/{component_id}"
    ));
    if let Some(token) = jwt {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    request.send().await.expect("get component").status()
}

async fn add_taskboard(client: &Client, base_url: &str, jwt_token: &str, project_id: Uuid) -> Uuid {
    let add = client
        .post(format!("{base_url}/api/projects/{project_id}/components"))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "component_type": "taskboard" }))
        .send()
        .await
        .expect("add component");
    assert_eq!(
        add.status(),
        StatusCode::CREATED,
        "add component should 201"
    );
    let added: Value = add.json().await.expect("add body");
    Uuid::parse_str(added["id"].as_str().expect("component id")).expect("component uuid")
}

async fn put_visibility(
    client: &Client,
    base_url: &str,
    jwt_token: &str,
    project_id: Uuid,
    visibility: &str,
) -> Value {
    let response = client
        .put(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "visibility": visibility }))
        .send()
        .await
        .expect("PUT visibility");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "PUT visibility should 200"
    );
    let body: Value = response.json().await.expect("PUT body");
    assert_eq!(body["visibility"], visibility);
    body
}

async fn make_publishable(
    client: &Client,
    base_url: &str,
    jwt_token: &str,
    project_id: Uuid,
) -> Uuid {
    let component_id = add_taskboard(client, base_url, jwt_token, project_id).await;

    for status in ["configured", "active"] {
        let patch = client
            .patch(format!(
                "{base_url}/api/projects/{project_id}/components/{component_id}"
            ))
            .header("Authorization", format!("Bearer {jwt_token}"))
            .header("Content-Type", "application/json")
            .json(&json!({ "status": status }))
            .send()
            .await
            .expect("patch component");
        assert_eq!(
            patch.status(),
            StatusCode::OK,
            "component status {status} should 200"
        );
    }
    component_id
}

#[tokio::test]
#[serial]
async fn create_private_has_no_wildcard_and_anonymous_get_is_403() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let user_id = Uuid::new_v4();
    let jwt = create_test_jwt_token(user_id);
    let created = create_personal_project(&client, &base_url, &jwt, "private").await;
    let project_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();

    assert!(
        !has_public_wildcard(&openfga, project_id).await,
        "create-as-private must not write viewer@user:*"
    );
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        anonymous_details(&client, &base_url, project_id).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
#[serial]
async fn create_public_does_not_write_wildcard_until_sync_then_anonymous_get_200() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let user_id = Uuid::new_v4();
    let jwt = create_test_jwt_token(user_id);
    let created = create_personal_project(&client, &base_url, &jwt, "public").await;
    let project_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    let name = created["name"].as_str().unwrap();

    assert_eq!(created["visibility"], "public");
    assert!(
        !has_public_wildcard(&openfga, project_id).await,
        "HTTP create does not write FGA; sentinel-sync is not in this process"
    );
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::FORBIDDEN
    );
    assert!(
        anonymous_list_contains(&client, &base_url, project_id, name).await,
        "SQL list shows public rows even before wildcard; GET needs the tuple"
    );

    openfga
        .allow_wildcard(Permission::Read, project_resource(project_id))
        .await
        .expect("simulate ProjectCreated public arm");

    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::OK
    );
    assert_eq!(
        anonymous_details(&client, &base_url, project_id).await,
        StatusCode::OK
    );
}

#[tokio::test]
#[serial]
async fn stranger_can_read_public_with_wildcard_but_cannot_write() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner_id = Uuid::new_v4();
    let stranger_id = Uuid::new_v4();
    let owner_jwt = create_test_jwt_token(owner_id);
    let stranger_jwt = create_test_jwt_token(stranger_id);
    let created = create_personal_project(&client, &base_url, &owner_jwt, "public").await;
    let project_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    grant_creator(&openfga, owner_id, project_id).await;
    openfga
        .allow_wildcard(Permission::Read, project_resource(project_id))
        .await
        .expect("public wildcard");

    let read = client
        .get(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {stranger_jwt}"))
        .send()
        .await
        .expect("stranger GET");
    assert_eq!(read.status(), StatusCode::OK);

    let write = client
        .put(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {stranger_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "name": "Hijacked" }))
        .send()
        .await
        .expect("stranger PUT");
    assert_eq!(write.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn write_member_can_flip_private_internal_but_not_public() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner_id = Uuid::new_v4();
    let writer_id = Uuid::new_v4();
    let owner_jwt = create_test_jwt_token(owner_id);
    let writer_jwt = create_test_jwt_token(writer_id);
    let created = create_personal_project(&client, &base_url, &owner_jwt, "private").await;
    let project_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    grant_write(&openfga, writer_id, project_id).await;

    put_visibility(&client, &base_url, &writer_jwt, project_id, "internal").await;

    let response = client
        .put(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {writer_jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "visibility": "public" }))
        .send()
        .await
        .expect("PUT public");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(!has_public_wildcard(&openfga, project_id).await);
}

#[tokio::test]
#[serial]
async fn put_visibility_without_write_is_403() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let owner_id = Uuid::new_v4();
    let jwt = create_test_jwt_token(owner_id);
    let created = create_personal_project(&client, &base_url, &jwt, "private").await;
    let project_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Read,
            project_resource(project_id),
        )
        .await
        .expect("read-only grant");

    let response = client
        .put(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&json!({ "visibility": "public" }))
        .send()
        .await
        .expect("PUT");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(!has_public_wildcard(&openfga, project_id).await);
}

#[tokio::test]
#[serial]
async fn publish_private_does_not_write_wildcard() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let user_id = Uuid::new_v4();
    let jwt = create_test_jwt_token(user_id);
    let created = create_personal_project(&client, &base_url, &jwt, "private").await;
    let project_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    grant_creator(&openfga, user_id, project_id).await;
    make_publishable(&client, &base_url, &jwt, project_id).await;

    let publish = client
        .post(format!("{base_url}/api/projects/{project_id}/publish"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("publish");
    assert_eq!(publish.status(), StatusCode::OK);
    let body: Value = publish.json().await.expect("publish body");
    assert_eq!(body["status"], "active");
    assert_eq!(body["visibility"], "private");

    assert!(
        !has_public_wildcard(&openfga, project_id).await,
        "publish must not write viewer@user:*"
    );
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
#[serial]
async fn put_to_public_does_not_write_wildcard_until_sync() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let user_id = Uuid::new_v4();
    let jwt = create_test_jwt_token(user_id);
    let created = create_personal_project(&client, &base_url, &jwt, "private").await;
    let project_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    grant_creator(&openfga, user_id, project_id).await;

    put_visibility(&client, &base_url, &jwt, project_id, "public").await;
    assert!(
        !has_public_wildcard(&openfga, project_id).await,
        "PUT visibility does not write FGA in-process"
    );
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::FORBIDDEN
    );

    openfga
        .allow_wildcard(Permission::Read, project_resource(project_id))
        .await
        .expect("simulate ProjectVisibilityChanged to public");
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::OK
    );
}

#[tokio::test]
#[serial]
async fn put_private_to_internal_never_writes_wildcard() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let user_id = Uuid::new_v4();
    let jwt = create_test_jwt_token(user_id);
    let created = create_personal_project(&client, &base_url, &jwt, "private").await;
    let project_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    grant_creator(&openfga, user_id, project_id).await;

    put_visibility(&client, &base_url, &jwt, project_id, "internal").await;
    assert!(!has_public_wildcard(&openfga, project_id).await);
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
#[serial]
async fn put_same_public_visibility_keeps_existing_wildcard() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let user_id = Uuid::new_v4();
    let jwt = create_test_jwt_token(user_id);
    let created = create_personal_project(&client, &base_url, &jwt, "public").await;
    let project_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    grant_creator(&openfga, user_id, project_id).await;
    openfga
        .allow_wildcard(Permission::Read, project_resource(project_id))
        .await
        .expect("create-as-public wildcard");

    put_visibility(&client, &base_url, &jwt, project_id, "public").await;
    assert!(has_public_wildcard(&openfga, project_id).await);
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::OK
    );
}

#[tokio::test]
#[serial]
async fn leftover_wildcard_does_not_keep_non_public_readable() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let user_id = Uuid::new_v4();
    let stranger_id = Uuid::new_v4();
    let jwt = create_test_jwt_token(user_id);
    let stranger_jwt = create_test_jwt_token(stranger_id);
    let created = create_personal_project(&client, &base_url, &jwt, "public").await;
    let project_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    grant_creator(&openfga, user_id, project_id).await;
    let component_id = add_taskboard(&client, &base_url, &jwt, project_id).await;
    openfga
        .allow_wildcard(Permission::Read, project_resource(project_id))
        .await
        .expect("create-as-public wildcard");
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::OK
    );
    assert_eq!(
        list_components(&client, &base_url, project_id, None).await,
        StatusCode::OK
    );

    put_visibility(&client, &base_url, &jwt, project_id, "private").await;
    assert!(
        has_public_wildcard(&openfga, project_id).await,
        "PUT does not delete FGA; GET must still fail-closed"
    );
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        anonymous_details(&client, &base_url, project_id).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        list_components(&client, &base_url, project_id, None).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get_component(&client, &base_url, project_id, component_id, None).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        list_components(&client, &base_url, project_id, Some(&stranger_jwt)).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get_component(
            &client,
            &base_url,
            project_id,
            component_id,
            Some(&stranger_jwt)
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        list_components(&client, &base_url, project_id, Some(&jwt)).await,
        StatusCode::OK
    );
    assert_eq!(
        get_component(&client, &base_url, project_id, component_id, Some(&jwt)).await,
        StatusCode::OK
    );

    let stranger_read = client
        .get(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {stranger_jwt}"))
        .send()
        .await
        .expect("stranger GET");
    assert_eq!(stranger_read.status(), StatusCode::FORBIDDEN);

    let owner_read = client
        .get(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("owner GET");
    assert_eq!(owner_read.status(), StatusCode::OK);

    put_visibility(&client, &base_url, &jwt, project_id, "public").await;
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::OK
    );
    assert_eq!(
        list_components(&client, &base_url, project_id, None).await,
        StatusCode::OK
    );

    put_visibility(&client, &base_url, &jwt, project_id, "internal").await;
    assert!(has_public_wildcard(&openfga, project_id).await);
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        list_components(&client, &base_url, project_id, None).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get_component(&client, &base_url, project_id, component_id, None).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
#[serial]
async fn archive_public_does_not_delete_wildcard_until_sync() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let user_id = Uuid::new_v4();
    let jwt = create_test_jwt_token(user_id);
    let created = create_personal_project(&client, &base_url, &jwt, "public").await;
    let project_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    let name = created["name"].as_str().unwrap();
    grant_creator(&openfga, user_id, project_id).await;
    let component_id = make_publishable(&client, &base_url, &jwt, project_id).await;

    let publish = client
        .post(format!("{base_url}/api/projects/{project_id}/publish"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("publish");
    assert_eq!(publish.status(), StatusCode::OK);

    openfga
        .allow_wildcard(Permission::Read, project_resource(project_id))
        .await
        .expect("public wildcard");

    let archive = client
        .post(format!("{base_url}/api/projects/{project_id}/archive"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("archive");
    assert_eq!(archive.status(), StatusCode::OK);
    let archived: Value = archive.json().await.expect("archive body");
    assert_eq!(archived["status"], "archived");

    assert!(
        has_public_wildcard(&openfga, project_id).await,
        "archive HTTP path does not delete FGA; GET must still fail-closed"
    );
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        anonymous_details(&client, &base_url, project_id).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        list_components(&client, &base_url, project_id, None).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get_component(&client, &base_url, project_id, component_id, None).await,
        StatusCode::FORBIDDEN
    );
    assert!(
        !anonymous_list_contains(&client, &base_url, project_id, name).await,
        "anonymous list must hide public+archived rows"
    );
}

#[tokio::test]
#[serial]
async fn anonymous_list_shows_public_sql_row_while_get_needs_wildcard() {
    let (_fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let user_id = Uuid::new_v4();
    let jwt = create_test_jwt_token(user_id);
    let created = create_personal_project(&client, &base_url, &jwt, "public").await;
    let project_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    let name = created["name"].as_str().unwrap();

    assert!(
        anonymous_list_contains(&client, &base_url, project_id, name).await,
        "list SQL includes visibility=public without FGA"
    );
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::FORBIDDEN
    );

    openfga
        .allow_wildcard(Permission::Read, project_resource(project_id))
        .await
        .expect("wildcard");
    assert_eq!(
        anonymous_get(&client, &base_url, project_id).await,
        StatusCode::OK
    );
}

async fn authenticated_get(
    client: &Client,
    base_url: &str,
    jwt: &str,
    project_id: Uuid,
) -> StatusCode {
    client
        .get(format!("{base_url}/api/projects/{project_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("authenticated GET")
        .status()
}

async fn authenticated_list_contains(
    client: &Client,
    base_url: &str,
    jwt: &str,
    project_id: Uuid,
    name: &str,
) -> bool {
    let listed = client
        .get(format!("{base_url}/api/projects"))
        .header("Authorization", format!("Bearer {jwt}"))
        .query(&[("search", name)])
        .send()
        .await
        .expect("authenticated list");
    assert_eq!(listed.status(), StatusCode::OK);
    let list_json: Value = listed.json().await.expect("list body");
    list_json["data"]
        .as_array()
        .expect("list data")
        .iter()
        .any(|row| row["id"] == project_id.to_string())
}

async fn seed_org_project(
    db: std::sync::Arc<sea_orm::DatabaseConnection>,
    org_id: Uuid,
    creator: Uuid,
    visibility: &str,
    name: &str,
) -> Uuid {
    let fixture = fixtures::DbFixtures::project()
        .organization(org_id, creator)
        .visibility(visibility)
        .name(name)
        .commit(db)
        .await
        .expect("seed org project");
    fixture.id()
}

async fn grant_org_member(openfga: &TestOpenFga, user_id: Uuid, org_id: Uuid) {
    openfga
        .write_tuple(
            &format!("user:{user_id}"),
            "member",
            &format!("organization:{org_id}"),
        )
        .await
        .expect("org member tuple");
}

async fn grant_org_admin(openfga: &TestOpenFga, user_id: Uuid, org_id: Uuid) {
    openfga
        .allow(
            Subject::new(user_id),
            Permission::Admin,
            ResourceRef::new("organization", org_id),
        )
        .await
        .expect("org admin tuple");
}

async fn link_project_to_org(openfga: &TestOpenFga, project_id: Uuid, org_id: Uuid) {
    openfga
        .write_tuple(
            &format!("organization:{org_id}"),
            "organization",
            &format!("project:{project_id}"),
        )
        .await
        .expect("project organization parent");
}

async fn grant_internal_org_viewer(openfga: &TestOpenFga, project_id: Uuid, org_id: Uuid) {
    openfga
        .write_tuple(
            &format!("organization:{org_id}#member"),
            "viewer",
            &format!("project:{project_id}"),
        )
        .await
        .expect("internal org viewer userset");
}

#[tokio::test]
#[serial]
async fn org_member_gets_internal_but_not_private_and_anonymous_internal_is_403() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let db = fixture.db();
    let org_id = Uuid::new_v4();
    let creator = Uuid::new_v4();
    let org_member = Uuid::new_v4();
    let internal_name = format!("OrgInternal-{}", Uuid::new_v4());
    let private_name = format!("OrgPrivate-{}", Uuid::new_v4());
    let internal_id =
        seed_org_project(db.clone(), org_id, creator, "internal", &internal_name).await;
    let private_id = seed_org_project(db.clone(), org_id, creator, "private", &private_name).await;

    grant_org_member(&openfga, org_member, org_id).await;
    link_project_to_org(&openfga, internal_id, org_id).await;
    link_project_to_org(&openfga, private_id, org_id).await;
    grant_internal_org_viewer(&openfga, internal_id, org_id).await;

    let jwt = create_test_jwt_token(org_member);
    assert_eq!(
        authenticated_get(&client, &base_url, &jwt, internal_id).await,
        StatusCode::OK
    );
    assert_eq!(
        authenticated_get(&client, &base_url, &jwt, private_id).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        anonymous_get(&client, &base_url, internal_id).await,
        StatusCode::FORBIDDEN
    );
    assert!(
        authenticated_list_contains(&client, &base_url, &jwt, internal_id, &internal_name).await,
        "org member must see internal in list"
    );
    assert!(
        !authenticated_list_contains(&client, &base_url, &jwt, private_id, &private_name).await,
        "org member must not see private in list"
    );
}

#[tokio::test]
#[serial]
async fn org_admin_gets_private_org_owned_project() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let db = fixture.db();
    let org_id = Uuid::new_v4();
    let creator = Uuid::new_v4();
    let org_admin = Uuid::new_v4();
    let name = format!("OrgAdminPrivate-{}", Uuid::new_v4());
    let project_id = seed_org_project(db.clone(), org_id, creator, "private", &name).await;

    grant_org_admin(&openfga, org_admin, org_id).await;
    link_project_to_org(&openfga, project_id, org_id).await;

    let jwt = create_test_jwt_token(org_admin);
    assert_eq!(
        authenticated_get(&client, &base_url, &jwt, project_id).await,
        StatusCode::OK
    );
    assert!(
        authenticated_list_contains(&client, &base_url, &jwt, project_id, &name).await,
        "org admin must see private in list"
    );
}
