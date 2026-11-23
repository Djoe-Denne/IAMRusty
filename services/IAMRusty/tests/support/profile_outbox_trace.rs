//! Verify the persisted profile event's logging boundary at the actual IAM root.

use std::{
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};

use iam_domain::{
    entity::events::{ProfileEmailVerificationRequested, PROFILE_EMAIL_VERIFICATION_REQUESTED},
    service::IamOutboxUnitOfWork,
};
use iam_infra::transaction::IamOutboxUnitOfWorkImpl;
use rustycog::{
    events::DomainEvent,
    outbox::{
        entity::{Column, OutboxEvents, STATUS_PENDING, STATUS_PUBLISHED},
        OutboxRecorder,
    },
    testing::MockEventPublisher,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, QueryFilter,
    Statement,
};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

use crate::common::{self, IAMRustyTestDescriptor, TestFixture};

const CAPTURE_LIMIT: usize = 16 * 1024;

#[derive(Clone, Default)]
struct PrivateCapture(Arc<Mutex<Vec<u8>>>);

impl Write for PrivateCapture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let mut captured = self.0.lock().expect("capture lock");
        let remaining = CAPTURE_LIMIT - captured.len();
        captured.extend_from_slice(&bytes[..remaining.min(bytes.len())]);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

async fn wait_until_published(
    db: &DatabaseConnection,
    event_id: Uuid,
) -> Result<(), sea_orm::DbErr> {
    loop {
        let exists = OutboxEvents::find()
            .filter(Column::EventId.eq(event_id))
            .filter(Column::Status.eq(STATUS_PUBLISHED))
            .one(db)
            .await?
            .is_some();
        if exists {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[test]
#[serial]
fn persisted_profile_link_is_private_at_the_normal_publisher() {
    // Keep spawned IAM workers on this thread so the scoped dispatcher reaches
    // them. Neither the fixture setup nor the rest of the binary is captured.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("current-thread fixture runtime");
    let (fixture, app, transport) = runtime.block_on(async {
        let fixture = TestFixture::new(Arc::new(IAMRustyTestDescriptor))
            .await
            .unwrap_or_else(|_| panic!("start the existing IAM/Postgres fixture"));
        let config = common::fixture_config(&fixture)
            .unwrap_or_else(|_| panic!("load the isolated IAM fixture configuration"));
        let transport = Arc::new(MockEventPublisher::new());
        let app = Box::pin(iam_setup::app::build_app_state_with_event_publisher(
            config,
            common::mock_publisher(transport.clone()),
            readiness::ComponentStatus::Injected,
            None,
        ))
        .await
        .unwrap_or_else(|_| panic!("build actual IAM root with the normal publisher adapter"));
        (fixture, app, transport)
    });

    let user = Uuid::new_v4();
    let fragment = format!("inert-profile-trace-{}", Uuid::new_v4().simple());
    let link = format!("https://profile-trace.example.invalid/confirm#{fragment}");
    let metadata_marker = format!("inert-metadata-{}", Uuid::new_v4().simple());
    let metadata = json!({"private_trace_fixture": metadata_marker});
    let event = ProfileEmailVerificationRequested::new(
        user,
        "profile-trace@example.invalid".to_owned(),
        link.clone(),
    );
    let event_id = event.event_id();
    let db = fixture.db();
    let expected_payload = runtime.block_on(async {
        let pool = fixture
            .database
            .as_ref()
            .expect("fixture database")
            .pool
            .clone();
        IamOutboxUnitOfWorkImpl::new(pool, OutboxRecorder)
            .record_event(Box::new(event))
            .await
            .unwrap_or_else(|_| panic!("record dedicated event through the real outbox recorder"));
        let row = OutboxEvents::find()
            .filter(Column::EventId.eq(event_id))
            .one(db.as_ref())
            .await
            .unwrap_or_else(|_| panic!("read recorded event"))
            .expect("recorded event exists before the dispatcher starts");
        assert_eq!(row.status, STATUS_PENDING);
        assert!(row.payload_json["verification_url"].as_str() == Some(link.as_str()));
        // The current event supplies empty metadata. Seed a private marker in
        // the persisted row to exercise forwarding and logging of nonempty metadata.
        let changed = db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE rustycog_outbox_events SET metadata_json = $1 WHERE event_id = $2",
                [metadata.clone().into(), event_id.into()],
            ))
            .await
            .unwrap_or_else(|_| panic!("seed private metadata on the recorded fixture row"));
        assert_eq!(changed.rows_affected(), 1);
        row.payload_json
    });

    let capture = PrivateCapture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            "off,rustycog_framework::events::adapter=debug,rustycog_framework::outbox::dispatcher=debug",
        ))
        .with_writer(move || writer.clone())
        .finish();
    let dispatch = tracing::Dispatch::new(subscriber);
    let (published, shutdown) = tracing::dispatcher::with_default(&dispatch, || {
        runtime.block_on(async {
            let mut workers = app.start_background_tasks();
            let published = tokio::time::timeout(
                Duration::from_secs(8),
                wait_until_published(db.as_ref(), event_id),
            )
            .await;
            let shutdown = app.shutdown_background_tasks(&mut workers).await;
            (matches!(published, Ok(Ok(()))), shutdown.is_ok())
        })
    });
    assert!(
        shutdown,
        "owned IAM dispatcher and cleanup worker must join"
    );
    assert!(published, "the persisted event must be marked published");

    let received = transport.get_published_events();
    assert_eq!(
        received.len(),
        1,
        "normal adapter must deliver exactly one event"
    );
    let delivered = &received[0];
    assert_eq!(delivered.event_id, event_id);
    assert_eq!(delivered.aggregate_id, user);
    assert_eq!(delivered.event_type, PROFILE_EMAIL_VERIFICATION_REQUESTED);
    assert_eq!(delivered.version, 1);
    assert!(
        delivered.parse_json().ok().as_ref() == Some(&expected_payload),
        "transport must receive the original payload, including the private URL"
    );
    assert!(
        delivered.metadata.len() == 1
            && delivered.metadata.get("private_trace_fixture") == Some(&metadata_marker),
        "transport must receive the original private metadata"
    );

    let bytes = capture.0.lock().expect("capture lock");
    assert!(
        bytes.len() < CAPTURE_LIMIT,
        "trace capture must not truncate"
    );
    let logs = std::str::from_utf8(&bytes).expect("trace capture UTF-8");
    let event_id = event_id.to_string();
    assert!(logs
        .lines()
        .any(|line| line.contains("INFO") && line.contains("Publishing event:")));
    assert!(logs.lines().any(|line| line.contains("DEBUG")
        && line.contains("Outbox event published")
        && line.contains(&event_id)));
    assert!(
        !logs.contains(&link) && !logs.contains(&fragment) && !logs.contains(&metadata_marker),
        "persisted profile capability or private metadata entered publisher traces"
    );
}
