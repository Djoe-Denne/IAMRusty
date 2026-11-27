use crate::replica_proof::{Proof, Snapshot};
use std::sync::{
    atomic::{AtomicU64, AtomicUsize},
    Arc,
};
use uuid::Uuid;

pub(super) struct ReplicaAdmission {
    pub(super) clock: Arc<AtomicU64>,
    pub(super) reads: AtomicUsize,
    pub(super) expected_subject: Uuid,
    pub(super) proof: Proof,
    pub(super) snapshot: Snapshot,
}
