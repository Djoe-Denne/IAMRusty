//! Idempotency ledger: records event processing state so completed events are
//! skipped while failed OpenFGA writes remain retryable.
//!
//! The in-memory implementation is sufficient for tests and local dev. The
//! Postgres-backed implementation stores durable `processing` / `failed` /
//! `completed` state for production-style workers.

use std::collections::HashSet;
use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, TransactionTrait,
};
use uuid::Uuid;

use crate::config::IdempotencyConfig;

/// Ledger that records processed event ids.
#[async_trait]
pub trait EventLedger: Send + Sync {
    /// Return `Ok(true)` when this delivery should be processed. Return
    /// `Ok(false)` when a previous delivery completed successfully.
    async fn begin(&self, event_id: Uuid) -> Result<bool>;

    /// Mark the event as fully applied.
    async fn complete(&self, event_id: Uuid) -> Result<()>;

    /// Mark an attempted delivery as failed while keeping it retryable.
    async fn fail(&self, event_id: Uuid, error: &str) -> Result<()>;
}

/// In-memory ledger. Loses state on restart — only use for tests and local
/// dev.
#[derive(Default)]
pub struct InMemoryEventLedger {
    completed: Mutex<HashSet<Uuid>>,
}

impl InMemoryEventLedger {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EventLedger for InMemoryEventLedger {
    async fn begin(&self, event_id: Uuid) -> Result<bool> {
        let guard = self.completed.lock().unwrap();
        Ok(!guard.contains(&event_id))
    }

    async fn complete(&self, event_id: Uuid) -> Result<()> {
        let mut guard = self.completed.lock().unwrap();
        guard.insert(event_id);
        Ok(())
    }

    async fn fail(&self, _event_id: Uuid, _error: &str) -> Result<()> {
        Ok(())
    }
}

pub struct PostgresEventLedger {
    db: DatabaseConnection,
}

impl PostgresEventLedger {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let db = Database::connect(database_url).await?;
        let ledger = Self { db };
        ledger.ensure_table().await?;
        Ok(ledger)
    }

    async fn ensure_table(&self) -> Result<()> {
        self.db
            .execute(Statement::from_string(
                DbBackend::Postgres,
                r#"
                CREATE TABLE IF NOT EXISTS sentinel_sync_event_ledger (
                    event_id TEXT PRIMARY KEY,
                    status TEXT NOT NULL,
                    attempts INTEGER NOT NULL DEFAULT 0,
                    last_error TEXT,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
                )
                "#,
            ))
            .await?;
        Ok(())
    }
}

#[async_trait]
impl EventLedger for PostgresEventLedger {
    async fn begin(&self, event_id: Uuid) -> Result<bool> {
        let txn = self.db.begin().await?;
        let event_id = event_id.to_string();
        let row = txn
            .query_one(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
                INSERT INTO sentinel_sync_event_ledger
                    (event_id, status, attempts, created_at, updated_at)
                VALUES ($1, 'processing', 1, now(), now())
                ON CONFLICT (event_id) DO UPDATE
                    SET status = 'processing',
                        attempts = sentinel_sync_event_ledger.attempts + 1,
                        last_error = NULL,
                        updated_at = now()
                WHERE sentinel_sync_event_ledger.status <> 'completed'
                RETURNING event_id
                "#,
                [event_id.into()],
            ))
            .await?;
        txn.commit().await?;
        Ok(row.is_some())
    }

    async fn complete(&self, event_id: Uuid) -> Result<()> {
        self.db
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
                UPDATE sentinel_sync_event_ledger
                SET status = 'completed',
                    last_error = NULL,
                    updated_at = now()
                WHERE event_id = $1
                "#,
                [event_id.to_string().into()],
            ))
            .await?;
        Ok(())
    }

    async fn fail(&self, event_id: Uuid, error: &str) -> Result<()> {
        self.db
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
                UPDATE sentinel_sync_event_ledger
                SET status = 'failed',
                    last_error = $2,
                    updated_at = now()
                WHERE event_id = $1 AND status <> 'completed'
                "#,
                [event_id.to_string().into(), error.to_string().into()],
            ))
            .await?;
        Ok(())
    }
}

/// Build a ledger from config.
pub async fn build_ledger(config: &IdempotencyConfig) -> Result<Box<dyn EventLedger>> {
    match config.backend.as_str() {
        "in-memory" => Ok(Box::new(InMemoryEventLedger::new())),
        "postgres" => {
            let database_url = config.database_url.as_deref().ok_or_else(|| {
                anyhow::anyhow!(
                    "idempotency.database_url is required when idempotency.backend = \"postgres\""
                )
            })?;
            Ok(Box::new(PostgresEventLedger::connect(database_url).await?))
        }
        other => Err(anyhow::anyhow!("unknown idempotency.backend: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn records_and_detects_duplicates() {
        let ledger = InMemoryEventLedger::new();
        let id = Uuid::new_v4();
        assert!(ledger.begin(id).await.unwrap());
        ledger.complete(id).await.unwrap();
        assert!(!ledger.begin(id).await.unwrap());
    }

    #[tokio::test]
    async fn failed_events_remain_retryable() {
        let ledger = InMemoryEventLedger::new();
        let id = Uuid::new_v4();
        assert!(ledger.begin(id).await.unwrap());
        ledger.fail(id, "fga unavailable").await.unwrap();
        assert!(ledger.begin(id).await.unwrap());
    }
}
