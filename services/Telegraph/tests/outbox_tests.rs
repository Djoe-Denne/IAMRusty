//! SQL-only AC1–AC3 component proofs. Real transport/disabled/runtime and the
//! Sentinel authorization path are deliberately completed by T3, not claimed here.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{SubsecRound, Utc};
use rustycog::config::ServerConfig;
use rustycog::events::{DomainEvent, EventPublisher};
use rustycog::outbox::{
    entity::{
        self as outbox, OutboxEvents, STATUS_FAILED, STATUS_PENDING, STATUS_PUBLISHED,
        STATUS_PUBLISHING,
    },
    OutboxConfig, OutboxDispatcher, OutboxRecorder,
};
use rustycog::testing::{ServiceTestDescriptor, TestFixture};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityName, EntityTrait,
    PaginatorTrait, QueryFilter, Set,
};
use serial_test::serial;
use telegraph_configuration::TelegraphConfig;
use telegraph_domain::{
    entity::communication::{CommunicationMode, CommunicationRecipient, NotificationCommunication},
    service::NotificationServiceImpl,
    DomainError, NotificationService,
};
use telegraph_events::TelegraphDomainEvent;
use telegraph_infra::repository::{
    entity::{notification_deliveries, notifications},
    CombinedNotificationRepositoryImpl, NotificationReadRepositoryImpl,
    NotificationWriteRepositoryImpl,
};
use telegraphmigration::{Migrator, MigratorTrait};
use uuid::Uuid;

struct SqlOutboxDescriptor {
    legacy: bool,
}

#[async_trait]
impl ServiceTestDescriptor<TestFixture> for SqlOutboxDescriptor {
    type Config = TelegraphConfig;

    async fn build_app(&self, _: TelegraphConfig, _: ServerConfig) -> anyhow::Result<()> {
        Ok(())
    }
    async fn run_app(&self, _: TelegraphConfig, _: ServerConfig) -> anyhow::Result<()> {
        anyhow::bail!("SQL-only descriptor must never start an application")
    }
    async fn run_migrations_up(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
        Migrator::up(db, self.legacy.then_some(1)).await?;
        Ok(())
    }
    async fn run_migrations_down(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
        Migrator::down(db, None).await?;
        Ok(())
    }
    fn has_db(&self) -> bool {
        true
    }
    fn has_sqs(&self) -> bool {
        false
    }
    fn has_openfga(&self) -> bool {
        false
    }
}

async fn fixture(legacy: bool) -> TestFixture {
    TestFixture::new(Arc::new(SqlOutboxDescriptor { legacy }))
        .await
        .expect("real PostgreSQL fixture")
}

fn service(
    db: Arc<DatabaseConnection>,
) -> NotificationServiceImpl<CombinedNotificationRepositoryImpl> {
    NotificationServiceImpl::new(Arc::new(CombinedNotificationRepositoryImpl::new(
        Arc::new(NotificationReadRepositoryImpl::new(db.clone())),
        Arc::new(NotificationWriteRepositoryImpl::new(
            db,
            OutboxRecorder::new(),
        )),
    )))
}

fn notification() -> NotificationCommunication {
    NotificationCommunication {
        recipient: CommunicationRecipient {
            user_id: Some(Uuid::new_v4()),
            email: None,
        },
        id: Some(Uuid::new_v4()),
        title: "outbox atomicity".into(),
        body: "durable body".into(),
        data: HashMap::new(),
        is_read: None,
        created_at: Some(
            chrono::DateTime::from_timestamp(1_700_000_000, 123_456_789).expect("test timestamp"),
        ),
        updated_at: None,
        read_at: None,
    }
}

async fn create(
    service: &impl NotificationService,
    input: NotificationCommunication,
    delivery: bool,
) -> Result<NotificationCommunication, DomainError> {
    if delivery {
        service
            .create_notification_with_delivery(input, CommunicationMode::Notification)
            .await
            .map(|(notification, delivery)| {
                assert_eq!(Some(delivery.message_id), notification.id);
                notification
            })
    } else {
        service.create_notification(input).await
    }
}

async fn counts(db: &DatabaseConnection, id: Uuid) -> (u64, u64, u64) {
    (
        notifications::Entity::find_by_id(id)
            .count(db)
            .await
            .expect("notification count"),
        notification_deliveries::Entity::find()
            .filter(notification_deliveries::Column::NotificationId.eq(id))
            .count(db)
            .await
            .expect("delivery count"),
        OutboxEvents::find()
            .filter(outbox::Column::AggregateId.eq(id))
            .count(db)
            .await
            .expect("outbox count"),
    )
}

