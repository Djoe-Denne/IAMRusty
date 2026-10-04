//! Bounded writer cleanup of expired OAuth secrets; lifecycle belongs to the root.

use iam_domain::{
    entity::oauth_transaction::OAuthTransactionError,
    port::repository::OAuthTransactionWriteRepository,
};
use std::{sync::Arc, time::Duration};
use tokio::{sync::watch, time::MissedTickBehavior};

#[derive(Clone, Copy, Debug)]
pub struct OAuthTransactionCleanupPolicy {
    pub interval: Duration,
    pub batch_size: u32,
}

impl Default for OAuthTransactionCleanupPolicy {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(60),
            batch_size: 100,
        }
    }
}

pub struct OAuthTransactionCleanup {
    writer: Arc<dyn OAuthTransactionWriteRepository>,
    policy: OAuthTransactionCleanupPolicy,
}

impl OAuthTransactionCleanup {
    /// # Errors
    /// Rejects a zero interval or batch size outside 1..=1000.
    pub fn new(
        writer: Arc<dyn OAuthTransactionWriteRepository>,
        policy: OAuthTransactionCleanupPolicy,
    ) -> Result<Self, OAuthTransactionError> {
        if policy.interval.is_zero() || !(1..=1000).contains(&policy.batch_size) {
            return Err(OAuthTransactionError::InvalidTransaction);
        }
        Ok(Self { writer, policy })
    }

