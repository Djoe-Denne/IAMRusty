//! AC4 and real SQS lifecycle proofs; no permissive authorization arrange.
mod common;
#[path = "fixtures/outbox_runtime.rs"]
mod runtime_fixture;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use async_trait::async_trait;
use common::{TelegraphTestDescriptor, TelegraphTestFixture};
use readiness::{create_signaled_multi_queue_event_publisher, ComponentStatus};
use runtime_fixture::*;
use rustycog::config::{KafkaConfig, QueueConfig};
use rustycog::core::error::ServiceError;
use rustycog::events::{
    ConcreteEventPublisher, DomainEvent, EventConsumer, EventHandler, EventPublisher,
};
use rustycog::outbox::entity::{STATUS_PENDING, STATUS_PUBLISHED};
use serial_test::serial;
use telegraph_domain::NotificationService;
use telegraph_infra::event::TelegraphErrorMapper;
use telegraph_setup::runtime::shutdown_owned_tasks;

struct HttpRequestOwner {
    destroyed: Arc<AtomicUsize>,
    dropped: Arc<tokio::sync::Notify>,
}

impl Drop for HttpRequestOwner {
    fn drop(&mut self) {
        self.destroyed.fetch_add(1, Ordering::SeqCst);
        self.dropped.notify_one();
    }
}

/// C-1 / E1: joining the HTTP parent must also destroy its in-flight request.
#[tokio::test]
#[serial]
async fn http_shutdown_destroys_inflight_handler_before_return_without_late_mutation(
) -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let dropped = Arc::new(tokio::sync::Notify::new());
    let destroyed = Arc::new(AtomicUsize::new(0));
    let mutations = Arc::new(AtomicUsize::new(0));
    let router = {
        let (entered, release, dropped, destroyed, mutations) = (
            entered.clone(),
            release.clone(),
            dropped.clone(),
            destroyed.clone(),
            mutations.clone(),
        );
        axum::Router::new().route(
            "/c1-inflight",
            axum::routing::get(move || {
                let (entered, release, dropped, destroyed, mutations) = (
                    entered.clone(),
                    release.clone(),
                    dropped.clone(),
                    destroyed.clone(),
                    mutations.clone(),
                );
                async move {
                    let _owner = HttpRequestOwner { destroyed, dropped };
                    entered.notify_one();
                    release.notified().await;
                    mutations.fetch_add(1, Ordering::SeqCst);
                    axum::http::StatusCode::NO_CONTENT
                }
            }),
        )
    };
    let config = rustycog::config::ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        tls_enabled: false,
        ..Default::default()
    };
    // E2 / G-1: same owner and cleanup helper adopted by standalone and host.
    // The historical legacy E1 source/log remains archived unchanged.
    let mut http = rustycog::http::start_router_owned(router, config).await?;
    let address = http.local_addrs()[0];
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let parent_destroyed = Arc::new(AtomicUsize::new(0));
    let parent_owner = PollerOwner(parent_destroyed.clone());
    let mut tasks = vec![tokio::spawn(async move {
        let _owner = parent_owner;
        let _ = stopped.await;
        Ok(())
    })];
    let setup = tokio::time::timeout(Duration::from_secs(3), async {
        let mut socket = loop {
            match tokio::net::TcpStream::connect(address).await {
                Ok(socket) => break socket,
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
                    anyhow::ensure!(!tasks[0].is_finished(), "HTTP server exited during setup");
                    tokio::task::yield_now().await;
                }
                Err(error) => return Err(error.into()),
            }
        };
        socket
            .write_all(b"GET /c1-inflight HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await?;
        entered.notified().await;
        Ok::<_, anyhow::Error>(socket)
    })
    .await;

    let started = tokio::time::Instant::now();
    let shutdown = telegraph_setup::runtime::shutdown_owned_runtime(&mut tasks, &mut http, async {
        stop.send(())
            .map_err(|()| anyhow::anyhow!("HTTP parent already stopped"))
    })
    .await;
    let elapsed = started.elapsed();
    let destroyed_at_return = destroyed.load(Ordering::SeqCst);
    let mutations_at_return = mutations.load(Ordering::SeqCst);

    // Always release and observe the request before asserting the red oracle.
    // On the old SDK this lets the detached handler finish instead of leaking it.
    release.notify_one();
    let mut response = Vec::new();
    let setup = setup
        .map_err(|error| anyhow::anyhow!("HTTP entry setup timeout: {error}"))
        .and_then(std::convert::identity);
    let (setup_error, cleanup) = match setup {
        Ok(mut socket) => {
            let cleanup = tokio::time::timeout(Duration::from_secs(3), async {
                if destroyed.load(Ordering::SeqCst) == 0 {
                    dropped.notified().await;
                }
                socket.read_to_end(&mut response).await?;
                socket.shutdown().await?;
                Ok::<_, anyhow::Error>(())
            })
            .await;
            (None, cleanup)
        }
        Err(error) => (Some(error), Ok(Ok(()))),
    };
    // Defensive cleanup of any parent left behind by a shutdown error.
    for task in &tasks {
        task.abort();
    }
    for task in tasks.drain(..) {
        let _ = tokio::time::timeout(Duration::from_secs(1), task).await;
    }
    let destroyed_after_cleanup = destroyed.load(Ordering::SeqCst);
    let mutations_after_cleanup = mutations.load(Ordering::SeqCst);
    eprintln!("C1_E1 elapsed_ms={} shutdown={shutdown:?} destroyed_at_return={destroyed_at_return} mutations_at_return={mutations_at_return} destroyed_after_cleanup={destroyed_after_cleanup} mutations_after_cleanup={mutations_after_cleanup} parent_destroyed={} response_bytes={}", elapsed.as_millis(), parent_destroyed.load(Ordering::SeqCst), response.len());
    if let Some(error) = setup_error {
        return Err(error.context("HTTP setup, not a production red"));
    }
    cleanup.map_err(|error| anyhow::anyhow!("HTTP request cleanup timeout: {error}"))??;
    assert!(
        shutdown?.forced,
        "blocked handler requires force, not parent-only completion"
    );
    assert_eq!(
        destroyed_after_cleanup, 1,
        "request owner cleaned before assertion"
    );
    assert_eq!(
        parent_destroyed.load(Ordering::SeqCst),
        1,
        "HTTP parent joined"
    );
    assert!(
        elapsed < Duration::from_secs(6),
        "C-1 global six-second budget exceeded: {elapsed:?}"
    );
    assert_eq!((destroyed_at_return, mutations_after_cleanup - mutations_at_return), (1, 0),
        "C-1: HTTP shutdown returned before destroying the handler, permitting mutation after return");
    Ok(())
}

