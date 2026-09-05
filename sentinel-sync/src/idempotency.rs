//! Idempotency ledger: records event processing state so completed events are
//! skipped while failed `OpenFGA` writes remain retryable.
//!
//! The in-memory implementation is sufficient for tests and local dev. The
//! Postgres-backed implementation stores durable `processing` / `failed` /
//! `completed` state for production-style workers.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, PoisonError};

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

    /// Return whether a visibility revision is the next change to apply for a project.
    /// Older revisions are skipped and gaps remain retryable until their predecessor completes.
    async fn begin_visibility_change(&self, project_id: Uuid, revision: i64) -> Result<bool>;

    /// Advance the durable visibility revision after OpenFGA accepted the delta.
    async fn complete_visibility_change(&self, project_id: Uuid, revision: i64) -> Result<()>;
}

/// In-memory ledger. Loses state on restart — only use for tests and local
/// dev.
#[derive(Default)]
pub struct InMemoryEventLedger {
    completed: Mutex<HashSet<Uuid>>,
    visibility_revisions: Mutex<HashMap<Uuid, i64>>,
}

impl InMemoryEventLedger {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EventLedger for InMemoryEventLedger {
    async fn begin(&self, event_id: Uuid) -> Result<bool> {
        let guard = self
            .completed
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        Ok(!guard.contains(&event_id))
    }

    async fn complete(&self, event_id: Uuid) -> Result<()> {
        self.completed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(event_id);
        Ok(())
    }

    async fn fail(&self, _event_id: Uuid, _error: &str) -> Result<()> {
        Ok(())
    }

    async fn begin_visibility_change(&self, project_id: Uuid, revision: i64) -> Result<bool> {
        let revisions = self
            .visibility_revisions
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        begin_visibility_revision(revisions.get(&project_id).copied(), revision)
    }

    async fn complete_visibility_change(&self, project_id: Uuid, revision: i64) -> Result<()> {
        let mut revisions = self
            .visibility_revisions
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        complete_visibility_revision(&mut revisions, project_id, revision)
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
                r"
                CREATE TABLE IF NOT EXISTS sentinel_sync_event_ledger (
                    event_id TEXT PRIMARY KEY,
                    status TEXT NOT NULL,
                    attempts INTEGER NOT NULL DEFAULT 0,
                    last_error TEXT,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
                )
                ",
            ))
            .await?;
        self.db
            .execute(Statement::from_string(
                DbBackend::Postgres,
                r"
                CREATE TABLE IF NOT EXISTS sentinel_sync_visibility_revisions (
                    project_id UUID PRIMARY KEY,
                    last_applied_revision BIGINT NOT NULL CHECK (last_applied_revision >= 0),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
                )
                ",
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
                r"
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
                ",
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
                r"
                UPDATE sentinel_sync_event_ledger
                SET status = 'completed',
                    last_error = NULL,
                    updated_at = now()
                WHERE event_id = $1
                ",
                [event_id.to_string().into()],
            ))
            .await?;
        Ok(())
    }

    async fn fail(&self, event_id: Uuid, error: &str) -> Result<()> {
        self.db
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r"
                UPDATE sentinel_sync_event_ledger
                SET status = 'failed',
                    last_error = $2,
                    updated_at = now()
                WHERE event_id = $1 AND status <> 'completed'
                ",
                [event_id.to_string().into(), error.to_string().into()],
            ))
            .await?;
        Ok(())
    }

    async fn begin_visibility_change(&self, project_id: Uuid, revision: i64) -> Result<bool> {
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT last_applied_revision FROM sentinel_sync_visibility_revisions WHERE project_id = $1",
                [project_id.into()],
            ))
            .await?;
        let last_applied_revision = row
            .map(|row| row.try_get::<i64>("", "last_applied_revision"))
            .transpose()?;
        begin_visibility_revision(last_applied_revision, revision)
    }

    async fn complete_visibility_change(&self, project_id: Uuid, revision: i64) -> Result<()> {
        if revision <= 0 {
            return Err(anyhow::anyhow!("visibility revision must be positive"));
        }
        self.db
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r"
                INSERT INTO sentinel_sync_visibility_revisions
                    (project_id, last_applied_revision, updated_at)
                VALUES ($1, $2, now())
                ON CONFLICT (project_id) DO UPDATE
                    SET last_applied_revision = EXCLUDED.last_applied_revision,
                        updated_at = now()
                WHERE sentinel_sync_visibility_revisions.last_applied_revision = EXCLUDED.last_applied_revision - 1
                ",
                [project_id.into(), revision.into()],
            ))
            .await?;

        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT last_applied_revision FROM sentinel_sync_visibility_revisions WHERE project_id = $1",
                [project_id.into()],
            ))
            .await?;
        let last_applied_revision = row
            .ok_or_else(|| anyhow::anyhow!("visibility revision row was not persisted"))?
            .try_get::<i64>("", "last_applied_revision")?;
        if last_applied_revision < revision {
            return Err(anyhow::anyhow!(
                "visibility revision {revision} cannot complete before its predecessor"
            ));
        }
        Ok(())
    }
}

fn begin_visibility_revision(last_applied_revision: Option<i64>, revision: i64) -> Result<bool> {
    if revision <= 0 {
        return Err(anyhow::anyhow!("visibility revision must be positive"));
    }
    let last = last_applied_revision.unwrap_or(0);
    if revision <= last {
        return Ok(false);
    }
    let expected = last
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("visibility revision overflow"))?;
    if revision != expected {
        return Err(anyhow::anyhow!(
            "visibility revision {revision} arrived before required revision {expected}"
        ));
    }
    Ok(true)
}

fn complete_visibility_revision(
    revisions: &mut HashMap<Uuid, i64>,
    project_id: Uuid,
    revision: i64,
) -> Result<()> {
    if !begin_visibility_revision(revisions.get(&project_id).copied(), revision)? {
        return Ok(());
    }
    revisions.insert(project_id, revision);
    Ok(())
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

    #[tokio::test]
    async fn visibility_retries_cannot_reapply_an_obsolete_revision() {
        let ledger = InMemoryEventLedger::new();
        let project_id = Uuid::new_v4();

        assert!(ledger.begin_visibility_change(project_id, 1).await.unwrap());
        assert!(ledger.begin_visibility_change(project_id, 1).await.unwrap());
        ledger
            .complete_visibility_change(project_id, 1)
            .await
            .unwrap();

        assert!(ledger.begin_visibility_change(project_id, 2).await.unwrap());
        ledger
            .complete_visibility_change(project_id, 2)
            .await
            .unwrap();
        assert!(!ledger.begin_visibility_change(project_id, 1).await.unwrap());
    }

    #[tokio::test]
    async fn visibility_revision_gaps_remain_retryable() {
        let ledger = InMemoryEventLedger::new();
        let project_id = Uuid::new_v4();

        assert!(ledger.begin_visibility_change(project_id, 2).await.is_err());
        assert!(ledger.begin_visibility_change(project_id, 1).await.unwrap());
    }
}
