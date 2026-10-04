//! Real PostgreSQL causality: separate pool holds a row lock; observe wait graph.
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DatabaseTransaction, Statement,
    TransactionTrait,
};
use uuid::Uuid;

pub async fn independent_writer(db: &DatabaseConnection) -> DatabaseConnection {
    let options = db
        .get_postgres_connection_pool()
        .connect_options()
        .as_ref()
        .clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .unwrap_or_else(|_| {
            panic!("independent test writer pool failed (connection details redacted)")
        });
    sea_orm::SqlxPostgresConnector::from_sqlx_postgres_pool(pool)
}

pub async fn hold(db: &DatabaseConnection, table: &str, id: Uuid) -> (DatabaseTransaction, i32) {
    assert!(matches!(table, "users" | "oauth_transactions"));
    let tx = db.begin().await.expect("lock transaction");
    let pid = tx
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pg_backend_pid() AS pid".to_owned(),
        ))
        .await
        .expect("backend PID")
        .expect("PID row")
        .try_get("", "pid")
        .expect("PID");
    let locked = tx
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("SELECT id FROM {table} WHERE id = $1 FOR UPDATE"),
            [id.into()],
        ))
        .await
        .expect("pre-acquire writer row lock");
    assert!(locked.is_some(), "barrier must lock an existing real row");
    (tx, pid)
}

pub async fn wait_for_blocked(db: &DatabaseConnection, holder: i32, minimum: i64) {
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "WITH RECURSIVE blocked AS (SELECT pid FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid)) UNION SELECT a.pid FROM pg_stat_activity a JOIN blocked b ON b.pid = ANY(pg_blocking_pids(a.pid))) SELECT COUNT(*) AS count FROM blocked",
                [holder.into()])).await.expect("observe actual lock waits").expect("count row");
            if row.try_get::<i64>("", "count").expect("wait count") >= minimum { break; }
            tokio::task::yield_now().await;
        }
    }).await.expect("workers must be causally blocked before releasing the row lock");
}
