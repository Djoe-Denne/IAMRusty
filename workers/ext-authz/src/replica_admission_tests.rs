use crate::replica_proof::{ReplicaSource, Snapshot};
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

pub(super) struct FixtureReplica {
    pub(super) reads: AtomicUsize,
    pub(super) expected_subject: Uuid,
    pub(super) expected_challenge: Uuid,
    pub(super) snapshot: Snapshot,
}

#[async_trait::async_trait]
impl ReplicaSource for FixtureReplica {
    async fn snapshot(&self, challenge: Uuid, subject: Uuid) -> Option<Snapshot> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        assert_eq!(self.expected_challenge, challenge);
        assert_eq!(self.expected_subject, subject);
        Some(self.snapshot)
    }
}
