//! Apparatus P1 — T5 ACL et événements (RED, TDD Phase C).
//!
//! OpenFGA réel (testcontainer). Harness unique `common::setup_test_server`.
//! `#[serial]` partout en live. Arrange : `project` + Admin/Read uniquement ;
//! jamais de tuple `component:{id}` ad hoc. Lecture seule des tuples existants.
//! T5 prouve uniquement : grants inchangés + revoke→403 au commit DB.
//! Pas de preuve event→FGA (sentinel-sync hors scope, cf. escalade).

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use rustycog::permission::{Permission, ResourceRef, Subject};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;
use uuid::Uuid;

fn create_test_jwt_token(user_id: Uuid) -> String {
    rustycog::testing::http::jwt::create_jwt_token(user_id)
}

#[test]
fn t5_model_has_no_new_apparatus_type() {
    // Garde : aucun nouveau type FGA (escalade obligatoire sinon).
    let model = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../openfga/model.fga");
    let content = std::fs::read_to_string(&model).expect("model.fga lisible");
    let lower = content.to_lowercase();
    assert!(
        !lower.contains("type apparatus"),
        "T5 : nouveau type FGA apparatus interdit sans escalade"
    );
    assert!(
        !lower.contains("apparatus_binding"),
        "T5 : type apparatus_binding interdit (ne pas inventer)"
    );
    for known in [
        "type user",
        "type organization",
        "type project",
        "type component",
        "type notification",
    ] {
        assert!(lower.contains(known), "T5 : type connu conservé ({known})");
    }
}

#[tokio::test]
#[serial]
async fn t5_project_grants_unchanged_by_component_write() {
    // Grants project inchangés : l'écriture composant n'ajoute aucun tuple.
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
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

    let object = format!("project:{}", project.id());
    let before = openfga
        .read_tuples(None, None, Some(object.as_str()))
        .await
        .expect("lecture tuples avant");
    assert!(!before.is_empty(), "grant arrangé présent");

    let jwt = create_test_jwt_token(owner_id);
    let resp = client
        .post(format!(
            "{}/api/projects/{}/components",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"component_type": "taskboard"}))
        .send()
        .await
        .expect("POST component");
    assert_eq!(resp.status(), 201);

    let after = openfga
        .read_tuples(None, None, Some(object.as_str()))
        .await
        .expect("lecture tuples après");
    assert_eq!(
        format!("{before:?}"),
        format!("{after:?}"),
        "T5 : tuples project inchangés après écriture composant"
    );

    // Aucun tuple component:{id} ad hoc écrit par l'app.
    let created: serde_json::Value = resp.json().await.expect("JSON");
    let cid = created["id"].as_str().expect("id composant");
    let component_tuples = openfga
        .read_tuples(None, None, Some(format!("component:{cid}").as_str()))
        .await
        .expect("lecture tuples component");
    assert!(
        component_tuples.is_empty(),
        "T5 : aucun tuple component:{{id}} ad hoc"
    );
}

#[tokio::test]
#[serial]
async fn t5_revoke_is_fail_closed_403_after_db_commit() {
    // Revoke→403 fail-closed dès le commit DB (TTL0, cf. test.toml).
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant");
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("grant admin");
    let jwt = create_test_jwt_token(owner_id);
    let url = format!(
        "{}/api/projects/{}/components/{}",
        base_url,
        project.id(),
        component.id()
    );
    let ok = client
        .patch(&url)
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"status": "configured"}))
        .send()
        .await
        .expect("PATCH avant revoke");
    assert_eq!(ok.status(), 200);

    openfga
        .deny(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("revoke admin");

    let denied = client
        .patch(&url)
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"status": "active"}))
        .send()
        .await
        .expect("PATCH après revoke");
    assert_eq!(denied.status(), 403, "T5 : revoke ⇒ 403 fail-closed");
}

#[tokio::test]
#[serial]
async fn t5_extension_row_commits_with_component_write() {
    // RED : l'écriture composant doit committer sa ligne d'extension (T1/T2 requis).
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let tables = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE '%apparatus%'".to_owned(),
        ))
        .await
        .expect("information_schema lisible");
    assert!(
        !tables.is_empty(),
        "RED T5 : aucune table d'extension '%apparatus%' — ownership extension non committé"
    );
    let table: String = tables[0].try_get("", "table_name").expect("table_name");
    let fk_col: String = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!(
                "SELECT kcu.column_name FROM information_schema.table_constraints tc \
                 JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name \
                 JOIN information_schema.constraint_column_usage ccu ON tc.constraint_name = ccu.constraint_name \
                 WHERE tc.table_name = '{table}' AND tc.constraint_type = 'FOREIGN KEY' \
                 AND ccu.table_name = 'project_components' AND ccu.column_name = 'id'"
            ),
        ))
        .await
        .expect("FK lisible")
        .expect("RED T5 : extension sans FK vers project_components(id)")
        .try_get("", "column_name")
        .expect("colonne FK");

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
    let cid = created["id"].as_str().expect("id composant");

    let count = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT COUNT(*) AS n FROM {table} WHERE {fk_col} = '{cid}'::uuid"),
        ))
        .await
        .expect("comptage extension")
        .expect("comptage présent");
    let n: i64 = count.try_get("", "n").expect("n");
    assert!(
        n >= 1,
        "RED T5 : ligne d'extension committée avec le composant"
    );
}
