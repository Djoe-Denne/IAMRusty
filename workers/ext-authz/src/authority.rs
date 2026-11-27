use crate::clock::{boot_time, within_boot_budget};
use rustycog::http::{
    SessionLivenessChecker, SessionLivenessError, SessionLivenessOutcome, SessionLivenessQuery,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};
use tokio::{
    sync::{watch, Semaphore},
    time::{timeout_at, Instant},
};
use uuid::Uuid;

type QueryKey = (String, Uuid, Option<u64>);
type SharedDecision = Option<Completed>;

#[derive(Clone, Copy)]
struct Completed {
    result: Result<SessionLivenessOutcome, SharedError>,
    deadline: Instant,
    boot_started: Option<Duration>,
}

#[derive(Clone, Copy)]
enum SharedError {
    Revoked,
    Unavailable,
}

struct Capacity {
    calls: HashMap<QueryKey, watch::Receiver<SharedDecision>>,
    next_dispatch: Instant,
}

/// Bounds authoritative requests per worker; only identical in-flight queries
/// share a response. Completed responses are never available to new requests.
pub struct Authority {
    upstream: Arc<dyn SessionLivenessChecker>,
    capacity: Arc<Mutex<Capacity>>,
    waiters: Semaphore,
}

const MAX_OUTSTANDING: usize = 8;
const MAX_WAITERS: usize = 128;
const DISPATCH_SPACING: Duration = Duration::from_millis(25);
const UPSTREAM_BUDGET: Duration = Duration::from_millis(600);

impl Authority {
    pub(super) fn new(upstream: Arc<dyn SessionLivenessChecker>) -> Self {
        Self {
            upstream,
            capacity: Arc::new(Mutex::new(Capacity {
                calls: HashMap::new(),
                next_dispatch: Instant::now(),
            })),
            waiters: Semaphore::new(MAX_WAITERS),
        }
    }

    fn dispatch(
        &self,
        query: &SessionLivenessQuery,
    ) -> Result<watch::Receiver<SharedDecision>, SessionLivenessError> {
        let key = (query.issuer.clone(), query.subject, query.auth_epoch);
        let mut capacity = self.capacity.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(existing) = capacity.calls.get(&key) {
            return Ok(existing.clone());
        }
        let now = Instant::now();
        if capacity.calls.len() >= MAX_OUTSTANDING || now < capacity.next_dispatch {
            return Err(SessionLivenessError::unavailable(
                "authority budget exhausted",
            ));
        }
        capacity.next_dispatch = now + DISPATCH_SPACING;
        let (sender, receiver) = watch::channel(None);
        capacity.calls.insert(key.clone(), receiver.clone());
        drop(capacity);

        let capacity = self.capacity.clone();
        let upstream = self.upstream.clone();
        let query = query.clone();
        let deadline = now + UPSTREAM_BUDGET;
        let boot_started = boot_time();
        tokio::spawn(async move {
            let result = if Instant::now() >= deadline {
                Err(SharedError::Unavailable)
            } else {
                match timeout_at(deadline, upstream.check(&query)).await {
                    Ok(Ok(value)) if Instant::now() < deadline => Ok(value),
                    Ok(Err(SessionLivenessError::Revoked)) => Err(SharedError::Revoked),
                    _ => Err(SharedError::Unavailable),
                }
            };
            capacity
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .calls
                .remove(&key);
            let _ = sender.send(Some(Completed {
                result,
                deadline,
                boot_started,
            }));
        });
        Ok(receiver)
    }
}

#[async_trait::async_trait]
impl SessionLivenessChecker for Authority {
    async fn check(
        &self,
        query: &SessionLivenessQuery,
    ) -> Result<SessionLivenessOutcome, SessionLivenessError> {
        let _waiter = self
            .waiters
            .try_acquire()
            .map_err(|_| SessionLivenessError::unavailable("too many authority waiters"))?;
        let mut receiver = self.dispatch(query)?;
        let result = receiver
            .wait_for(Option::is_some)
            .await
            .map_err(|_| SessionLivenessError::unavailable("authority closed"))?;
        match *result {
            Some(completed) if completed.is_fresh() => match completed.result {
                Ok(value) => Ok(value),
                Err(SharedError::Revoked) => Err(SessionLivenessError::Revoked),
                Err(SharedError::Unavailable) => {
                    Err(SessionLivenessError::unavailable("authority failed"))
                }
            },
            _ => Err(SessionLivenessError::unavailable(
                "no fresh authoritative decision",
            )),
        }
    }
}

