//! Apparatus P2 — T2 migration additive réversible (ADR-0006 D).
//!
//! Harness unique `common::setup_test_server` (5-tuple). `#[serial]` sur tout
//! test touchant Postgres / `TestOpenFga`.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

const P2_MIGRATION_FILE: &str = "m20260912_000013_apparatus_p2_runtime.rs";

const P2_COLUMNS: [&str; 9] = [
    "desired_generation",
    "observed_generation",
    "observed_digest",
    "lease_epoch",
    "lease_owner",
    "lease_expires_at",
    "next_retry_at",
    "retry_count",
    "last_error_code",
];

const P2_INDEXES: [&str; 4] = [
    "idx_apparatus_bindings_managed_due",
    "idx_apparatus_bindings_lease_expiry",
    "uq_apparatus_cleanup_jobs_open",
    "idx_apparatus_cleanup_jobs_due",
];

fn migration_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migration/src")
}

fn p2_migration_path() -> std::path::PathBuf {
    migration_dir().join(P2_MIGRATION_FILE)
}

async fn query_all(
    db: &Arc<sea_orm::DatabaseConnection>,
    sql: impl Into<String>,
) -> Vec<sea_orm::QueryResult> {
    db.query_all(Statement::from_string(
        DatabaseBackend::Postgres,
        sql.into(),
    ))
    .await
    .expect("SQL lisible")
}

async fn column_row(
    db: &Arc<sea_orm::DatabaseConnection>,
    table: &str,
    column: &str,
) -> Option<sea_orm::QueryResult> {
    db.query_one(Statement::from_string(
        DatabaseBackend::Postgres,
        format!(
            "SELECT column_name, data_type, is_nullable, character_maximum_length \
             FROM information_schema.columns \
             WHERE table_schema = 'public' AND table_name = '{table}' AND column_name = '{column}'"
        ),
    ))
    .await
    .expect("information_schema.columns lisible")
}

async fn table_exists(db: &Arc<sea_orm::DatabaseConnection>, table: &str) -> bool {
    !query_all(
        db,
        format!(
            "SELECT table_name FROM information_schema.tables \
             WHERE table_schema = 'public' AND table_name = '{table}'"
        ),
    )
    .await
    .is_empty()
}

// ---------------------------------------------------------------------------
// Gates purs (sans Docker)
// ---------------------------------------------------------------------------

#[test]
fn t2_migration_file_exists() {
    assert!(
        p2_migration_path().is_file(),
        "RED T2 : fichier {P2_MIGRATION_FILE} absent"
    );
}

#[test]
fn t2_migration_registered_in_migrator() {
    let lib = std::fs::read_to_string(migration_dir().join("lib.rs")).expect("lib.rs lisible");
    assert!(
        lib.contains("m20260912_000013_apparatus_p2_runtime"),
        "RED T2 : Migrator n'enregistre pas m20260912_000013_apparatus_p2_runtime"
    );
}

#[test]
fn t2_migration_declares_nine_additive_columns() {
    let content = std::fs::read_to_string(p2_migration_path()).expect("RED T2 : migration absente");
    let lower = content.to_lowercase();
    for col in P2_COLUMNS {
        assert!(
            lower.contains(col),
            "RED T2 : la migration doit déclarer la colonne {col}"
        );
    }
}

#[test]
fn t2_migration_declares_cleanup_jobs_without_fk() {
    let content = std::fs::read_to_string(p2_migration_path()).expect("RED T2 : migration absente");
    let lower = content.to_lowercase();
    assert!(
        lower.contains("apparatus_cleanup_jobs"),
        "RED T2 : table apparatus_cleanup_jobs non déclarée"
    );
    for col in [
        "component_id",
        "project_id",
        "desired_generation",
        "digest",
        "completed_at",
    ] {
        assert!(
            lower.contains(col),
            "RED T2 : apparatus_cleanup_jobs doit déclarer {col}"
        );
    }
    assert!(
        !content.contains("ForeignKey"),
        "RED T2 : apparatus_cleanup_jobs ne doit pas porter de FK"
    );
}

