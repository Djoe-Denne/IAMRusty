//! Idempotency ledger: records every `event_id` the worker has already
//! processed so retries and replays never re-apply the same tuple change.
//!
//! The in-memory implementation is sufficient for tests and local dev. The
//! Postgres-backed implementation is tracked in the sync-worker reference
//! page and will be added in a follow-up.

use std::collections::HashSet;
use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::config::IdempotencyConfig;

/// Ledger that records processed event ids.
#[async_trait]
pub trait EventLedger: Send + Sync {
    /// Return `Ok(true)` when `event_id` has not been seen before and was
    /// recorded. Return `Ok(false)` when it is a duplicate.
    async fn record(&self, event_id: Uuid) -> Result<bool>;
}

/// In-memory ledger. Loses state on restart — only use for tests and local
/// dev.
#[derive(Default)]
pub struct InMemoryEventLedger {
    seen: Mutex<HashSet<Uuid>>,
}

impl InMemoryEventLedger {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EventLedger for InMemoryEventLedger {
    async fn record(&self, event_id: Uuid) -> Result<bool> {
        let mut guard = self.seen.lock().unwrap();
        Ok(guard.insert(event_id))
    }
}

/// Build a ledger from config. Today only the in-memory backend is wired;
/// `"postgres"` is reserved for a follow-up.
pub fn build_ledger(config: &IdempotencyConfig) -> Result<Box<dyn EventLedger>> {
    match config.backend.as_str() {
        "in-memory" => Ok(Box::new(InMemoryEventLedger::new())),
        "postgres" => Err(anyhow::anyhow!(
            "idempotency.backend = \"postgres\" is not yet implemented; set to \"in-memory\" for now"
        )),
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
        assert!(ledger.record(id).await.unwrap());
        assert!(!ledger.record(id).await.unwrap());
    }
}