async fn outbox_row(db: &DatabaseConnection, id: Uuid) -> outbox::Model {
    OutboxEvents::find()
        .filter(outbox::Column::AggregateId.eq(id))
        .one(db)
        .await
        .expect("outbox query")
        .expect("durable intention")
}

fn assert_payload(row: &outbox::Model, created: &NotificationCommunication) {
    let TelegraphDomainEvent::NotificationCreated(event) =
        serde_json::from_value(row.payload_json.clone()).expect("unchanged Telegraph event schema");
    assert_eq!(row.status, STATUS_PENDING);
    assert_eq!(row.attempts, 0);
    assert_ne!(
        row.id, row.event_id,
        "SQL identity is not the event identity"
    );
    assert_eq!(row.event_id, event.base.event_id);
    assert_eq!(row.aggregate_id, event.base.aggregate_id);
    assert_eq!(Some(event.notification_id), created.id);
    assert_eq!(row.aggregate_id, event.notification_id);
    assert_eq!(Some(event.user_id), created.recipient.user_id);
    assert_eq!(Some(event.created_at), created.created_at);
    assert_eq!(row.event_type, "notification_created");
    assert_eq!(event.base.event_type, row.event_type);
    assert_eq!(row.occurred_at, event.base.occurred_at.trunc_subsecs(6));
    assert_eq!(
        row.version,
        i32::try_from(event.base.version).expect("event version")
    );
}

fn assert_infrastructure(error: &DomainError) {
    assert!(
        matches!(error, DomainError::InfrastructureError(_)),
        "expected infrastructure category: {error}"
    );
    assert!(error.is_recoverable());
    assert!(matches!(
        rustycog::core::error::ServiceError::from(error),
        rustycog::core::error::ServiceError::Infrastructure { .. }
    ));
}

#[tokio::test]
#[serial]
async fn both_creation_paths_commit_exact_business_and_outbox_deltas() {
    let fixture = fixture(false).await;
    let db = fixture.db();
    let service = service(db.clone());
    for delivery in [false, true] {
        let input = notification();
        let id = input.id.expect("supplied id");
        let user_id = input.recipient.user_id;
        let timestamp = input.created_at.expect("supplied time").trunc_subsecs(6);
        assert_eq!(counts(&db, id).await, (0, 0, 0));
        let created = create(&service, input, delivery)
            .await
            .expect("creation independent of publisher");
        assert_eq!(created.id, Some(id));
        assert_eq!(created.recipient.user_id, user_id);
        assert_eq!(created.created_at, Some(timestamp));
        assert_eq!(created.updated_at, Some(timestamp));
        assert_eq!(created.is_read, Some(false));
        assert_eq!(counts(&db, id).await, (1, u64::from(delivery), 1));
        let stored = notifications::Entity::find_by_id(id)
            .one(db.as_ref())
            .await
            .expect("writer query")
            .expect("stored notification");
        assert_eq!(Some(stored.created_at), created.created_at);
        assert_payload(&outbox_row(&db, id).await, &created);
    }
}

#[tokio::test]
#[serial]
async fn generated_ids_and_timestamps_are_shared_with_the_durable_payload() {
    let fixture = fixture(false).await;
    let db = fixture.db();
    let service = service(db.clone());
    for delivery in [false, true] {
        let mut input = notification();
        input.id = None;
        input.created_at = None;
        let user_id = input.recipient.user_id.expect("user");
        assert_eq!(
            notifications::Entity::find()
                .filter(notifications::Column::UserId.eq(user_id))
                .count(db.as_ref())
                .await
                .expect("before"),
            0
        );
        let created = create(&service, input, delivery)
            .await
            .expect("generated defaults");
        let id = created.id.expect("generated id");
        assert!(!id.is_nil());
        assert_eq!(counts(&db, id).await, (1, u64::from(delivery), 1));
        assert_payload(&outbox_row(&db, id).await, &created);
        assert_eq!(
            notifications::Entity::find()
                .filter(notifications::Column::UserId.eq(user_id))
                .count(db.as_ref())
                .await
                .expect("after"),
            1
        );
    }
}

