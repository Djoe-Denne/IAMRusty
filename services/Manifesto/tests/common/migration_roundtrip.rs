//! Initial-schema rollback evidence is isolated from retained runtime authority.
use manifesto_migration::{Migrator, MigratorTrait, SchemaManager};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement, TransactionTrait};

pub async fn assert_initial_schema_roundtrip(db: &DatabaseConnection, required_tables: &[&str]) {
    let tx = db.begin().await.expect("disposable migration transaction");
    let schema = format!("initial_roundtrip_{}", uuid::Uuid::new_v4().simple());
    tx.execute_unprepared(&format!("CREATE SCHEMA {schema}"))
        .await
        .expect("isolated schema");
    tx.execute_unprepared(&format!("SET LOCAL search_path TO {schema}"))
        .await
        .expect("isolated search path");
    let manager = SchemaManager::new(&tx);
    let migration = Migrator::migrations()
        .into_iter()
        .find(|migration| migration.name() == "m20241015_000001_initial_schema")
        .expect("registered initial schema");
    let inventory = || {
        Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT table_name || '.' || column_name AS name FROM information_schema.columns WHERE table_schema=$1 ORDER BY table_name,ordinal_position",
        [schema.clone().into()])
    };
    migration.up(&manager).await.expect("initial up");
    let before: Vec<String> = tx
        .query_all(inventory())
        .await
        .unwrap()
        .iter()
        .map(|row| row.try_get("", "name").unwrap())
        .collect();
    for table in required_tables {
        assert!(
            before
                .iter()
                .any(|name| name.starts_with(&format!("{table}."))),
            "initial migration must create {table}"
        );
    }
    migration
        .down(&manager)
        .await
        .expect("initial down in isolated transaction");
    assert!(
        tx.query_all(inventory()).await.unwrap().is_empty(),
        "initial down must remove all initial tables and columns"
    );
    migration.up(&manager).await.expect("initial up after down");
    let after: Vec<String> = tx
        .query_all(inventory())
        .await
        .unwrap()
        .iter()
        .map(|row| row.try_get("", "name").unwrap())
        .collect();
    assert_eq!(
        after, before,
        "initial roundtrip preserves every table and column"
    );
    tx.rollback().await.expect("discard never-persisted schema");
}