impl Completed {
    fn is_fresh(&self) -> bool {
        Instant::now() < self.deadline && within_boot_budget(self.boot_started, UPSTREAM_BUDGET)
    }
}

#[cfg(test)]
mod tests {
    use super::{Authority, DISPATCH_SPACING, MAX_OUTSTANDING, UPSTREAM_BUDGET};
    use rustycog::http::{
        SessionLivenessChecker, SessionLivenessDecision, SessionLivenessError,
        SessionLivenessOutcome, SessionLivenessQuery,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::sync::Semaphore;
    use uuid::Uuid;

    struct HeldAuthority {
        calls: AtomicUsize,
        release: Semaphore,
    }

    #[async_trait::async_trait]
    impl SessionLivenessChecker for HeldAuthority {
        async fn check(
            &self,
            query: &SessionLivenessQuery,
        ) -> Result<SessionLivenessOutcome, SessionLivenessError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.release
                .acquire()
                .await
                .expect("fixture semaphore")
                .forget();
            let decision = if query.auth_epoch == Some(1) {
                SessionLivenessDecision::Live
            } else {
                SessionLivenessDecision::Revoked
            };
            Ok(SessionLivenessOutcome::with_current_epoch(decision, 1))
        }
    }

    #[tokio::test(start_paused = true)]
    async fn shares_only_identical_pending_claims_and_drops_completed_decisions() {
        let upstream = Arc::new(HeldAuthority {
            calls: AtomicUsize::new(0),
            release: Semaphore::new(0),
        });
        let authority = Authority::new(upstream.clone());
        let query = SessionLivenessQuery {
            issuer: "https://iam.example.test".into(),
            subject: Uuid::new_v4(),
            auth_epoch: Some(1),
        };
        let mut live = authority.dispatch(&query).expect("start live query");
        let mut shared = authority.dispatch(&query).expect("share identical query");
        let old = SessionLivenessQuery {
            auth_epoch: Some(0),
            ..query.clone()
        };
        assert!(
            authority.dispatch(&old).is_err(),
            "rate cap arrives before a second lookup"
        );
        tokio::time::advance(DISPATCH_SPACING).await;
        let mut revoked = authority
            .dispatch(&old)
            .expect("different epoch has its own lookup");
        upstream.release.add_permits(2);
        for (receiver, expected) in [
            (&mut live, SessionLivenessDecision::Live),
            (&mut shared, SessionLivenessDecision::Live),
            (&mut revoked, SessionLivenessDecision::Revoked),
        ] {
            let outcome = *receiver
                .wait_for(Option::is_some)
                .await
                .expect("query completed");
            assert!(
                matches!(outcome.map(|completed| completed.result), Some(Ok(value)) if value.decision == expected)
            );
        }
        assert_eq!(upstream.calls.load(Ordering::SeqCst), 2);
        tokio::time::advance(DISPATCH_SPACING).await;
        upstream.release.add_permits(1);
        assert!(authority.check(&query).await.is_ok());
        assert_eq!(
            upstream.calls.load(Ordering::SeqCst),
            3,
            "no completed allow cache"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn rejects_excess_outstanding_calls_and_releases_capacity_on_timeout() {
        let upstream = Arc::new(HeldAuthority {
            calls: AtomicUsize::new(0),
            release: Semaphore::new(0),
        });
        let authority = Authority::new(upstream.clone());
        let mut query = SessionLivenessQuery {
            issuer: "https://iam.example.test".into(),
            subject: Uuid::new_v4(),
            auth_epoch: None,
        };
        for _ in 0..MAX_OUTSTANDING {
            query.subject = Uuid::new_v4();
            authority.dispatch(&query).expect("capacity remains");
            tokio::time::advance(DISPATCH_SPACING).await;
        }
        query.subject = Uuid::new_v4();
        assert!(authority.dispatch(&query).is_err());
        tokio::time::advance(UPSTREAM_BUDGET).await;
        tokio::task::yield_now().await;
        upstream.release.add_permits(1);
        assert!(
            authority.check(&query).await.is_ok(),
            "timeouts free all slots"
        );
    }
}
