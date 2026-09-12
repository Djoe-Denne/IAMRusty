//! Apparatus P1 — T2 backfill legacy (RED, TDD Phase C).
//!
//! Harness unique `common::setup_test_server` (5-tuple). `#[serial]` sur tout
//! test touchant `TestOpenFga`/wiremock. Arrange : `project` + Admin/Read uniquement.
//! Escalade : la forme du backfill (migration de données vs fonction setup vs
//! commande) reste une décision GREEN ; les tests live constatent l'état.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use manifesto_infra::apparatus_backfill::backfill_apparatus_legacy;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

fn backfill_mentions() -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut hits = Vec::new();
    for sub in [
        "domain/src",
        "application/src",
        "infra/src",
        "setup/src",
        "migration/src",
    ] {
        collect(&root.join(sub), &mut hits);
    }
    hits
}

fn collect(dir: &std::path::Path, hits: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let Ok(path) = entry.map(|e| e.path()) else {
            continue;
        };
        if path.is_dir() {
            collect(&path, hits);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let name = path.to_string_lossy().to_lowercase();
            if name.contains("backfill") {
                hits.push(path.display().to_string());
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                let lower = content.to_lowercase();
                if lower.contains("backfill") && lower.contains("apparatus") {
                    hits.push(path.display().to_string());
                }
            }
        }
    }
}

#[test]
fn t2_backfill_entrypoint_exists() {
    assert!(
        !backfill_mentions().is_empty(),
        "RED T2 : aucun backfill apparatus (fichier ou mention backfill+apparatus) dans le prod Manifesto"
    );
}

/// Nom de la table d'extension (`%apparatus%`) ou RED explicite.
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
        "RED T2 : aucune table d'extension '%apparatus%' — migration T1 / backfill T2 non implémentés"
    );
    rows[0]
        .try_get::<String>("", "table_name")
        .expect("table_name lisible")
}

/// Colonne FK de l'extension vers `project_components(id)` (nom décidé par GREEN).
async fn extension_fk_column(db: &Arc<sea_orm::DatabaseConnection>, table: &str) -> String {
    let rows = db
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
        !rows.is_empty(),
        "RED T2 : {table} sans FK vers project_components(id)"
    );
    rows[0]
        .try_get::<String>("", "column_name")
        .expect("colonne FK")
}

#[tokio::test]
#[serial]
async fn t2_legacy_rows_keep_ids_status_and_component_type() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let table = require_extension_table(&db).await;

    let owner_id = Uuid::new_v4();
    let (_project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant legacy");
    // GREEN T2 (test-only) : backfill explicite idempotent — aucune écriture
    // auto au commit (préserve T4 : 0 ligne sans appel explicite).
    backfill_apparatus_legacy(db.as_ref())
        .await
        .expect("backfill legacy");
    let before = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!(
                "SELECT id, component_type, status FROM project_components WHERE id = '{}'",
                component.id()
            ),
        ))
        .await
        .expect("lecture legacy")
        .expect("ligne legacy présente");
    let id: Uuid = before.try_get("", "id").expect("id");
    let ctype: String = before.try_get("", "component_type").expect("type");
    let status: String = before.try_get("", "status").expect("status");
    assert_eq!(id, component.id(), "ID legacy inchangé");
    assert_eq!(ctype, "taskboard", "component_type non réécrit");
    assert_eq!(status, "pending", "statut legacy inchangé");

    // Le backfill marque source=legacy, release/digest non résolus (NULL).
    let fk_col = extension_fk_column(&db, &table).await;
    let ext = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!(
                "SELECT source, digest FROM {table} WHERE {fk_col} = '{}'",
                component.id()
            ),
        ))
        .await
        .expect("lecture extension")
        .expect("RED T2 : aucune ligne d'extension pour le composant legacy — backfill absent");
    let source: String = ext.try_get("", "source").expect("source");
    let digest: Option<String> = ext.try_get("", "digest").expect("digest");
    assert_eq!(source, "legacy", "backfill source=legacy");
    assert!(digest.is_none(), "release/digest non résolus (NULL)");
}

#[tokio::test]
#[serial]
async fn t2_backfill_is_idempotent_two_runs_same_state() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let table = require_extension_table(&db).await;

    let owner_id = Uuid::new_v4();
    let (_project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "wiki")
            .await
            .expect("composant legacy");
    // Deux runs explicites : même état (zéro duplication d'extension).
    let fk_col = extension_fk_column(&db, &table).await;
    for _ in 0..2 {
        // GREEN T2 (test-only) : rejoue le backfill puis constate l'état.
        backfill_apparatus_legacy(db.as_ref())
            .await
            .expect("backfill rejouable");
        let count = db
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                format!(
                    "SELECT COUNT(*) AS n FROM {table} WHERE {fk_col} = '{}'",
                    component.id()
                ),
            ))
            .await
            .expect("comptage extension")
            .expect("comptage présent");
        let n: i64 = count.try_get("", "n").expect("n");
        assert_eq!(
            n, 1,
            "RED T2 : exactement 1 ligne d'extension (backfill idempotent)"
        );
    }
}

#[tokio::test]
#[serial]
async fn t2_controller_off_and_zero_workload() {
    // Gardes vertes : contrôleur off, aucun workload démarré par le backfill.
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    for pattern in ["%apparatus_workload%", "%apparatus_controller%"] {
        let rows = db
            .query_all(Statement::from_string(
                DatabaseBackend::Postgres,
                format!(
                    "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE '{pattern}'"
                ),
            ))
            .await
            .expect("information_schema lisible");
        assert!(
            rows.is_empty(),
            "T2 : aucune table {pattern} (contrôleur off, zéro workload)"
        );
    }
}

#[tokio::test]
#[serial]
async fn t2_no_valid_verified_status_introduced() {
    // Garde verte : aucun statut VALID/VERIFIED produit par le backfill.
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let rows = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT DISTINCT status FROM project_components".to_owned(),
        ))
        .await
        .expect("statuts lisibles");
    for row in &rows {
        let status: String = row.try_get("", "status").expect("status");
        assert!(
            !matches!(status.as_str(), "VALID" | "VERIFIED" | "valid" | "verified"),
            "T2 : statut {status} interdit"
        );
    }
}
