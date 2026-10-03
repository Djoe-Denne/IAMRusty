//! G1 — writer managed écrit digest + declared_capabilities depuis le catalogue.
//!
//! Pas de seed SQL. Fail-closed si `io.aiforall.reference-kv` sans digest.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use rustycog::permission::{Permission, ResourceRef, Subject};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;
use uuid::Uuid;

const M1_DESCRIPTOR: &str =
    "sha256:98ba747fc572de29d76dfd08f92537782bf0353b652adcace10200020ee560cf";

fn create_test_jwt_token(user_id: Uuid) -> String {
    rustycog::testing::http::jwt::create_jwt_token(user_id)
}

#[tokio::test]
#[serial]
async fn g1_reference_kv_inserts_catalog_digest_and_declared() {
    let (fixture, base_url, client, openfga, components) =
        setup_test_server().await.expect("setup_test_server");
    let db = fixture.db();
    components.reset().await;
    components
        .mock_list_components(vec![
            ComponentInfoBody::reference_kv(M1_DESCRIPTOR),
            ComponentInfoBody::new("taskboard"),
            ComponentInfoBody::new("wiki"),
        ])
        .await;

    let owner_id = Uuid::new_v4();
    let (project, _member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("projet");
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("grant admin");

    let jwt = create_test_jwt_token(owner_id);
    let resp = client
        .post(format!(
            "{}/api/projects/{}/components",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"component_type": "io.aiforall.reference-kv"}))
        .send()
        .await
        .expect("POST component");
    let status = resp.status();
    let created: serde_json::Value = resp.json().await.expect("JSON");
    assert_eq!(status, 201, "{created}");
    let component_id = Uuid::parse_str(created["id"].as_str().expect("id")).expect("uuid");

    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT digest, declared_capabilities::text AS declared \
             FROM apparatus_bindings WHERE component_id = $1",
            [component_id.into()],
        ))
        .await
        .expect("lecture binding")
        .expect("ligne apparatus_bindings");
    let digest: Option<String> = row.try_get("", "digest").expect("digest");
    let declared: String = row.try_get("", "declared").expect("declared");
    assert_eq!(digest.as_deref(), Some(M1_DESCRIPTOR));
    assert!(declared.contains("storage.kv.read"), "{declared}");
    assert!(declared.contains("storage.kv.write"), "{declared}");
    assert!(declared.contains("project.read"), "{declared}");
}

#[tokio::test]
#[serial]
async fn g1_reference_kv_without_digest_does_not_insert() {
    let (fixture, base_url, client, openfga, components) =
        setup_test_server().await.expect("setup_test_server");
    let db = fixture.db();
    components.reset().await;
    let mut bare = ComponentInfoBody::new("io.aiforall.reference-kv");
    bare.digest = None;
    components
        .mock_list_components(vec![bare, ComponentInfoBody::new("wiki")])
        .await;

    let owner_id = Uuid::new_v4();
    let (project, _member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("projet");
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("grant admin");

    let jwt = create_test_jwt_token(owner_id);
    let resp = client
        .post(format!(
            "{}/api/projects/{}/components",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"component_type": "io.aiforall.reference-kv"}))
        .send()
        .await
        .expect("POST component");
    assert!(
        resp.status().is_client_error() || resp.status().is_server_error(),
        "fail-closed expected, got {}",
        resp.status()
    );

    let count = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS n FROM apparatus_bindings ab \
             INNER JOIN project_components pc ON pc.id = ab.component_id \
             WHERE pc.project_id = $1",
            [project.id().into()],
        ))
        .await
        .expect("count")
        .expect("row");
    let n: i64 = count.try_get("", "n").expect("n");
    assert_eq!(n, 0, "pas d'INSERT NULL pour reference-kv sans digest");
}

#[tokio::test]
#[serial]
async fn g1_wiki_without_digest_still_inserts_binding() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("setup_test_server");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .expect("projet");
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("grant admin");

    let jwt = create_test_jwt_token(owner_id);
    let resp = client
        .post(format!(
            "{}/api/projects/{}/components",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"component_type": "wiki"}))
        .send()
        .await
        .expect("POST component");
    assert_eq!(resp.status(), 201);
    let created: serde_json::Value = resp.json().await.expect("JSON");
    let component_id = Uuid::parse_str(created["id"].as_str().expect("id")).expect("uuid");
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT digest FROM apparatus_bindings WHERE component_id = $1",
            [component_id.into()],
        ))
        .await
        .expect("lecture")
        .expect("binding wiki");
    let digest: Option<String> = row.try_get("", "digest").expect("digest");
    assert!(digest.is_none(), "wiki sans digest catalogue → digest NULL");
}
