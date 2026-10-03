//! Apparatus P3 — T2 migration additive réversible (ADR-0007).
//!
//! Harness unique `common::setup_test_server`. `#[serial]` sur tout test
//! touchant Postgres / `TestOpenFga`.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

const P3_MIGRATION_FILE: &str = "m20260913_000014_apparatus_p3_grants.rs";
const DECLARED_MIGRATION_FILE: &str = "m20260916_000015_apparatus_declared_capabilities.rs";

fn migration_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migration/src")
}

fn p3_migration_path() -> std::path::PathBuf {
    migration_dir().join(P3_MIGRATION_FILE)
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
            "SELECT column_name, data_type, is_nullable, column_default, character_maximum_length \
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

async fn constraint_names(db: &Arc<sea_orm::DatabaseConnection>, table: &str) -> Vec<String> {
    let rows = query_all(
        db,
        format!(
            "SELECT con.conname \
             FROM pg_constraint con \
             JOIN pg_class rel ON con.conrelid = rel.oid \
             JOIN pg_namespace nsp ON rel.relnamespace = nsp.oid \
             WHERE nsp.nspname = 'public' AND rel.relname = '{table}'"
        ),
    )
    .await;
    rows.iter()
        .map(|r| r.try_get("", "conname").expect("conname"))
        .collect()
}

// ---------------------------------------------------------------------------
// Gates purs (sans Docker)
// ---------------------------------------------------------------------------

#[test]
fn t2_migration_file_exists() {
    assert!(
        p3_migration_path().is_file(),
        "RED T2 : fichier {P3_MIGRATION_FILE} absent"
    );
}

#[test]
fn t2_migration_registered_in_migrator() {
    let lib = std::fs::read_to_string(migration_dir().join("lib.rs")).expect("lib.rs lisible");
    assert!(
        lib.contains("m20260913_000014_apparatus_p3_grants"),
        "RED T2 : Migrator n'enregistre pas m20260913_000014_apparatus_p3_grants"
    );
    assert!(
        lib.contains("m20260916_000015_apparatus_declared_capabilities"),
        "RED T2 : Migrator n'enregistre pas m20260916_000015_apparatus_declared_capabilities"
    );
}

#[test]
fn t2_declared_capabilities_migration_file_exists() {
    assert!(
        migration_dir().join(DECLARED_MIGRATION_FILE).is_file(),
        "RED T2 : fichier {DECLARED_MIGRATION_FILE} absent"
    );
    let content = std::fs::read_to_string(migration_dir().join(DECLARED_MIGRATION_FILE))
        .expect("migration 000015 lisible");
    let lower = content.to_lowercase();
    assert!(
        lower.contains("declared_capabilities"),
        "RED T2 : declared_capabilities absent de 000015"
    );
    assert!(
        lower.contains("jsonb") || lower.contains("json_binary"),
        "RED T2 : declared_capabilities n'est pas jsonb"
    );
}

#[test]
fn t2_migration_declares_grant_revision_and_consents() {
    let content = std::fs::read_to_string(p3_migration_path()).expect("RED T2 : migration absente");
    let lower = content.to_lowercase();
    assert!(
        lower.contains("grant_revision"),
        "RED T2 : colonne grant_revision non déclarée"
    );
    assert!(
        lower.contains("apparatus_capability_consents"),
        "RED T2 : table apparatus_capability_consents non déclarée"
    );
    for name in [
        "chk_apparatus_bindings_grant_revision_non_negative",
        "chk_apparatus_capability_consents_status",
        "chk_apparatus_capability_consents_grant_revision_non_negative",
        "uq_apparatus_capability_consents_component_capability",
        "fk_apparatus_capability_consents_component",
    ] {
        assert!(
            lower.contains(name),
            "RED T2 : contrainte nommée {name} absente de la migration"
        );
    }
    assert!(
        lower.contains("consented") && lower.contains("revoked"),
        "RED T2 : status consented|revoked non déclaré"
    );
}

// ---------------------------------------------------------------------------
// Intégration DB
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn t2_bindings_grant_revision_type_null_default() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();

    let row = column_row(&db, "apparatus_bindings", "grant_revision")
        .await
        .expect("RED T2 : colonne apparatus_bindings.grant_revision absente");
    let got_type: String = row.try_get("", "data_type").expect("data_type");
    let got_null: String = row.try_get("", "is_nullable").expect("is_nullable");
    let got_default: Option<String> = row.try_get("", "column_default").ok().flatten();
    assert_eq!(got_type, "bigint", "type grant_revision");
    assert_eq!(got_null, "NO", "nullability grant_revision");
    let default = got_default.expect("column_default grant_revision");
    assert!(default.contains('0'), "DEFAULT 0 attendu, obtenu {default}");

    let declared = column_row(&db, "apparatus_bindings", "declared_capabilities")
        .await
        .expect("RED T2 : colonne apparatus_bindings.declared_capabilities absente");
    let declared_type: String = declared.try_get("", "data_type").expect("data_type");
    let declared_null: String = declared.try_get("", "is_nullable").expect("is_nullable");
    let declared_default: Option<String> = declared.try_get("", "column_default").ok().flatten();
    assert_eq!(declared_type, "jsonb", "type declared_capabilities");
    assert_eq!(declared_null, "NO", "nullability declared_capabilities");
    let declared_default = declared_default.expect("column_default declared_capabilities");
    assert!(
        declared_default.contains("[]"),
        "DEFAULT [] attendu, obtenu {declared_default}"
    );
}