// ---------------------------------------------------------------------------
// Intégration DB
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn t2_bindings_have_nine_p2_columns_types() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();

    let expected: [(&str, &str, &str, Option<i32>); 9] = [
        ("desired_generation", "bigint", "NO", None),
        ("observed_generation", "bigint", "NO", None),
        ("observed_digest", "character varying", "YES", Some(128)),
        ("lease_epoch", "bigint", "NO", None),
        ("lease_owner", "character varying", "NO", Some(64)),
        ("lease_expires_at", "timestamp with time zone", "YES", None),
        ("next_retry_at", "timestamp with time zone", "YES", None),
        ("retry_count", "integer", "NO", None),
        ("last_error_code", "character varying", "YES", Some(64)),
    ];
    for (name, data_type, nullable, max_len) in expected {
        let row = column_row(&db, "apparatus_bindings", name)
            .await
            .unwrap_or_else(|| panic!("RED T2 : colonne apparatus_bindings.{name} absente"));
        let got_type: String = row.try_get("", "data_type").expect("data_type");
        let got_null: String = row.try_get("", "is_nullable").expect("is_nullable");
        assert_eq!(got_type, data_type, "type {name}");
        assert_eq!(got_null, nullable, "nullability {name}");
        if let Some(len) = max_len {
            let got_len: Option<i32> = row.try_get("", "character_maximum_length").ok().flatten();
            assert_eq!(got_len, Some(len), "varchar length {name}");
        }
    }
}

#[tokio::test]
#[serial]
async fn t2_cleanup_jobs_exists_without_fk_to_project_components() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    assert!(
        table_exists(&db, "apparatus_cleanup_jobs").await,
        "RED T2 : table apparatus_cleanup_jobs absente"
    );
    let fks = query_all(
        &db,
        "SELECT tc.constraint_name FROM information_schema.table_constraints tc \
         JOIN information_schema.constraint_column_usage ccu \
           ON tc.constraint_name = ccu.constraint_name \
          AND tc.table_schema = ccu.table_schema \
         WHERE tc.table_schema = 'public' AND tc.table_name = 'apparatus_cleanup_jobs' \
           AND tc.constraint_type = 'FOREIGN KEY' AND ccu.table_name = 'project_components'",
    )
    .await;
    assert!(
        fks.is_empty(),
        "RED T2 : apparatus_cleanup_jobs ne doit pas FK vers project_components"
    );
}

#[tokio::test]
#[serial]
async fn t2_one_to_one_unique_and_fk_cascade_intact() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();

    let uniq = query_all(
        &db,
        "SELECT tc.constraint_name FROM information_schema.table_constraints tc \
         JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name \
         WHERE tc.table_schema = 'public' AND tc.table_name = 'apparatus_bindings' \
           AND tc.constraint_type = 'UNIQUE' AND kcu.column_name = 'component_id'",
    )
    .await;
    assert!(
        !uniq.is_empty(),
        "UNIQUE apparatus_bindings.component_id (1:1) doit rester"
    );

    let fks = query_all(
        &db,
        "SELECT rc.delete_rule FROM information_schema.table_constraints tc \
         JOIN information_schema.referential_constraints rc \
           ON tc.constraint_name = rc.constraint_name \
          AND tc.constraint_schema = rc.constraint_schema \
         JOIN information_schema.constraint_column_usage ccu \
           ON tc.constraint_name = ccu.constraint_name \
          AND tc.table_schema = ccu.table_schema \
         WHERE tc.table_schema = 'public' AND tc.table_name = 'apparatus_bindings' \
           AND tc.constraint_type = 'FOREIGN KEY' \
           AND ccu.table_name = 'project_components' AND ccu.column_name = 'id'",
    )
    .await;
    assert!(
        !fks.is_empty(),
        "FK apparatus_bindings → project_components absente"
    );
    let rule: String = fks[0].try_get("", "delete_rule").expect("delete_rule");
    assert_eq!(rule, "CASCADE", "FK CASCADE P1 doit rester");
}

#[tokio::test]
#[serial]
async fn t2_source_check_legacy_managed_intact() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let checks = query_all(
        &db,
        "SELECT cc.check_clause FROM information_schema.table_constraints tc \
         JOIN information_schema.check_constraints cc ON tc.constraint_name = cc.constraint_name \
         WHERE tc.table_schema = 'public' AND tc.table_name = 'apparatus_bindings'",
    )
    .await;
    let found = checks.iter().any(|row| {
        let clause: String = row.try_get("", "check_clause").expect("check_clause");
        clause.contains("legacy") && clause.contains("managed")
    });
    assert!(found, "CHECK source legacy|managed doit rester");
}