    /// One batch immediately, then one per tick (no unbounded drain). A writer
    /// error emits only a fixed event and retries next tick. Closing the stop
    /// channel or sending true also cancels in-flight writer I/O cooperatively.
    /// No tasks are spawned here; the root owns and joins the single run handle.
    /// Backlog or writer failure may delay deletion beyond TTL plus one interval.
    ///
    /// # Errors
    /// Reserved for fatal helper failures; ordinary writer failures are retried.
    pub async fn run(&self, mut stop: watch::Receiver<bool>) -> Result<(), OAuthTransactionError> {
        let mut ticks = tokio::time::interval(self.policy.interval);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut last_success = None;
        loop {
            if *stop.borrow() || stop.has_changed().is_err() {
                return Ok(());
            }
            tokio::select! {
                biased;
                changed = stop.changed() => {
                    if changed.is_err() || *stop.borrow() { return Ok(()) }
                    continue;
                }
                _ = ticks.tick() => {}
            }
            // A false watch update must not cancel and restart the batch, which
            // could exceed the one-batch-per-tick cap. Pin the same future.
            let batch = self.writer.purge_expired(self.policy.batch_size);
            tokio::pin!(batch);
            let result = loop {
                tokio::select! {
                    biased;
                    changed = stop.changed() => {
                        if changed.is_err() || *stop.borrow() { return Ok(()) }
                    }
                    result = &mut batch => break result,
                }
            };
            match result {
                Ok(deleted) => {
                    let succeeded_at = chrono::Utc::now().timestamp();
                    last_success = Some(succeeded_at);
                    tracing::info!(
                        event = "oauth_cleanup_success",
                        deleted,
                        last_success = succeeded_at,
                        "OAuth transaction cleanup completed"
                    );
                }
                Err(_) => {
                    tracing::warn!(
                        event = "oauth_cleanup_failure",
                        last_success,
                        "OAuth transaction cleanup failed; retry next tick"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use iam_domain::entity::oauth_transaction::{ConsumeOAuthTransaction, OAuthTransaction};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };
    use tokio::sync::Notify;

    #[derive(Default)]
    struct Writer {
        calls: AtomicUsize,
        called: Notify,
        blocked: bool,
        block_first: bool,
        release: Notify,
        fail_first: bool,
        sizes: Mutex<Vec<u32>>,
    }
    #[async_trait]
    impl OAuthTransactionWriteRepository for Writer {
        async fn create(&self, _: &OAuthTransaction) -> Result<(), OAuthTransactionError> {
            unreachable!()
        }
        async fn consume(
            &self,
            _: &ConsumeOAuthTransaction,
        ) -> Result<Option<OAuthTransaction>, OAuthTransactionError> {
            unreachable!()
        }
        async fn purge_expired(&self, size: u32) -> Result<u64, OAuthTransactionError> {
            self.sizes.lock().unwrap().push(size);
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            self.called.notify_one();
            if self.blocked {
                std::future::pending::<()>().await;
            }
            if self.block_first && call == 0 {
                self.release.notified().await;
            }
            if self.fail_first && call == 0 {
                Err(OAuthTransactionError::Storage)
            } else {
                Ok(1)
            }
        }
    }

    #[test]
    fn cleanup_policy_rejects_invalid_bounds_and_defaults_to_60s_100() {
        let policy = OAuthTransactionCleanupPolicy::default();
        assert_eq!(policy.interval, Duration::from_secs(60));
        assert_eq!(policy.batch_size, 100);
        for (interval, batch_size) in [
            (Duration::ZERO, 100),
            (policy.interval, 0),
            (policy.interval, 1001),
        ] {
            assert!(OAuthTransactionCleanup::new(
                Arc::new(Writer::default()),
                OAuthTransactionCleanupPolicy {
                    interval,
                    batch_size
                }
            )
            .is_err());
        }
    }

    #[tokio::test(start_paused = true)]
    async fn cleanup_immediate_batch_retries_next_tick_then_stops_without_spawning() {
        let writer = Arc::new(Writer {
            fail_first: true,
            ..Writer::default()
        });
        let cleanup = OAuthTransactionCleanup::new(
            writer.clone(),
            OAuthTransactionCleanupPolicy {
                interval: Duration::from_secs(60),
                batch_size: 7,
            },
        )
        .unwrap();
        let (tx, rx) = watch::channel(false);
        let observe = async {
            writer.called.notified().await;
            assert_eq!(writer.calls.load(Ordering::SeqCst), 1);
            tokio::time::advance(Duration::from_secs(60)).await;
            writer.called.notified().await;
            tx.send(true).unwrap();
        };
        let (result, ()) = tokio::join!(cleanup.run(rx), observe);
        result.unwrap();
        assert_eq!(*writer.sizes.lock().unwrap(), vec![7, 7]);
    }

    #[tokio::test(start_paused = true)]
    async fn cleanup_skips_missed_ticks_and_false_stop_update_does_not_restart_batch() {
        let writer = Arc::new(Writer {
            block_first: true,
            ..Writer::default()
        });
        let cleanup =
            OAuthTransactionCleanup::new(writer.clone(), OAuthTransactionCleanupPolicy::default())
                .unwrap();
        let (tx, rx) = watch::channel(false);
        let observe = async {
            writer.called.notified().await;
            tx.send(false).unwrap();
            tokio::time::advance(Duration::from_secs(600)).await;
            assert_eq!(writer.calls.load(Ordering::SeqCst), 1);
            writer.release.notify_one();
            writer.called.notified().await;
            tokio::time::advance(Duration::from_secs(1)).await;
            // Delay uses one catch-up tick, never ten draining batches.
            assert_eq!(writer.calls.load(Ordering::SeqCst), 2);
            tx.send(true).unwrap();
        };
        let (result, ()) = tokio::join!(cleanup.run(rx), observe);
        result.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn cleanup_shutdown_cancels_inflight_io_or_tick_and_closed_channel() {
        for blocked in [false, true] {
            let writer = Arc::new(Writer {
                blocked,
                ..Writer::default()
            });
            let cleanup = OAuthTransactionCleanup::new(
                writer.clone(),
                OAuthTransactionCleanupPolicy::default(),
            )
            .unwrap();
            let (tx, rx) = watch::channel(false);
            let stop = async {
                writer.called.notified().await;
                tx.send(true).unwrap();
            };
            let (result, ()) = tokio::join!(cleanup.run(rx), stop);
            result.unwrap();
            assert_eq!(writer.calls.load(Ordering::SeqCst), 1);
        }
        let writer = Arc::new(Writer::default());
        let cleanup =
            OAuthTransactionCleanup::new(writer.clone(), OAuthTransactionCleanupPolicy::default())
                .unwrap();
        let (tx, rx) = watch::channel(false);
        drop(tx);
        cleanup.run(rx).await.unwrap();
        assert_eq!(writer.calls.load(Ordering::SeqCst), 0);
    }
}