#[tokio::test]
#[serial]
async fn t2_capability_consents_table_and_named_constraints() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    assert!(
        table_exists(&db, "apparatus_capability_consents").await,
        "RED T2 : table apparatus_capability_consents absente"
    );

    let expected: [(&str, &str, &str); 4] = [
        ("id", "bigint", "NO"),
        ("component_id", "uuid", "NO"),
        ("capability", "text", "NO"),
        ("status", "text", "NO"),
    ];
    for (name, data_type, nullable) in expected {
        let row = column_row(&db, "apparatus_capability_consents", name)
            .await
            .unwrap_or_else(|| {
                panic!("RED T2 : colonne apparatus_capability_consents.{name} absente")
            });
        let got_type: String = row.try_get("", "data_type").expect("data_type");
        let got_null: String = row.try_get("", "is_nullable").expect("is_nullable");
        assert_eq!(got_type, data_type, "type {name}");
        assert_eq!(got_null, nullable, "nullability {name}");
    }
    let grant = column_row(&db, "apparatus_capability_consents", "grant_revision")
        .await
        .expect("RED T2 : grant_revision sur consents absente");
    let grant_type: String = grant.try_get("", "data_type").expect("data_type");
    let grant_null: String = grant.try_get("", "is_nullable").expect("is_nullable");
    assert_eq!(grant_type, "bigint");
    assert_eq!(grant_null, "NO");

    let names = constraint_names(&db, "apparatus_capability_consents").await;
    for expected_name in [
        "chk_apparatus_capability_consents_status",
        "chk_apparatus_capability_consents_grant_revision_non_negative",
        "uq_apparatus_capability_consents_component_capability",
        "fk_apparatus_capability_consents_component",
    ] {
        assert!(
            names.iter().any(|n| n == expected_name),
            "RED T2 : contrainte {expected_name} absente, trouvé {names:?}"
        );
    }

    let binding_checks = constraint_names(&db, "apparatus_bindings").await;
    assert!(
        binding_checks
            .iter()
            .any(|n| n == "chk_apparatus_bindings_grant_revision_non_negative"),
        "RED T2 : chk_apparatus_bindings_grant_revision_non_negative absente, trouvé {binding_checks:?}"
    );
}

#[tokio::test]
#[serial]
async fn t2_consents_unique_and_fk_cascade_to_project_components() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();

    let uniq = query_all(
        &db,
        "SELECT tc.constraint_name FROM information_schema.table_constraints tc \
         JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name \
         WHERE tc.table_schema = 'public' AND tc.table_name = 'apparatus_capability_consents' \
           AND tc.constraint_type = 'UNIQUE' \
           AND kcu.column_name IN ('component_id', 'capability')",
    )
    .await;
    assert!(
        !uniq.is_empty(),
        "UNIQUE (component_id, capability) absente"
    );

    let fks = query_all(
        &db,
        "SELECT rc.delete_rule, tc.constraint_name FROM information_schema.table_constraints tc \
         JOIN information_schema.referential_constraints rc \
           ON tc.constraint_name = rc.constraint_name \
          AND tc.constraint_schema = rc.constraint_schema \
         JOIN information_schema.constraint_column_usage ccu \
           ON tc.constraint_name = ccu.constraint_name \
          AND tc.table_schema = ccu.table_schema \
         WHERE tc.table_schema = 'public' AND tc.table_name = 'apparatus_capability_consents' \
           AND tc.constraint_type = 'FOREIGN KEY' \
           AND ccu.table_name = 'project_components' AND ccu.column_name = 'id'",
    )
    .await;
    assert!(
        !fks.is_empty(),
        "FK apparatus_capability_consents → project_components absente"
    );
    let name: String = fks[0]
        .try_get("", "constraint_name")
        .expect("constraint_name");
    assert_eq!(name, "fk_apparatus_capability_consents_component");
    let rule: String = fks[0].try_get("", "delete_rule").expect("delete_rule");
    assert_eq!(rule, "CASCADE", "FK CASCADE attendu");

    let owner = Uuid::new_v4();
    let (_project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner, "taskboard")
            .await
            .expect("composant");
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO apparatus_capability_consents (component_id, capability, status, grant_revision) \
         VALUES ($1, 'kv.read', 'consented', 0)",
        [component.id().into()],
    ))
    .await
    .expect("INSERT consent");

    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM project_components WHERE id = $1",
        [component.id().into()],
    ))
    .await
    .expect("DELETE component");

    let leftover = query_all(
        &db,
        format!(
            "SELECT id FROM apparatus_capability_consents WHERE component_id = '{}'",
            component.id()
        ),
    )
    .await;
    assert!(
        leftover.is_empty(),
        "CASCADE n'a pas retiré le consentement"
    );
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