#[tokio::test]
#[serial]
async fn real_delivery_and_outbox_sql_faults_rollback_the_entire_attempt_then_recover() {
    let fixture = fixture(false).await;
    let db = fixture.db();
    let service = service(db.clone());
    // Delivery failure is only applicable to the path with a delivery. Outbox
    // failure must roll back both paths, including the already-inserted delivery.
    for (delivery, table, column) in [
        (
            true,
            notification_deliveries::Entity.table_name(),
            "notification_id",
        ),
        (false, outbox::Entity.table_name(), "aggregate_id"),
        (true, outbox::Entity.table_name(), "aggregate_id"),
    ] {
        let input = notification();
        let id = input.id.expect("attempt id");
        assert_eq!(counts(&db, id).await, (0, 0, 0));
        db.execute_unprepared(&format!(
            "ALTER TABLE {table} ADD CONSTRAINT reject_ac1_attempt CHECK ({column} <> '{id}')"
        ))
        .await
        .expect("install real SQL fault");
        let result = create(&service, input.clone(), delivery).await;
        db.execute_unprepared(&format!(
            "ALTER TABLE {table} DROP CONSTRAINT reject_ac1_attempt"
        ))
        .await
        .expect("remove fault");
        let error = result.expect_err("SQL fault must fail creation");
        assert_infrastructure(&error);
        assert_eq!(
            counts(&db, id).await,
            (0, 0, 0),
            "notification/delivery/outbox all rolled back"
        );
        let created = create(&service, input, delivery)
            .await
            .expect("same attempt succeeds after removing fault");
        assert_eq!(counts(&db, id).await, (1, u64::from(delivery), 1));
        assert_payload(&outbox_row(&db, id).await, &created);
    }
}

#[tokio::test]
#[serial]
async fn missing_recipient_rejects_both_paths_before_writing() {
    let fixture = fixture(false).await;
    let db = fixture.db();
    let service = service(db.clone());
    for delivery in [false, true] {
        let mut input = notification();
        let id = input.id.expect("attempt id");
        input.recipient.user_id = None;
        assert_eq!(counts(&db, id).await, (0, 0, 0));
        let error = create(&service, input, delivery)
            .await
            .expect_err("missing user rejected");
        assert!(matches!(error, DomainError::InvalidRecipient(_)));
        assert!(error.is_validation_error());
        assert!(!error.is_recoverable());
        assert_eq!(counts(&db, id).await, (0, 0, 0));
    }
}

#[tokio::test]
#[serial]
async fn additive_migration_preserves_old_rows_is_repeatable_and_down_only_removes_outbox() {
    let fixture = fixture(true).await;
    let db = fixture.db();
    let old = notification();
    let old_id = old.id.expect("legacy id");
    let timestamp = old.created_at.expect("legacy timestamp").trunc_subsecs(6);
    let legacy_row = notifications::Entity::insert(notifications::ActiveModel {
        id: Set(old_id),
        user_id: Set(old.recipient.user_id.expect("legacy user")),
        title: Set(old.title),
        content: Set(old.body.into_bytes()),
        content_type: Set("application/text".into()),
        is_read: Set(false),
        priority: Set(1),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        read_at: Set(None),
        ..Default::default()
    })
    .exec_with_returning(db.as_ref())
    .await
    .expect("old-schema seed");
    // A missing outbox table is a persistence failure, never publish fallback.
    let input = notification();
    let id = input.id.expect("attempt id");
    let error = create(&service(db.clone()), input.clone(), true)
        .await
        .expect_err("schema must precede binary");
    assert_infrastructure(&error);
    assert_eq!(
        notifications::Entity::find_by_id(id)
            .count(db.as_ref())
            .await
            .expect("rollback"),
        0
    );
    assert_eq!(
        notification_deliveries::Entity::find()
            .filter(notification_deliveries::Column::NotificationId.eq(id))
            .count(db.as_ref())
            .await
            .expect("delivery rollback"),
        0
    );
    Migrator::up(db.as_ref(), None)
        .await
        .expect("upgrade existing database");
    Migrator::up(db.as_ref(), None)
        .await
        .expect("idempotent migration runner");
    assert_eq!(
        notifications::Entity::find_by_id(old_id)
            .one(db.as_ref())
            .await
            .expect("legacy read"),
        Some(legacy_row.clone())
    );
    assert_eq!(
        counts(&db, old_id).await,
        (1, 0, 0),
        "no historical backfill"
    );
    assert_eq!(counts(&db, id).await, (0, 0, 0));
    create(&service(db.clone()), input, true)
        .await
        .expect("upgraded writer");
    assert_eq!(counts(&db, id).await, (1, 1, 1));
    // Owned test DB only: down is NOT a delivery/rollback procedure.
    Migrator::down(db.as_ref(), Some(1))
        .await
        .expect("down only additive migration");
    assert_eq!(
        notifications::Entity::find_by_id(old_id)
            .one(db.as_ref())
            .await
            .expect("old row preserved"),
        Some(legacy_row)
    );
    assert_eq!(
        notifications::Entity::find_by_id(id)
            .count(db.as_ref())
            .await
            .expect("new row preserved"),
        1
    );
    assert!(
        OutboxEvents::find().all(db.as_ref()).await.is_err(),
        "only outbox removed"
    );
    Migrator::up(db.as_ref(), None)
        .await
        .expect("restore additive migration");
    assert_eq!(counts(&db, id).await, (1, 1, 0));
}