#[tokio::test]
#[serial]
async fn disabled_and_factory_degraded_noop_preserve_intentions_and_http_then_rebuild_drains() {
    let fixture = TelegraphTestFixture::new(Arc::new(TelegraphTestDescriptor))
        .await
        .unwrap();
    let db = fixture.db();
    let real = config();
    for degraded in [false, true] {
        let mut suspended = real.clone();
        suspended.queue = if degraded {
            // This workspace deliberately has no Kafka feature. Exercise the
            // actual factory fallback, not an injected no-op or a false flag.
            QueueConfig::Kafka(KafkaConfig {
                enabled: true,
                ..KafkaConfig::default()
            })
        } else {
            QueueConfig::Disabled
        };
        let signaled = create_signaled_multi_queue_event_publisher(
            "telegraph",
            &suspended.queue,
            None,
            Arc::new(TelegraphErrorMapper),
        )
        .await
        .unwrap();
        assert!(matches!(
            signaled.transport.as_ref(),
            ConcreteEventPublisher::NoOp(_)
        ));
        assert!(if degraded {
            matches!(signaled.status, ComponentStatus::Degraded { .. })
        } else {
            signaled.status == ComponentStatus::Disabled
        });
        let app = RunningApp::start(suspended).await;
        assert_eq!(
            app.tasks.len(),
            0,
            "no consumer or dispatcher parent for either no-op"
        );
        app.assert_http_healthy().await;
        let created = service(db.clone())
            .create_notification(notification())
            .await
            .unwrap();
        let id = created.id.unwrap();
        let before = row(db.as_ref(), id).await;
        assert_eq!(before.status, STATUS_PENDING);
        let event: telegraph_events::TelegraphDomainEvent =
            serde_json::from_value(before.payload_json.clone()).unwrap();
        signaled.publisher.publish(&event).await.unwrap();
        // Its Ok above is deliberately not a transport success or a published row.
        // More than two default dispatcher poll opportunities.
        tokio::time::sleep(Duration::from_millis(4300)).await;
        let after = row(db.as_ref(), id).await;
        assert_eq!((after.status.as_str(), after.attempts), (STATUS_PENDING, 0));
        assert_eq!(after.payload_json, before.payload_json);
        assert_eq!(after.event_id, before.event_id);
        assert!(fixture
            .sqs()
            .receive_messages_from_queue("test-telegraph-outbound", 10, 0)
            .await
            .unwrap()
            .is_empty());
        assert!(reqwest::get(format!("{}/health", app.url))
            .await
            .unwrap()
            .status()
            .is_success());
        app.stop().await;
        let live = RunningApp::start(real.clone()).await;
        assert_eq!(
            live.tasks.len(),
            2,
            "consumer and dispatcher; HTTP owned separately"
        );
        live.assert_http_healthy().await;
        let published = wait_status(db.as_ref(), id, STATUS_PUBLISHED).await;
        assert_eq!(published.event_id, before.event_id);
        assert_eq!(published.payload_json, before.payload_json);
        let messages = fixture
            .sqs()
            .receive_messages_from_queue("test-telegraph-outbound", 10, 2)
            .await
            .unwrap();
        assert_eq!(messages.len(), 1);
        let envelope: serde_json::Value =
            serde_json::from_str(messages[0].body().unwrap()).unwrap();
        assert_eq!(envelope["event_id"], before.event_id.to_string());
        let url = fixture
            .sqs()
            .queue_url_for("test-telegraph-outbound")
            .unwrap();
        fixture
            .sqs()
            .client
            .delete_message()
            .queue_url(url)
            .receipt_handle(messages[0].receipt_handle().unwrap())
            .send()
            .await
            .unwrap();
        live.stop().await;
    }
}

struct SuspendedPublisher {
    entered: tokio::sync::Notify,
    destroyed: Arc<AtomicUsize>,
}
#[async_trait]
impl EventPublisher<telegraph_domain::DomainError> for SuspendedPublisher {
    async fn publish(&self, _: &dyn DomainEvent) -> Result<(), telegraph_domain::DomainError> {
        let _owner = PollerOwner(self.destroyed.clone());
        self.entered.notify_one();
        std::future::pending().await
    }
    async fn publish_batch(
        &self,
        _: &[Box<dyn DomainEvent>],
    ) -> Result<(), telegraph_domain::DomainError> {
        panic!("dispatcher uses single-event publish")
    }
    async fn health_check(&self) -> Result<(), telegraph_domain::DomainError> {
        Ok(())
    }
}