#[tokio::test]
#[serial]
async fn t2_grant_revision_distinct_from_desired_generation() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    assert!(
        column_row(&db, "apparatus_bindings", "desired_generation")
            .await
            .is_some(),
        "desired_generation P2 doit rester"
    );
    assert!(
        column_row(&db, "apparatus_bindings", "grant_revision")
            .await
            .is_some(),
        "grant_revision P3 doit exister"
    );
    assert_ne!(
        "desired_generation", "grant_revision",
        "les deux identifiants doivent rester distincts"
    );

    let owner = Uuid::new_v4();
    let (_project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner, "taskboard")
            .await
            .expect("composant");
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO apparatus_bindings (component_id, source, desired_generation, grant_revision) \
         VALUES ($1, 'managed', 3, 7)",
        [component.id().into()],
    ))
    .await
    .expect("INSERT indépendant desired vs grant");

    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT desired_generation, grant_revision FROM apparatus_bindings WHERE component_id = $1",
            [component.id().into()],
        ))
        .await
        .expect("select")
        .expect("binding");
    let desired: i64 = row.try_get("", "desired_generation").expect("desired");
    let grant: i64 = row.try_get("", "grant_revision").expect("grant");
    assert_eq!(desired, 3);
    assert_eq!(grant, 7);
}

#[tokio::test]
#[serial]
async fn t2_insert_binding_without_grant_revision_uses_default() {
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
    .expect("INSERT managed sans grant_revision");

    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT grant_revision FROM apparatus_bindings WHERE component_id = $1",
            [component.id().into()],
        ))
        .await
        .expect("select default")
        .expect("binding inséré");
    let grant: i64 = row.try_get("", "grant_revision").expect("grant_revision");
    assert_eq!(grant, 0);
}

#[tokio::test]
#[serial]
async fn t2_down_one_step_then_up_reversible() {
    use manifesto_migration::{Migrator, MigratorTrait};
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();

    let applied = Migrator::get_applied_migrations(db.as_ref())
        .await
        .expect("migrations appliquées");
    let mut steps = 0u32;
    let mut found = false;
    for migration in applied.iter().rev() {
        steps += 1;
        if migration.name().contains("apparatus_p3_grants") {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "RED T2 : m20260913_000014_apparatus_p3_grants non appliquée"
    );
    Migrator::down(db.as_ref(), Some(steps))
        .await
        .expect("down P3");

    assert!(
        column_row(&db, "apparatus_bindings", "grant_revision")
            .await
            .is_none(),
        "down : grant_revision doit disparaître"
    );
    assert!(
        column_row(&db, "apparatus_bindings", "declared_capabilities")
            .await
            .is_none(),
        "down : declared_capabilities doit disparaître"
    );
    assert!(
        !table_exists(&db, "apparatus_capability_consents").await,
        "down : apparatus_capability_consents doit disparaître"
    );
    assert!(
        table_exists(&db, "apparatus_bindings").await,
        "down ne doit pas drop apparatus_bindings"
    );
    for col in [
        "id",
        "component_id",
        "digest",
        "source",
        "desired_generation",
    ] {
        assert!(
            column_row(&db, "apparatus_bindings", col).await.is_some(),
            "colonne P1/P2 {col} doit rester après down P3"
        );
    }

    Migrator::up(db.as_ref(), None)
        .await
        .expect("up après down P3");
    assert!(
        column_row(&db, "apparatus_bindings", "grant_revision")
            .await
            .is_some(),
        "up : grant_revision doit revenir"
    );
    assert!(
        table_exists(&db, "apparatus_capability_consents").await,
        "up : apparatus_capability_consents doit revenir"
    );
    assert!(
        column_row(&db, "apparatus_bindings", "declared_capabilities")
            .await
            .is_some(),
        "up : declared_capabilities doit revenir"
    );
}

#[tokio::test]
#[serial]
async fn t2_no_kv_tables_in_manifesto_db() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    assert!(
        !table_exists(&db, "apparatus_kv_entries").await,
        "RED T2 : apparatus_kv_entries ne doit pas exister dans Manifesto"
    );
    let kv = query_all(
        &db,
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name LIKE '%kv%'",
    )
    .await;
    assert!(
        kv.is_empty(),
        "RED T2 : table KV inattendue dans Manifesto : {kv:?}"
    );
}