#[tokio::test]
#[serial]
async fn t2_p2_indexes_present() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let rows = query_all(
        &db,
        "SELECT indexname FROM pg_indexes WHERE schemaname = 'public'",
    )
    .await;
    let names: Vec<String> = rows
        .iter()
        .map(|r| r.try_get("", "indexname").expect("indexname"))
        .collect();
    for idx in P2_INDEXES {
        assert!(
            names.iter().any(|n| n == idx),
            "RED T2 : index {idx} absent"
        );
    }
}

#[tokio::test]
#[serial]
async fn t2_down_one_step_then_up_reversible() {
    use manifesto_migration::{Migrator, MigratorTrait};
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();

    // P2 est enregistrée avant outbox : `down(Some(1))` reverse-vec peut
    // d'abord retirer outbox. On compte jusqu'à la migration P2.
    let applied = Migrator::get_applied_migrations(db.as_ref())
        .await
        .expect("migrations appliquées");
    let mut steps = 0u32;
    let mut found = false;
    for migration in applied.iter().rev() {
        steps += 1;
        if migration.name().contains("apparatus_p2_runtime") {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "RED T2 : m20260912_000013_apparatus_p2_runtime non appliquée"
    );
    Migrator::down(db.as_ref(), Some(steps))
        .await
        .expect("down P2");

    for col in P2_COLUMNS {
        assert!(
            column_row(&db, "apparatus_bindings", col).await.is_none(),
            "down : {col} doit disparaître"
        );
    }
    assert!(
        !table_exists(&db, "apparatus_cleanup_jobs").await,
        "down : apparatus_cleanup_jobs doit disparaître"
    );
    assert!(
        table_exists(&db, "apparatus_bindings").await,
        "down ne doit pas drop apparatus_bindings"
    );
    for col in ["id", "component_id", "digest", "source"] {
        assert!(
            column_row(&db, "apparatus_bindings", col).await.is_some(),
            "P1 {col} doit rester après down P2"
        );
    }

    Migrator::up(db.as_ref(), None)
        .await
        .expect("up après down P2");
    for col in P2_COLUMNS {
        assert!(
            column_row(&db, "apparatus_bindings", col).await.is_some(),
            "up : {col} doit revenir"
        );
    }
    assert!(
        table_exists(&db, "apparatus_cleanup_jobs").await,
        "up : apparatus_cleanup_jobs doit revenir"
    );
}

#[tokio::test]
#[serial]
async fn t2_insert_managed_without_p2_columns_uses_defaults() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner = Uuid::new_v4();
    let (_project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner, "taskboard")
            .await
            .expect("composant");

    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO apparatus_bindings (component_id, source) VALUES ($1, 'managed')",
        [component.id().into()],
    ))
    .await
    .expect("INSERT managed sans colonnes P2");

    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT desired_generation, observed_generation, lease_owner, \
                    (lease_expires_at IS NULL) AS lease_expires_is_null \
             FROM apparatus_bindings WHERE component_id = $1",
            [component.id().into()],
        ))
        .await
        .expect("select defaults")
        .expect("binding inséré");
    let desired: i64 = row
        .try_get("", "desired_generation")
        .expect("desired_generation");
    let observed: i64 = row
        .try_get("", "observed_generation")
        .expect("observed_generation");
    let owner: String = row.try_get("", "lease_owner").expect("lease_owner");
    let expires_null: bool = row
        .try_get("", "lease_expires_is_null")
        .expect("lease_expires_is_null");
    assert_eq!(desired, 0);
    assert_eq!(observed, 0);
    assert_eq!(owner, "");
    assert!(expires_null, "lease_expires_at NULL par défaut");
}

#[tokio::test]
#[serial]
async fn t2_no_second_public_uuid_on_bindings() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let rows = query_all(
        &db,
        "SELECT column_name FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = 'apparatus_bindings' \
           AND data_type = 'uuid'",
    )
    .await;
    let names: Vec<String> = rows
        .iter()
        .map(|r| r.try_get("", "column_name").expect("column_name"))
        .collect();
    assert_eq!(
        names,
        vec!["component_id".to_owned()],
        "aucune nouvelle colonne UUID publique sur apparatus_bindings"
    );
}
