use std::{
    cmp::Ordering,
    sync::{Arc, RwLock},
    time::Duration,
};
use uuid::Uuid;

const MAX_PROOF_AGE: Duration = Duration::from_secs(3);

#[derive(Clone, Copy)]
pub(super) struct Proof {
    pub(super) challenge: Uuid,
    pub(super) generation: Uuid,
    pub(super) started: Duration,
}

#[derive(Clone, Copy)]
pub(super) struct Snapshot {
    pub(super) ready_generation: Option<Uuid>,
    pub(super) applied_challenge: Option<Uuid>,
    pub(super) epoch: Option<u64>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Decision {
    Live,
    Revoked,
    NeedAuthority,
}

impl Proof {
    fn is_fresh(self, now: Duration) -> bool {
        now.checked_sub(self.started)
            .is_some_and(|age| age < MAX_PROOF_AGE)
    }

    fn decide(&self, now: Duration, snapshot: &Snapshot, claim: Option<u64>) -> Decision {
        if !self.is_fresh(now)
            || snapshot.ready_generation != Some(self.generation)
            || snapshot.applied_challenge != Some(self.challenge)
        {
            return Decision::NeedAuthority;
        }
        let Some(epoch) = snapshot.epoch else {
            return Decision::NeedAuthority;
        };
        match claim.unwrap_or(0).cmp(&epoch) {
            Ordering::Less => Decision::Revoked,
            Ordering::Equal => Decision::Live,
            Ordering::Greater => Decision::NeedAuthority,
        }
    }
}

#[async_trait::async_trait]
pub(super) trait ReplicaSource: Send + Sync {
    async fn snapshot(&self, challenge: Uuid, subject: Uuid) -> Option<Snapshot>;
}

pub(super) struct ReplicaAdmission {
    pub(super) clock: Arc<dyn Fn() -> Option<Duration> + Send + Sync>,
    pub(super) proof: RwLock<Option<Proof>>,
    pub(super) source: Arc<dyn ReplicaSource>,
}

impl ReplicaAdmission {
    fn current_proof(&self) -> Option<Proof> {
        *self
            .proof
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(super) async fn decide(
        &self,
        subject: Uuid,
        claim: Option<u64>,
    ) -> (Decision, Option<Proof>) {
        let Some(proof) = self.current_proof() else {
            return (Decision::NeedAuthority, None);
        };
        if !(self.clock)().is_some_and(|now| proof.is_fresh(now)) {
            return (Decision::NeedAuthority, None);
        }
        let result = tokio::time::timeout(
            Duration::from_millis(200),
            self.source.snapshot(proof.challenge, subject),
        )
        .await;
        if let (Ok(Some(snapshot)), Some(now)) = (result, (self.clock)()) {
            let decision = proof.decide(now, &snapshot, claim);
            let used = (decision == Decision::Live).then_some(proof);
            return (decision, used);
        }
        (Decision::NeedAuthority, None)
    }

    pub(super) fn proof_still_valid(&self, proof: Proof) -> bool {
        (self.clock)().is_some_and(|now| proof.is_fresh(now))
            && self.current_proof().is_some_and(|current| {
                current.generation == proof.generation && current.challenge == proof.challenge
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Decision, Proof, Snapshot};
    use std::time::Duration;
    use uuid::Uuid;

    fn fixture(epoch: Option<u64>) -> (Proof, Snapshot) {
        let proof = Proof {
            challenge: Uuid::new_v4(),
            generation: Uuid::new_v4(),
            started: Duration::from_secs(10),
        };
        let snapshot = Snapshot {
            ready_generation: Some(proof.generation),
            applied_challenge: Some(proof.challenge),
            epoch,
        };
        (proof, snapshot)
    }

    #[test]
    fn replica_decides_only_inside_the_strict_proof_age() {
        let (proof, snapshot) = fixture(Some(4));
        assert_eq!(
            proof.decide(proof.started + Duration::from_secs(3), &snapshot, Some(4)),
            Decision::NeedAuthority
        );
        assert_eq!(
            proof.decide(proof.started - Duration::from_millis(1), &snapshot, Some(4)),
            Decision::NeedAuthority,
            "a regressed clock cannot renew proof"
        );
        assert_eq!(
            proof.decide(
                proof.started + Duration::from_millis(2_999),
                &snapshot,
                Some(4),
            ),
            Decision::Live,
            "a verified replica inside the proof window decides without IAM"
        );
    }

    #[test]
    fn replica_requires_the_same_ready_generation_and_applied_challenge() {
        let (proof, snapshot) = fixture(Some(4));
        for untrusted in [
            Snapshot {
                ready_generation: None,
                ..snapshot
            },
            Snapshot {
                ready_generation: Some(Uuid::new_v4()),
                ..snapshot
            },
            Snapshot {
                applied_challenge: None,
                ..snapshot
            },
            Snapshot {
                applied_challenge: Some(Uuid::new_v4()),
                ..snapshot
            },
        ] {
            assert_eq!(
                proof.decide(proof.started, &untrusted, Some(4)),
                Decision::NeedAuthority
            );
        }
    }

    #[test]
    fn replica_compares_each_claim_to_an_existing_account_epoch() {
        let (proof, snapshot) = fixture(Some(4));
        assert_eq!(
            proof.decide(proof.started, &snapshot, Some(5)),
            Decision::NeedAuthority
        );
        assert_eq!(
            proof.decide(proof.started, &snapshot, Some(3)),
            Decision::Revoked
        );
        assert_eq!(
            proof.decide(proof.started, &snapshot, None),
            Decision::Revoked
        );
        for epoch in [None, Some(0)] {
            let snapshot = Snapshot { epoch, ..snapshot };
            let expected = if epoch.is_some() {
                Decision::Live
            } else {
                Decision::NeedAuthority
            };
            assert_eq!(proof.decide(proof.started, &snapshot, None), expected);
        }
    }
}