#[tokio::test]
#[serial]
async fn shutdown_during_publish_destroys_future_and_preserves_unpublished_same_id_for_new_dispatcher(
) {
    let fixture = TelegraphTestFixture::new(Arc::new(TelegraphTestDescriptor))
        .await
        .unwrap();
    let db = fixture.db();
    let created = service(db.clone())
        .create_notification(notification())
        .await
        .unwrap();
    let id = created.id.unwrap();
    let before = row(db.as_ref(), id).await;
    let publisher = Arc::new(SuspendedPublisher {
        entered: tokio::sync::Notify::new(),
        destroyed: Arc::new(AtomicUsize::new(0)),
    });
    let pool = rustycog::db::DbConnectionPool::new(&config().database)
        .await
        .unwrap();
    let dispatcher = Arc::new(rustycog::outbox::OutboxDispatcher::new(
        pool.clone(),
        publisher.clone(),
        rustycog::outbox::OutboxConfig {
            lock_timeout: Duration::from_millis(100),
            poll_interval: Duration::from_millis(20),
            ..Default::default()
        },
    ));
    let running = dispatcher.clone();
    let parent_destroyed = Arc::new(AtomicUsize::new(0));
    let parent_owner = PollerOwner(parent_destroyed.clone());
    let mut tasks = vec![tokio::spawn(async move {
        let _owner = parent_owner;
        running.start().await.map_err(Into::into)
    })];
    tokio::time::timeout(Duration::from_secs(3), publisher.entered.notified())
        .await
        .unwrap();
    let started = tokio::time::Instant::now();
    shutdown_owned_tasks(&mut tasks, async {
        dispatcher.stop().await.map_err(Into::into)
    })
    .await
    .unwrap();
    assert!(started.elapsed() < Duration::from_secs(6));
    assert!(tasks.is_empty());
    assert_eq!(publisher.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(parent_destroyed.load(Ordering::SeqCst), 1);
    let interrupted = row(db.as_ref(), id).await;
    assert_eq!(
        interrupted.status,
        rustycog::outbox::entity::STATUS_PUBLISHING
    );
    assert_eq!(interrupted.event_id, before.event_id);
    assert_eq!(interrupted.payload_json, before.payload_json);
    drop(dispatcher);
    let live = create_signaled_multi_queue_event_publisher(
        "telegraph",
        &config().queue,
        None,
        Arc::new(TelegraphErrorMapper),
    )
    .await
    .unwrap();
    assert!(matches!(live.status, ComponentStatus::Live { .. }));
    assert!(!matches!(
        live.transport.as_ref(),
        ConcreteEventPublisher::NoOp(_)
    ));
    let resumed = rustycog::outbox::OutboxDispatcher::new(pool, live.publisher, Default::default());
    assert_eq!(resumed.dispatch_once().await.unwrap(), 1);
    let after = row(db.as_ref(), id).await;
    assert_eq!(after.status, STATUS_PUBLISHED);
    assert_eq!(after.event_id, before.event_id);
    let messages = fixture
        .sqs()
        .receive_messages_from_queue("test-telegraph-outbound", 10, 2)
        .await
        .unwrap();
    assert_eq!(messages.len(), 1);
    let envelope: serde_json::Value = serde_json::from_str(messages[0].body().unwrap()).unwrap();
    assert_eq!(envelope["event_id"], before.event_id.to_string());
}

// G-C117: prospective component witness, NOT a replay of SQL cancellation109
// or a replacement for the vertical recovery oracle. Existing witnesses intact.
fn claim117_trace(stage: &str, started: tokio::time::Instant, data: serde_json::Value) {
    use std::io::Write;
    let mut output = std::io::stdout().lock();
    let value = serde_json::json!({"stage": stage, "utc": chrono::Utc::now(),
        "elapsed_us": started.elapsed().as_micros(), "data": data});
    let _ = writeln!(output, "TELEGRAPH_CONTROLLED_CLAIM117 {value}");
    let _ = output.flush();
}

async fn claim117_row(
    db: &sea_orm::DatabaseConnection,
    id: uuid::Uuid,
    started: tokio::time::Instant,
    stage: &str,
) -> anyhow::Result<rustycog::outbox::entity::Model> {
    use sea_orm::EntityTrait;
    let before = chrono::Utc::now();
    let before_us = started.elapsed().as_micros();
    let row = rustycog::outbox::entity::OutboxEvents::find_by_id(id)
        .one(db)
        .await
        .map_err(|_| anyhow::anyhow!("controlled claim SQL read failed"))?
        .ok_or_else(|| anyhow::anyhow!("controlled claim row missing"))?;
    claim117_trace(
        stage,
        started,
        serde_json::json!({
            "read_before_utc": before, "read_after_utc": chrono::Utc::now(),
            "read_before_us": before_us, "read_after_us": started.elapsed().as_micros(),
            "outbox_id": row.id, "event_id": row.event_id, "aggregate_id": row.aggregate_id,
            "status": row.status, "attempts": row.attempts, "next_attempt_at": row.next_attempt_at,
            "locked_by": row.locked_by, "locked_until": row.locked_until, "updated_at": row.updated_at,
            "error_present": row.last_error.is_some(),
            "error_nonempty": row.last_error.as_deref().is_some_and(|error| !error.is_empty()),
        }),
    );
    Ok(row)
}

#[tokio::test]
#[serial]
async fn controlled_committed_claim_cancellation_keeps_default_lock_through_25s_observation(
) -> anyhow::Result<()> {
    use rustycog::outbox::{OutboxConfig, OutboxDispatcher};
    use telegraph_setup::runtime::OwnedTaskAbortGuard;
    let fixture = TelegraphTestFixture::new(Arc::new(TelegraphTestDescriptor))
        .await
        .map_err(|_| anyhow::anyhow!("controlled claim fixture setup failed"))?;
    let db = fixture.db();
    let started = tokio::time::Instant::now();
    let created = service(db.clone())
        .create_notification(notification())
        .await
        .map_err(|_| anyhow::anyhow!("controlled claim notification creation failed"))?;
    let id = created
        .id
        .ok_or_else(|| anyhow::anyhow!("notification id missing"))?;
    let before = row(db.as_ref(), id).await;
    let old_config = OutboxConfig::default();
    let new_config = OutboxConfig::default();
    assert_eq!(old_config.lock_timeout, Duration::from_secs(30));
    assert_eq!(old_config.poll_interval, Duration::from_secs(2));
    assert_eq!(old_config.retry_base_delay, Duration::from_secs(5));
    assert_eq!(old_config.max_attempts, 20);
    let old_worker = old_config.worker_id.clone();
    let new_worker = new_config.worker_id.clone();
    let poll = new_config.poll_interval;
    let pool = rustycog::db::DbConnectionPool::new(&config().database)
        .await
        .map_err(|_| anyhow::anyhow!("controlled claim pool setup failed"))?;
    let publisher = Arc::new(SuspendedPublisher {
        entered: tokio::sync::Notify::new(),
        destroyed: Arc::new(AtomicUsize::new(0)),
    });
    let dispatcher = Arc::new(OutboxDispatcher::new(
        pool.clone(),
        publisher.clone(),
        old_config,
    ));
    let old_destroyed = Arc::new(AtomicUsize::new(0));
    let owner = PollerOwner(old_destroyed.clone());
    let running = dispatcher.clone();
    let (cancel, cancelled) = tokio::sync::oneshot::channel::<()>();
    // Local owned cancellation boundary AFTER publish entry/committed claim.
    // Not the actual application wrapper, nor the in-flight SQL race from109.
    let mut tasks = vec![tokio::spawn(async move {
        let _owner = owner;
        tokio::select! {
            biased;
            _ = cancelled => Ok(()),
            result = running.start() => result.map_err(Into::into),
        }
    })];
    let _old_abort_on_cancel = OwnedTaskAbortGuard::new(&tasks);
    let claimed = tokio::time::timeout(Duration::from_secs(3), async {
        publisher.entered.notified().await;
        let claimed = claim117_row(
            db.as_ref(),
            before.id,
            started,
            "publisher_entered_claim_committed",
        )
        .await?;
        anyhow::ensure!(
            claimed.status == rustycog::outbox::entity::STATUS_PUBLISHING,
            "publisher entry must follow real publishing claim"
        );
        anyhow::ensure!(
            claimed.event_id == before.event_id
                && claimed.aggregate_id == id
                && claimed.payload_json == before.payload_json,
            "claim identity/payload changed"
        );
        anyhow::ensure!(
            claimed.attempts == before.attempts + 1
                && claimed.locked_by.as_deref() == Some(old_worker.as_str())
                && claimed.locked_until.is_some(),
            "claim attempts/owner/lock not established"
        );
        Ok::<_, anyhow::Error>(claimed)
    })
    .await;
    let stop_started = tokio::time::Instant::now();
    claim117_trace(
        "cancel_owner",
        started,
        serde_json::json!({"worker": old_worker}),
    );
    let stopped = shutdown_owned_tasks(&mut tasks, async {
        let _ = cancel.send(());
        Ok(())
    })
    .await;
    let stop_elapsed = stop_started.elapsed();
    // Cleanup first, even if entry/read/precondition failed.
    stopped?;
    anyhow::ensure!(
        tasks.is_empty() && stop_elapsed < Duration::from_secs(6),
        "old owner stop/join failed"
    );
    anyhow::ensure!(
        publisher.destroyed.load(Ordering::SeqCst) == 1
            && old_destroyed.load(Ordering::SeqCst) == 1,
        "publisher future/parent not destroyed at join"
    );
    let claimed =
        claimed.map_err(|_| anyhow::anyhow!("publisher entry/claim observation timed out"))??;
    let interrupted = claim117_row(db.as_ref(), before.id, started, "old_owner_joined").await?;
    anyhow::ensure!(
        interrupted.status == rustycog::outbox::entity::STATUS_PUBLISHING
            && interrupted.id == claimed.id
            && interrupted.event_id == before.event_id
            && interrupted.payload_json == before.payload_json,
        "durable claim changed across owned cancellation"
    );
    drop(dispatcher);
    let live = create_signaled_multi_queue_event_publisher(
        "telegraph",
        &config().queue,
        None,
        Arc::new(TelegraphErrorMapper),
    )
    .await
    .map_err(|_| anyhow::anyhow!("real publisher reconstruction failed"))?;
    anyhow::ensure!(
        matches!(live.status, ComponentStatus::Live { .. })
            && !matches!(live.transport.as_ref(), ConcreteEventPublisher::NoOp(_)),
        "real transport required"
    );
    let resumed = OutboxDispatcher::new(pool, live.publisher, new_config);
    // The ONLY 25s observation deadline: starts immediately before new owner.
    let window_started = tokio::time::Instant::now();
    let window_utc = chrono::Utc::now();
    let deadline = window_started + Duration::from_secs(25);
    let deadline_utc = window_utc + chrono::Duration::seconds(25);
    let lock_until = interrupted
        .locked_until
        .ok_or_else(|| anyhow::anyhow!("INCONCLUSIVE: no measured lock"))?;
    anyhow::ensure!(
        lock_until > deadline_utc,
        "INCONCLUSIVE: 25s window not contained in measured lock"
    );
    claim117_trace(
        "observation_begin",
        started,
        serde_json::json!({"old_worker": old_worker,
        "new_worker": new_worker, "window_utc": window_utc, "deadline_utc": deadline_utc,
        "locked_until": lock_until, "budget_ms": 25_000}),
    );
    let new_destroyed = Arc::new(AtomicUsize::new(0));
    let cycles_started = Arc::new(AtomicUsize::new(0));
    let cycles_completed = Arc::new(AtomicUsize::new(0));
    let actual_claims = Arc::new(AtomicUsize::new(0));
    let activity = (
        cycles_started.clone(),
        cycles_completed.clone(),
        actual_claims.clone(),
    );
    let owner = PollerOwner(new_destroyed.clone());
    let (cancel, cancelled) = tokio::sync::oneshot::channel::<()>();
    let (cycles, mut observations) = tokio::sync::mpsc::unbounded_channel();
    // Explicit component driver of REAL selections, with default poll2s. This
    // exposes dispatch_once results without adding a production hook to start().
    let mut tasks = vec![tokio::spawn(async move {
        let _owner = owner;
        tokio::select! {
            biased;
            _ = cancelled => Ok(()),
            _ = tokio::time::sleep_until(deadline) => Ok(()),
            result = async {
                loop {
                    activity.0.fetch_add(1, Ordering::SeqCst);
                    let count = resumed.dispatch_once().await
                        .map_err(|_| anyhow::anyhow!("real reconstructed dispatch cycle failed"))?;
                    activity.2.fetch_add(count, Ordering::SeqCst);
                    activity.1.fetch_add(1, Ordering::SeqCst);
                    if cycles.send((count, chrono::Utc::now())).is_err() { return Ok(()); }
                    tokio::time::sleep(poll).await;
                }
            } => result,
        }
    })];
    let _new_abort_on_cancel = OwnedTaskAbortGuard::new(&tasks);
    let observation = async {
        let mut cycles = 0_u64;
        let mut claims = 0_usize;
        let mut unchanged = true;
        // One zero-wait broker observation INSIDE the same window, not a wait
        // for publication after lock expiry. Explicit timing, not a race sleep.
        let broker = async {
            tokio::time::sleep_until(deadline - Duration::from_secs(1)).await;
            fixture.sqs().receive_messages_from_queue("test-telegraph-outbound", 10, 0).await
        };
        tokio::pin!(broker);
        let mut broker_empty = None;
        loop {
            tokio::select! {
                biased;
                _ = tokio::time::sleep_until(deadline) => break,
                messages = &mut broker, if broker_empty.is_none() => {
                    broker_empty = Some(messages.map_err(|_| anyhow::anyhow!("broker observation failed"))?.is_empty());
                }
                cycle = observations.recv() => {
                    let Some((count, cycle_utc)) = cycle else { break; };
                    cycles += 1;
                    claims += count;
                    claim117_trace("real_selection_completed", started,
                        serde_json::json!({"worker": new_worker, "cycle": cycles, "claimed": count, "cycle_utc": cycle_utc}));
                    let state = tokio::time::timeout_at(deadline,
                        claim117_row(db.as_ref(), before.id, started, "during_lock_window")).await
                        .map_err(|_| anyhow::anyhow!("INCONCLUSIVE: SQL observation unfinished at deadline"))??;
                    unchanged &= state.id == before.id && state.event_id == before.event_id
                        && state.aggregate_id == id && state.payload_json == before.payload_json
                        && state.status == rustycog::outbox::entity::STATUS_PUBLISHING
                        && state.attempts == claimed.attempts && state.locked_by == claimed.locked_by
                        && state.locked_until == claimed.locked_until;
                }
            }
        }
        Ok::<_, anyhow::Error>((cycles, claims, unchanged, broker_empty))
    }.await;
    let stop_started = tokio::time::Instant::now();
    let stopped = shutdown_owned_tasks(&mut tasks, async {
        let _ = cancel.send(());
        Ok(())
    })
    .await;
    let stop_elapsed = stop_started.elapsed();
    stopped?;
    anyhow::ensure!(
        tasks.is_empty()
            && new_destroyed.load(Ordering::SeqCst) == 1
            && stop_elapsed < Duration::from_secs(6),
        "new owner not destroyed/joined within budget"
    );
    let (cycles, claims, unchanged, broker_empty) = observation?;
    claim117_trace(
        "component_outcome_not_vertical_acceptance",
        started,
        serde_json::json!({
            "cycles": cycles, "claims": claims, "durable_identity_and_lock_unchanged": unchanged,
            "broker_empty": broker_empty, "window_elapsed_ms": window_started.elapsed().as_millis(),
            "cycles_started": cycles_started.load(Ordering::SeqCst),
            "cycles_completed": cycles_completed.load(Ordering::SeqCst),
            "actual_claims": actual_claims.load(Ordering::SeqCst),
            "lock_until": lock_until, "new_owner_destroyed": new_destroyed.load(Ordering::SeqCst),
        }),
    );
    anyhow::ensure!(
        chrono::Utc::now() < lock_until && tokio::time::Instant::now() >= deadline,
        "INCONCLUSIVE: measured window/cleanup did not stay inside lock"
    );
    anyhow::ensure!(
        cycles >= 2,
        "INCONCLUSIVE: insufficient completed real selection cycles"
    );
    anyhow::ensure!(
        broker_empty.is_some(),
        "INCONCLUSIVE: broker observation missing"
    );
    anyhow::ensure!(
        claims == 0
            && actual_claims.load(Ordering::SeqCst) == 0
            && unchanged
            && broker_empty == Some(true),
        "controlled default-lock prediction contradicted; retain observed state/cycles"
    );
    anyhow::ensure!(
        cycles_started.load(Ordering::SeqCst) == cycles_completed.load(Ordering::SeqCst),
        "INCONCLUSIVE: a real dispatch cycle was cancelled before its result was known"
    );
    Ok(())
}

// G-C122: additive post-expiry AC3 witness; all117 code remains unchanged.
struct Expiry122Publisher {
    inner: Arc<dyn EventPublisher<telegraph_domain::DomainError>>,
    db: Arc<sea_orm::DatabaseConnection>,
    outbox_id: uuid::Uuid,
    started: tokio::time::Instant,
    deadline: tokio::time::Instant,
    claims: std::sync::Mutex<Vec<(uuid::Uuid, Option<rustycog::outbox::entity::Model>)>>,
    successes: AtomicUsize,
}

#[async_trait]
impl EventPublisher<telegraph_domain::DomainError> for Expiry122Publisher {
    async fn publish(&self, event: &dyn DomainEvent) -> Result<(), telegraph_domain::DomainError> {
        if tokio::time::Instant::now() >= self.deadline {
            std::future::pending::<()>().await;
        }
        // Observe the committed new owner BEFORE the real transport clears the
        // lock. Missing observation is retained as None, never synthesized.
        let claim = claim117_row(
            &self.db,
            self.outbox_id,
            self.started,
            "122_claim_before_real_send",
        )
        .await
        .ok();
        self.claims
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((event.event_id(), claim));
        if tokio::time::Instant::now() >= self.deadline {
            std::future::pending::<()>().await;
        }
        let result = self.inner.publish(event).await;
        if result.is_ok() {
            self.successes.fetch_add(1, Ordering::SeqCst);
        }
        claim117_trace(
            "122_real_publish_return",
            self.started,
            serde_json::json!({"event_id": event.event_id(), "success": result.is_ok()}),
        );
        result
    }

    async fn publish_batch(
        &self,
        events: &[Box<dyn DomainEvent>],
    ) -> Result<(), telegraph_domain::DomainError> {
        self.inner.publish_batch(events).await
    }

    async fn health_check(&self) -> Result<(), telegraph_domain::DomainError> {
        self.inner.health_check().await
    }
}

async fn expiry122_counts(
    db: &sea_orm::DatabaseConnection,
    id: uuid::Uuid,
) -> anyhow::Result<(u64, u64, u64)> {
    use rustycog::outbox::entity as outbox;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
    use telegraph_infra::repository::entity::{notification_deliveries, notifications};
    let notifications = notifications::Entity::find_by_id(id)
        .count(db)
        .await
        .map_err(|_| anyhow::anyhow!("notification count unavailable"))?;
    let deliveries = notification_deliveries::Entity::find()
        .filter(notification_deliveries::Column::NotificationId.eq(id))
        .count(db)
        .await
        .map_err(|_| anyhow::anyhow!("delivery count unavailable"))?;
    let events = outbox::OutboxEvents::find()
        .filter(outbox::Column::AggregateId.eq(id))
        .count(db)
        .await
        .map_err(|_| anyhow::anyhow!("outbox count unavailable"))?;
    Ok((notifications, deliveries, events))
}

fn expiry122_clock_coherent(
    t0: tokio::time::Instant,
    utc0: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<()> {
    let elapsed = chrono::Duration::from_std(t0.elapsed())
        .map_err(|_| anyhow::anyhow!("INCONCLUSIVE: monotonic duration unavailable"))?;
    // Conservative measurement precondition, NOT an extension to E/R/D.
    anyhow::ensure!(
        chrono::Utc::now()
            .signed_duration_since(utc0 + elapsed)
            .num_milliseconds()
            .unsigned_abs()
            <= 100,
        "INCONCLUSIVE: UTC/monotonic conversion diverged by more than100ms"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn committed_claim_recovers_same_event_after_measured_expiry_with_fixed_rebuild_deadline(
) -> anyhow::Result<()> {
    use rustycog::outbox::{entity as outbox, OutboxConfig, OutboxDispatcher};
    use telegraph_setup::runtime::OwnedTaskAbortGuard;
    let fixture = TelegraphTestFixture::new(Arc::new(TelegraphTestDescriptor))
        .await
        .map_err(|_| anyhow::anyhow!("122 fixture setup failed"))?;
    let db = fixture.db();
    let trace_started = tokio::time::Instant::now();
    let created = service(db.clone())
        .create_notification(notification())
        .await
        .map_err(|_| anyhow::anyhow!("122 creation failed"))?;
    let id = created
        .id
        .ok_or_else(|| anyhow::anyhow!("notification id missing"))?;
    let before = row(db.as_ref(), id).await;
    let counts = expiry122_counts(&db, id).await?;
    anyhow::ensure!(
        counts == (1, 0, 1),
        "single notification-only intention required"
    );
    let old_config = OutboxConfig::default();
    let new_config = OutboxConfig::default();
    anyhow::ensure!(
        old_config.lock_timeout == Duration::from_secs(30)
            && old_config.poll_interval == Duration::from_secs(2)
            && old_config.retry_base_delay == Duration::from_secs(5)
            && old_config.max_attempts == 20,
        "default dispatcher contract changed"
    );
    let old_worker = old_config.worker_id.clone();
    let new_worker = new_config.worker_id.clone();
    let poll = new_config.poll_interval;
    let max_attempts = new_config.max_attempts;
    let pool = rustycog::db::DbConnectionPool::new(&config().database)
        .await
        .map_err(|_| anyhow::anyhow!("122 pool setup failed"))?;
    let suspended = Arc::new(SuspendedPublisher {
        entered: tokio::sync::Notify::new(),
        destroyed: Arc::new(AtomicUsize::new(0)),
    });
    let old = Arc::new(OutboxDispatcher::new(
        pool.clone(),
        suspended.clone(),
        old_config,
    ));
    let running = old.clone();
    let old_destroyed = Arc::new(AtomicUsize::new(0));
    let owner = PollerOwner(old_destroyed.clone());
    let (cancel, cancelled) = tokio::sync::oneshot::channel::<()>();
    let mut tasks = vec![tokio::spawn(async move {
        let _owner = owner;
        // Controlled local cancellation AFTER committed claim, not SQL109.
        tokio::select! {
            biased;
            _ = cancelled => Ok(()),
            result = running.start() => result.map_err(Into::into),
        }
    })];
    let _old_guard = OwnedTaskAbortGuard::new(&tasks);
    let claimed = tokio::time::timeout(Duration::from_secs(3), async {
        suspended.entered.notified().await;
        let claimed =
            claim117_row(&db, before.id, trace_started, "122_old_claim_committed").await?;
        anyhow::ensure!(
            claimed.status == outbox::STATUS_PUBLISHING
                && claimed.id == before.id
                && claimed.event_id == before.event_id
                && claimed.aggregate_id == id
                && claimed.payload_json == before.payload_json
                && claimed.attempts == before.attempts + 1
                && claimed.locked_by.as_deref() == Some(old_worker.as_str())
                && claimed.locked_until.is_some(),
            "122 real claim precondition failed"
        );
        Ok::<_, anyhow::Error>(claimed)
    })
    .await;
    let stopping = tokio::time::Instant::now();
    let stopped = shutdown_owned_tasks(&mut tasks, async {
        let _ = cancel.send(());
        Ok(())
    })
    .await;
    stopped?;
    anyhow::ensure!(
        tasks.is_empty()
            && stopping.elapsed() < Duration::from_secs(6)
            && suspended.destroyed.load(Ordering::SeqCst) == 1
            && old_destroyed.load(Ordering::SeqCst) == 1,
        "122 old publisher/parent not destroyed and joined"
    );
    let claimed = claimed.map_err(|_| anyhow::anyhow!("122 claim observation timed out"))??;
    drop(old);

    // Reconstruction begins HERE: includes eligibility read, factory and owner.
    let t0 = tokio::time::Instant::now();
    let utc0 = chrono::Utc::now();
    // D cannot be earlier than t0+25. Bound this bootstrap read by that minimum
    // while E is not known, rather than permitting an unbounded initial SQL read.
    let eligible_row = tokio::time::timeout_at(
        t0 + Duration::from_secs(25),
        claim117_row(&db, before.id, trace_started, "122_rebuild_eligibility"),
    )
    .await
    .map_err(|_| anyhow::anyhow!("INCONCLUSIVE: eligibility read timed out"))??;
    expiry122_clock_coherent(t0, utc0)?;
    anyhow::ensure!(
        eligible_row.id == before.id
            && eligible_row.event_id == before.event_id
            && eligible_row.aggregate_id == id
            && eligible_row.payload_json == before.payload_json
            && eligible_row.attempts < max_attempts,
        "INCONCLUSIVE: eligibility identity/attempts invalid"
    );
    let eligibility = match eligible_row.status.as_str() {
        outbox::STATUS_PUBLISHING => eligible_row
            .locked_until
            .ok_or_else(|| anyhow::anyhow!("INCONCLUSIVE: publishing lock missing"))?,
        outbox::STATUS_PENDING | outbox::STATUS_FAILED => eligible_row.next_attempt_at,
        _ => anyhow::bail!("INCONCLUSIVE: unsupported eligibility state"),
    };
    let residual = eligibility
        .signed_duration_since(utc0)
        .to_std()
        .unwrap_or(Duration::ZERO);
    anyhow::ensure!(
        residual <= Duration::from_secs(30),
        "INCONCLUSIVE: residual eligibility exceeds30s"
    );
    let deadline = t0 + residual + Duration::from_secs(25);
    let deadline_utc = utc0
        + chrono::Duration::from_std(residual + Duration::from_secs(25))
            .map_err(|_| anyhow::anyhow!("INCONCLUSIVE: deadline conversion failed"))?;
    claim117_trace(
        "122_fixed_rebuild_budget",
        trace_started,
        serde_json::json!({
            "t0_utc": utc0, "t0_elapsed_us": t0.duration_since(trace_started).as_micros(),
            "eligibility_utc": eligibility, "residual_us": residual.as_micros(), "deadline_utc": deadline_utc,
            "deadline_elapsed_us": deadline.duration_since(trace_started).as_micros(), "clock_tolerance_ms": 100,
        }),
    );
    anyhow::ensure!(
        eligible_row.status == outbox::STATUS_PUBLISHING
            && eligible_row.locked_by.as_deref() == Some(old_worker.as_str())
            && eligible_row.attempts == claimed.attempts
            && residual > Duration::ZERO,
        "INCONCLUSIVE: controlled retained-lock branch unavailable"
    );
    anyhow::ensure!(
        tokio::time::Instant::now() < deadline,
        "122 rebuild deadline already exhausted"
    );
    let live = tokio::time::timeout_at(
        deadline,
        create_signaled_multi_queue_event_publisher(
            "telegraph",
            &config().queue,
            None,
            Arc::new(TelegraphErrorMapper),
        ),
    )
    .await
    .map_err(|_| anyhow::anyhow!("122 factory exceeded fixed deadline"))?
    .map_err(|_| anyhow::anyhow!("122 real publisher factory failed"))?;
    anyhow::ensure!(
        matches!(live.status, ComponentStatus::Live { .. })
            && !matches!(live.transport.as_ref(), ConcreteEventPublisher::NoOp(_)),
        "122 requires real transport"
    );
    anyhow::ensure!(
        tokio::time::Instant::now() < deadline,
        "122 factory consumed fixed rebuild deadline"
    );
    let publisher = Arc::new(Expiry122Publisher {
        inner: live.publisher,
        db: db.clone(),
        outbox_id: before.id,
        started: trace_started,
        deadline,
        claims: std::sync::Mutex::new(Vec::new()),
        successes: AtomicUsize::new(0),
    });
    let resumed = OutboxDispatcher::new(pool, publisher.clone(), new_config);
    let activity = Arc::new((
        AtomicUsize::new(0),
        AtomicUsize::new(0),
        AtomicUsize::new(0),
    ));
    let running_activity = activity.clone();
    let new_destroyed = Arc::new(AtomicUsize::new(0));
    let owner = PollerOwner(new_destroyed.clone());
    let (cancel, cancelled) = tokio::sync::oneshot::channel::<()>();
    let (cycles, mut observations) = tokio::sync::mpsc::unbounded_channel();
    let mut tasks = vec![tokio::spawn(async move {
        let _owner = owner;
        // Component driver, not whole-application start/watch-stop. Every cycle
        // uses the real SDK selection/claim/publish, with unchanged default poll.
        tokio::select! {
            biased;
            _ = cancelled => Ok(()),
            _ = tokio::time::sleep_until(deadline) => Ok(()),
            result = async {
                loop {
                    running_activity.0.fetch_add(1, Ordering::SeqCst);
                    let began = chrono::Utc::now();
                    let count = resumed.dispatch_once().await
                        .map_err(|_| anyhow::anyhow!("122 real dispatch cycle failed"))?;
                    running_activity.2.fetch_add(count, Ordering::SeqCst);
                    running_activity.1.fetch_add(1, Ordering::SeqCst);
                    if cycles.send((count, began, chrono::Utc::now())).is_err() { return Ok(()); }
                    tokio::time::sleep(poll).await;
                }
            } => result,
        }
    })];
    let _new_guard = OwnedTaskAbortGuard::new(&tasks);
    // ALL observations, real Receive and final counts share the SAME D.
    let observation = tokio::time::timeout_at(deadline, async {
        let mut before_cycles = 0_u64;
        let mut after_cycles = 0_u64;
        loop {
            let (count, began, finished) = observations.recv().await
                .ok_or_else(|| anyhow::anyhow!("122 owner ended without publication evidence"))?;
            expiry122_clock_coherent(t0, utc0)?;
            if finished < eligibility {
                before_cycles += 1;
                anyhow::ensure!(count == 0, "122 reclaim completed before eligibility");
            } else { after_cycles += 1; }
            claim117_trace("122_real_cycle", trace_started, serde_json::json!({
                "worker": new_worker, "began_utc": began, "finished_utc": finished, "claimed": count,
                "before_cycles": before_cycles, "after_cycles": after_cycles, "eligibility_utc": eligibility,
            }));
            anyhow::ensure!(tokio::time::Instant::now() < deadline, "122 observation deadline exhausted");
            let state = claim117_row(&db, before.id, trace_started, "122_during_rebuild").await?;
            anyhow::ensure!(state.id == before.id && state.event_id == before.event_id
                && state.aggregate_id == id && state.payload_json == before.payload_json,
                "122 durable identity/payload changed");
            if state.status == outbox::STATUS_PUBLISHED {
                anyhow::ensure!(publisher.successes.load(Ordering::SeqCst) > 0,
                    "122 published before real transport success");
                anyhow::ensure!(tokio::time::Instant::now() < deadline, "122 receive deadline exhausted");
                let messages = fixture.sqs().receive_messages_from_queue("test-telegraph-outbound", 10, 2)
                    .await.map_err(|_| anyhow::anyhow!("122 real SQS receive failed"))?;
                anyhow::ensure!(!messages.is_empty(), "122 real SQS bytes missing");
                for message in &messages {
                    let bytes = message.body().ok_or_else(|| anyhow::anyhow!("122 empty SQS body"))?;
                    let envelope: serde_json::Value = serde_json::from_str(bytes)
                        .map_err(|_| anyhow::anyhow!("122 SQS envelope invalid"))?;
                    anyhow::ensure!(envelope["event_id"] == before.event_id.to_string()
                        && envelope["aggregate_id"] == id.to_string() && envelope["event_type"] == before.event_type
                        && envelope["version"] == before.version && envelope["data"] == before.payload_json,
                        "122 actual SQS envelope identity/version/payload mismatch");
                }
                anyhow::ensure!(tokio::time::Instant::now() < deadline, "122 counts deadline exhausted");
                anyhow::ensure!(expiry122_counts(&db, id).await? == counts, "122 business counts changed");
                // T-1: timeout_at alone does not attest a ready inner future's
                // completion time. Check AFTER the final await, before success.
                let completed_at = tokio::time::Instant::now();
                anyhow::ensure!(completed_at < deadline, "122 observations completed outside fixed deadline");
                return Ok::<_, anyhow::Error>((state, before_cycles, after_cycles, messages.len(), completed_at));
            }
        }
    }).await;
    let stopping = tokio::time::Instant::now();
    let stopped = shutdown_owned_tasks(&mut tasks, async {
        let _ = cancel.send(());
        Ok(())
    })
    .await;
    // No SQL, Receive or retry after D to rescue a failed observation.
    stopped?;
    anyhow::ensure!(
        tasks.is_empty()
            && stopping.elapsed() < Duration::from_secs(6)
            && new_destroyed.load(Ordering::SeqCst) == 1
            && t0.elapsed() <= Duration::from_secs(61),
        "122 new owner/cleanup exceeded bounded reconstruction phase"
    );
    let (published, before_cycles, after_cycles, messages, completed_at) = observation
        .map_err(|_| anyhow::anyhow!("122 fixed E/R/25s publication deadline exceeded"))??;
    expiry122_clock_coherent(t0, utc0)?;
    let claims = publisher
        .claims
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    anyhow::ensure!(
        before_cycles > 0 && after_cycles > 0,
        "INCONCLUSIVE: pre/post eligibility activity missing"
    );
    anyhow::ensure!(
        activity.0.load(Ordering::SeqCst) == activity.1.load(Ordering::SeqCst)
            && activity.2.load(Ordering::SeqCst) > 0
            && activity.2.load(Ordering::SeqCst) == claims.len(),
        "INCONCLUSIVE: missing claim or incomplete real cycle"
    );
    let mut expected_attempt = claimed.attempts + 1;
    for (event_id, claim) in claims.iter() {
        let claim = claim
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("122 new claim observation missing"))?;
        // Persisted claim time, not cycle start: a real selection may straddle E.
        anyhow::ensure!(
            *event_id == before.event_id
                && claim.id == before.id
                && claim.aggregate_id == id
                && claim.event_id == before.event_id
                && claim.payload_json == before.payload_json
                && claim.status == outbox::STATUS_PUBLISHING
                && claim.attempts == expected_attempt
                && claim.locked_by.as_deref() == Some(new_worker.as_str())
                && claim.updated_at >= eligibility
                && claim
                    .locked_until
                    .is_some_and(|until| until > claim.updated_at),
            "122 new owner/timing/attempt/identity claim invariant failed"
        );
        expected_attempt += 1;
    }
    anyhow::ensure!(
        published.attempts == expected_attempt - 1
            && published.locked_by.is_none()
            && published.locked_until.is_none(),
        "122 published attempts/lock invariant failed"
    );
    claim117_trace(
        "122_component_post_expiry_proved_not_global_acceptance",
        trace_started,
        serde_json::json!({
            "outbox_id": published.id, "event_id": published.event_id, "aggregate_id": published.aggregate_id,
        "eligibility_utc": eligibility, "deadline_utc": deadline_utc, "phase_elapsed_ms": t0.elapsed().as_millis(),
        "observation_completed_elapsed_us": completed_at.duration_since(t0).as_micros(),
        "observation_deadline_elapsed_us": deadline.duration_since(t0).as_micros(),
            "before_cycles": before_cycles, "after_cycles": after_cycles, "messages": messages,
            "cycles_started": activity.0.load(Ordering::SeqCst), "cycles_completed": activity.1.load(Ordering::SeqCst),
            "claims": activity.2.load(Ordering::SeqCst), "real_publish_successes": publisher.successes.load(Ordering::SeqCst),
            "old_parent_destroyed": old_destroyed.load(Ordering::SeqCst),
            "old_publisher_destroyed": suspended.destroyed.load(Ordering::SeqCst),
            "new_parent_destroyed": new_destroyed.load(Ordering::SeqCst),
        }),
    );
    Ok(())
}

struct PollerOwner(Arc<AtomicUsize>);
impl Drop for PollerOwner {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
#[async_trait]
impl EventHandler for SqsHandler {
    async fn handle_event(&self, _: Box<dyn DomainEvent>) -> Result<(), ServiceError> {
        self.entered.notify_one();
        std::future::pending().await
    }
    fn supports_event_type(&self, _: &str) -> bool {
        true
    }
}

struct SqsHandler {
    _owner: PollerOwner,
    entered: Arc<tokio::sync::Notify>,
}

#[tokio::test]
#[serial]
async fn real_sqs_long_poll_shutdown_joins_parent_and_destroys_all_poller_resources() {
    let fixture = TelegraphTestFixture::new(Arc::new(TelegraphTestDescriptor))
        .await
        .unwrap();
    let QueueConfig::Sqs(mut sqs_config) = config().queue else {
        panic!("SQS opt-in");
    };
    // Two pollers exercise the same RunResources owner; its handler cannot be
    // destroyed until every poller and the parent released the encompassing Arc.
    sqs_config.queues.clear();
    sqs_config.default_queues = vec!["telegraph-events".into(), "test-telegraph-outbound".into()];
    let consumer = Arc::new(
        rustycog::events::SqsEventConsumer::new(sqs_config)
            .await
            .unwrap(),
    );
    let destroyed = Arc::new(AtomicUsize::new(0));
    let parent_destroyed = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new(tokio::sync::Notify::new());
    let owner = SqsHandler {
        _owner: PollerOwner(destroyed.clone()),
        entered: entered.clone(),
    };
    let parent_owner = PollerOwner(parent_destroyed.clone());
    fixture
        .sqs()
        .client
        .set_queue_attributes()
        .queue_url(&fixture.sqs().queue_url)
        .attributes(
            aws_sdk_sqs::types::QueueAttributeName::VisibilityTimeout,
            "1",
        )
        .send()
        .await
        .unwrap();
    let id = uuid::Uuid::new_v4();
    fixture
        .sqs()
        .send_event(&iam_events::IamDomainEvent::UserEmailVerified(
            iam_events::UserEmailVerifiedEvent {
                base: rustycog::events::event::BaseEvent::new("user_email_verified".into(), id),
                user_id: id,
                email: "blocked@example.test".into(),
            },
        ))
        .await
        .unwrap();
    let task_consumer = consumer.clone();
    let mut tasks = vec![tokio::spawn(async move {
        let _parent_owner = parent_owner;
        task_consumer.start(owner).await.map_err(Into::into)
    })];
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    // One poller is inside an async handler, the other is in an empty-queue
    // long-poll. The SDK owner encompasses both, not just the parent handle.
    assert_eq!(destroyed.load(Ordering::SeqCst), 0);
    let start = tokio::time::Instant::now();
    shutdown_owned_tasks(&mut tasks, async {
        consumer.stop().await.map_err(Into::into)
    })
    .await
    .unwrap();
    assert!(start.elapsed() < Duration::from_secs(6));
    assert_eq!(
        destroyed.load(Ordering::SeqCst),
        1,
        "all pollers released resources"
    );
    assert_eq!(parent_destroyed.load(Ordering::SeqCst), 1);
    assert!(tasks.is_empty());
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        destroyed.load(Ordering::SeqCst),
        1,
        "no activity after shutdown"
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(
        fixture.sqs().receive_messages(10, 1).await.unwrap().len(),
        1,
        "cancelled handler must not acknowledge the message"
    );
    drop(fixture);
}
