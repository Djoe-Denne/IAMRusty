use std::{cmp::Ordering, time::Duration};
use uuid::Uuid;

const MAX_PROOF_AGE: Duration = Duration::from_secs(3);

#[derive(Clone, Copy)]
pub(super) struct Proof {
    pub(super) challenge: Uuid,
    pub(super) generation: Uuid,
    pub(super) started: Duration,
}

pub(super) struct Snapshot {
    pub(super) ready_generation: Option<Uuid>,
    pub(super) applied_challenge: Option<Uuid>,
    pub(super) epoch: Option<u64>,
}

#[derive(Debug, PartialEq, Eq)]
enum Decision {
    Live,
    Revoked,
    NeedAuthority,
}

impl Proof {
    fn decide(&self, now: Duration, snapshot: &Snapshot, claim: Option<u64>) -> Decision {
        if !now
            .checked_sub(self.started)
            .is_some_and(|age| age < MAX_PROOF_AGE)
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
