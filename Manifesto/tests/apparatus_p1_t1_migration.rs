//! Apparatus P1 — T1 migration additive réversible (RED, TDD Phase C).
//!
//! Harness unique `common::setup_test_server` (5-tuple). `#[serial]` sur tout
//! test touchant `TestOpenFga`/wiremock. Arrange : `project` + Admin/Read uniquement.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use rustycog::permission::{Permission, ResourceRef, Subject};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

fn create_test_jwt_token(user_id: Uuid) -> String {
    rustycog::testing::http::jwt::create_jwt_token(user_id)
}

fn migration_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migration/src")
}

/// Fichiers de migration portant `apparatus` dans leur nom.
fn apparatus_migrations() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(migration_dir()).expect("migration/src lisible");
    for entry in entries {
        let path = entry.expect("entrée lisible").path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.contains("apparatus") && n.ends_with(".rs"))
        {
            out.push(path);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Gates purs (sans Docker) — RED : migration absente
// ---------------------------------------------------------------------------

#[test]
fn t1_migration_file_exists() {
    assert!(
        !apparatus_migrations().is_empty(),
        "RED T1 : aucun fichier Manifesto/migration/src/*apparatus*.rs"
    );
}

#[test]
fn t1_migration_declares_extension_shape() {
    let files = apparatus_migrations();
    assert!(
        !files.is_empty(),
        "RED T1 : migration absente, forme d'extension non déclarée"
    );
    let mut ok = false;
    for file in &files {
        let content = std::fs::read_to_string(file).expect("migration lisible");
        let lower = content.to_lowercase();
        if lower.contains("project_components")
            && lower.contains("digest")
            && lower.contains("source")
            && lower.contains("legacy")
            && lower.contains("managed")
        {
            ok = true;
        }
    }
    assert!(
        ok,
        "RED T1 : la migration doit référencer project_components.id + digest nullable + source legacy|managed"
    );
}

#[test]
fn t1_migration_registered_in_migrator() {
    let lib = migration_dir().join("lib.rs");
    let content = std::fs::read_to_string(&lib).expect("migration lib lisible");
    assert!(
        content.to_lowercase().contains("apparatus"),
        "RED T1 : Migrator ne référence aucune migration apparatus"
    );
}

// ---------------------------------------------------------------------------
// Intégration DB (Postgres réel) — RED : extension absente
// ---------------------------------------------------------------------------

/// Nom de la table d'extension (`%apparatus%`) ou RED explicite.
/// Le nom exact reste une décision GREEN ; seule la convention est imposée.
async fn require_extension_table(db: &Arc<sea_orm::DatabaseConnection>) -> String {
    let rows = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE '%apparatus%'".to_owned(),
        ))
        .await
        .expect("information_schema lisible");
    assert!(
        !rows.is_empty(),
        "RED T1 : aucune table d'extension '%apparatus%' — migration non appliquée"
    );
    rows[0]
        .try_get::<String>("", "table_name")
        .expect("table_name lisible")
}

#[tokio::test]
#[serial]
async fn t1_up_creates_one_to_one_extension_table() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let table = require_extension_table(&db).await;
    assert!(!table.is_empty(), "nom de table d'extension");
}

#[tokio::test]
#[serial]
async fn t1_extension_fk_targets_project_components_id_with_uniqueness() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let table = require_extension_table(&db).await;

    // FK vers project_components(id).
    let fks = db
        .query_all(Statement::from_string(
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
        .expect("contraintes FK lisibles");
    assert!(
        !fks.is_empty(),
        "RED T1 : {table} doit référencer project_components(id) en FK"
    );
    let fk_col: String = fks[0].try_get("", "column_name").expect("colonne FK");

    // Unicité 1:1 sur la colonne FK (contrainte DB, pas `if` applicatif).
    let uniq = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            format!(
                "SELECT tc.constraint_name FROM information_schema.table_constraints tc \
                 JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name \
                 WHERE tc.table_name = '{table}' AND tc.constraint_type = 'UNIQUE' AND kcu.column_name = '{fk_col}'"
            ),
        ))
        .await
        .expect("contraintes UNIQUE lisibles");
    assert!(
        !uniq.is_empty(),
        "RED T1 : {table}.{fk_col} doit porter une contrainte UNIQUE (1:1)"
    );
}

#[tokio::test]
#[serial]
async fn t1_extension_digest_nullable_and_source_legacy_managed() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let table = require_extension_table(&db).await;

    let col = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!(
                "SELECT is_nullable FROM information_schema.columns WHERE table_name = '{table}' AND column_name = 'digest'"
            ),
        ))
        .await
        .expect("colonne digest lisible")
        .expect("RED T1 : colonne digest absente de la table d'extension");
    let nullable: String = col.try_get("", "is_nullable").expect("is_nullable");
    assert_eq!(nullable, "YES", "RED T1 : digest doit être nullable");

    let checks = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            format!(
                "SELECT cc.check_clause FROM information_schema.table_constraints tc \
                 JOIN information_schema.check_constraints cc ON tc.constraint_name = cc.constraint_name \
                 WHERE tc.table_name = '{table}'"
            ),
        ))
        .await
        .expect("checks lisibles");
    let mut found = false;
    for row in &checks {
        let clause: String = row.try_get("", "check_clause").expect("check_clause");
        if clause.contains("legacy") && clause.contains("managed") {
            found = true;
        }
    }
    assert!(
        found,
        "RED T1 : {table}.source doit être contraint à legacy|managed"
    );
}

#[tokio::test]
#[serial]
async fn t1_existing_uniqueness_intact_after_migration() {
    // Garde verte : l'unicité (project_id, component_type) survit à la migration.
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
    let jwt = create_test_jwt_token(owner_id);
    let mut statuses = Vec::new();
    for _ in 0..2 {
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
        statuses.push(resp.status().as_u16());
    }
    assert_eq!(
        statuses,
        vec![201, 409],
        "unicité (project_id, component_type) intacte après migration"
    );
    let jwt2 = create_test_jwt_token(owner_id);
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Read,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("grant read");
    let list = client
        .get(format!(
            "{}/api/projects/{}/components",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt2}"))
        .send()
        .await
        .expect("GET list");
    assert_eq!(list.status(), 200, "liste lisible après migration");
}

#[tokio::test]
#[serial]
async fn t1_migration_idempotent_up_and_clean_down_up() {
    use manifesto_migration::{Migrator, MigratorTrait};
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    // Rejouer up sur base migrée : idempotent.
    Migrator::up(db.as_ref(), None)
        .await
        .expect("up idempotent");
    let table = require_extension_table(&db).await;
    // down retire l'extension, up la restaure (rollback propre).
    Migrator::down(db.as_ref(), None)
        .await
        .expect("down propre");
    let gone = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            format!(
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name = '{table}'"
            ),
        ))
        .await
        .expect("tables lisibles");
    assert!(gone.is_empty(), "down doit retirer {table}");
    Migrator::up(db.as_ref(), None)
        .await
        .expect("up après down");
    let _ = require_extension_table(&db).await;
}