struct RecordingPublisher {
    fail: bool,
    events: Mutex<Vec<(Uuid, serde_json::Value)>>,
}

impl RecordingPublisher {
    fn new(fail: bool) -> Arc<Self> {
        Arc::new(Self {
            fail,
            events: Mutex::new(Vec::new()),
        })
    }
}

#[async_trait]
impl EventPublisher<DomainError> for RecordingPublisher {
    async fn publish(&self, event: &dyn DomainEvent) -> Result<(), DomainError> {
        let payload = serde_json::from_str(&event.to_json().expect("stored event serializes"))
            .expect("stored JSON");
        self.events
            .lock()
            .expect("event witness")
            .push((event.event_id(), payload));
        if self.fail {
            Err(DomainError::infrastructure_error("forced publisher outage"))
        } else {
            Ok(())
        }
    }
    async fn publish_batch(&self, events: &[Box<dyn DomainEvent>]) -> Result<(), DomainError> {
        for event in events {
            self.publish(event.as_ref()).await?;
        }
        Ok(())
    }
    async fn health_check(&self) -> Result<(), DomainError> {
        Ok(())
    }
}

fn dispatcher_config() -> OutboxConfig {
    OutboxConfig {
        worker_id: Uuid::new_v4().to_string(),
        max_attempts: 2,
        lock_timeout: Duration::from_secs(2),
        retry_base_delay: Duration::from_secs(1),
        ..OutboxConfig::default()
    }
}

async fn make_due(db: &DatabaseConnection, row: &outbox::Model) {
    outbox::ActiveModel {
        id: Set(row.id),
        next_attempt_at: Set(Utc::now() - chrono::Duration::seconds(1)),
        ..Default::default()
    }
    .update(db)
    .await
    .expect("make stored retry due");
}

#[tokio::test]
#[serial]
async fn failed_publish_is_durable_and_new_dispatcher_reuses_identity_without_new_business_rows() {
    let fixture = fixture(false).await;
    let db = fixture.db();
    let pool = fixture.database.as_ref().expect("database").pool.clone();
    let input = notification();
    let id = input.id.expect("id");
    create(&service(db.clone()), input, true)
        .await
        .expect("commit with dispatcher stopped");
    let original = outbox_row(&db, id).await;
    let failure = RecordingPublisher::new(true);
    let first = OutboxDispatcher::new(pool.clone(), failure.clone(), dispatcher_config());
    assert_eq!(
        first
            .dispatch_once()
            .await
            .expect("failed publish is recorded in SQL"),
        1
    );
    let failed = outbox_row(&db, id).await;
    assert_eq!(failed.status, STATUS_FAILED);
    assert_eq!(failed.attempts, 1);
    assert!(failed
        .last_error
        .as_ref()
        .expect("durable error")
        .contains("forced publisher outage"));
    assert!(failed.next_attempt_at > failed.updated_at);
    assert_eq!(failed.payload_json, original.payload_json);
    assert_eq!(counts(&db, id).await, (1, 1, 1));
    first.stop().await.expect("stop first instance");
    drop(first);
    make_due(&db, &failed).await;
    let success = RecordingPublisher::new(false);
    let second = OutboxDispatcher::new(pool, success.clone(), dispatcher_config());
    assert_eq!(
        second
            .dispatch_once()
            .await
            .expect("new instance retries same DB"),
        1
    );
    let published = outbox_row(&db, id).await;
    assert_eq!(published.id, original.id);
    assert_eq!(published.event_id, original.event_id);
    assert_eq!(published.payload_json, original.payload_json);
    assert_eq!(published.status, STATUS_PUBLISHED);
    assert_eq!(published.attempts, 2);
    assert_eq!(
        *success.events.lock().expect("sent"),
        vec![(original.event_id, original.payload_json.clone())]
    );
    assert_eq!(
        *failure.events.lock().expect("attempted"),
        *success.events.lock().expect("retried")
    );
    assert_eq!(counts(&db, id).await, (1, 1, 1));
}

