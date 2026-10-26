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