struct PendingPublisher(Arc<tokio::sync::Notify>);

#[async_trait]
impl EventPublisher<DomainError> for PendingPublisher {
    async fn publish(&self, _: &dyn DomainEvent) -> Result<(), DomainError> {
        self.0.notify_one();
        std::future::pending().await
    }
    async fn publish_batch(&self, _: &[Box<dyn DomainEvent>]) -> Result<(), DomainError> {
        unreachable!("dispatcher publishes individually")
    }
    async fn health_check(&self) -> Result<(), DomainError> {
        Ok(())
    }
}

#[tokio::test]
#[serial]
async fn cancelled_publish_claim_is_recovered_only_after_expiry_by_new_instance() {
    let fixture = fixture(false).await;
    let db = fixture.db();
    let pool = fixture.database.as_ref().expect("database").pool.clone();
    let input = notification();
    let id = input.id.expect("id");
    create(&service(db.clone()), input, false)
        .await
        .expect("committed creation");
    let original = outbox_row(&db, id).await;
    let entered = Arc::new(tokio::sync::Notify::new());
    let first = OutboxDispatcher::new(
        pool.clone(),
        Arc::new(PendingPublisher(entered.clone())),
        dispatcher_config(),
    );
    let mut in_flight = Box::pin(first.dispatch_once());
    tokio::time::timeout(Duration::from_secs(2), async {
        tokio::select! { () = entered.notified() => {}, result = &mut in_flight => panic!("publish must remain pending: {result:?}") }
    }).await.expect("claim reached publisher");
    drop(in_flight);
    first.stop().await.expect("stop interrupted instance");
    drop(first);
    let claimed = outbox_row(&db, id).await;
    assert_eq!(claimed.status, STATUS_PUBLISHING);
    assert_eq!(claimed.attempts, 1);
    let locked_until = claimed.locked_until.expect("claim expiry");
    let publisher = RecordingPublisher::new(false);
    let second = OutboxDispatcher::new(pool, publisher.clone(), dispatcher_config());
    assert_eq!(
        second
            .dispatch_once()
            .await
            .expect("unexpired claim is not selectable"),
        0
    );
    assert!(publisher.events.lock().expect("not sent").is_empty());
    tokio::time::timeout(Duration::from_secs(3), async {
        while Utc::now() < locked_until {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("bounded lock expiry");
    assert_eq!(
        second
            .dispatch_once()
            .await
            .expect("expired claim recovered"),
        1
    );
    let published = outbox_row(&db, id).await;
    assert_eq!(published.status, STATUS_PUBLISHED);
    assert_eq!(published.event_id, original.event_id);
    assert_eq!(published.payload_json, original.payload_json);
    assert_eq!(
        *publisher.events.lock().expect("same event"),
        vec![(original.event_id, original.payload_json)]
    );
    assert_eq!(counts(&db, id).await, (1, 0, 1));
}

#[tokio::test]
#[serial]
async fn exhausted_publish_budget_remains_visible_without_unlimited_retry() {
    let fixture = fixture(false).await;
    let db = fixture.db();
    let input = notification();
    let id = input.id.expect("id");
    create(&service(db.clone()), input, false)
        .await
        .expect("committed creation");
    let publisher = RecordingPublisher::new(true);
    let dispatcher = OutboxDispatcher::new(
        fixture.database.as_ref().expect("database").pool.clone(),
        publisher.clone(),
        dispatcher_config(),
    );
    for attempt in 1..=2 {
        make_due(&db, &outbox_row(&db, id).await).await;
        assert_eq!(dispatcher.dispatch_once().await.expect("attempt"), 1);
        let failed = outbox_row(&db, id).await;
        assert_eq!(failed.status, STATUS_FAILED);
        assert_eq!(failed.attempts, attempt);
        assert!(failed.last_error.is_some());
    }
    let exhausted = outbox_row(&db, id).await;
    make_due(&db, &exhausted).await;
    assert_eq!(dispatcher.dispatch_once().await.expect("finite budget"), 0);
    assert_eq!(outbox_row(&db, id).await.attempts, 2);
    assert_eq!(publisher.events.lock().expect("attempt count").len(), 2);
    assert_eq!(counts(&db, id).await, (1, 0, 1));
}
