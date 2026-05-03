mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use common::{IAMRustyTestDescriptor, TestFixture};
use iam_domain::{
    entity::events::{DomainEvent as IamDomainEvent, PasswordResetRequestedEvent},
    error::DomainError,
    service::IamOutboxUnitOfWork,
};
use iam_infra::transaction::IamOutboxUnitOfWorkImpl;
use rustycog_core::error::ServiceError;
use rustycog_events::event::{DomainEvent, EventPublisher};
use rustycog_outbox::{
    entity::{
        Column as OutboxColumn, OutboxEvents, STATUS_FAILED, STATUS_PENDING, STATUS_PUBLISHED,
    },
    OutboxConfig, OutboxDispatcher, OutboxRecorder,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serial_test::serial;
use uuid::Uuid;

struct TestPublisher {
    should_fail: bool,
    published_event_ids: Mutex<Vec<Uuid>>,
}

impl TestPublisher {
    const fn success() -> Self {
        Self {
            should_fail: false,
            published_event_ids: Mutex::new(Vec::new()),
        }
    }

    const fn failure() -> Self {
        Self {
            should_fail: true,
            published_event_ids: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl EventPublisher<DomainError> for TestPublisher {
    async fn publish(&self, event: &dyn DomainEvent) -> Result<(), DomainError> {
        if self.should_fail {
            return Err(DomainError::EventError(
                "forced publish failure".to_string(),
            ));
        }

        self.published_event_ids
            .lock()
            .expect("published event lock poisoned")
            .push(event.event_id());
        Ok(())
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

#[derive(Debug)]
struct BadEvent {
    event_id: Uuid,
}

impl DomainEvent for BadEvent {
    fn event_type(&self) -> &'static str {
        "bad_iam_test_event"
    }

    fn event_id(&self) -> Uuid {
        self.event_id
    }

    fn aggregate_id(&self) -> Uuid {
        self.event_id
    }

    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }

    fn version(&self) -> u32 {
        1
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        Err(ServiceError::internal("forced serialization failure"))
    }

    fn metadata(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

fn password_reset_requested_event() -> IamDomainEvent {
    let user_id = Uuid::new_v4();
    IamDomainEvent::PasswordResetRequested(PasswordResetRequestedEvent::new(
        user_id,
        format!("outbox-{user_id}@example.com"),
        "raw-reset-token".to_string(),
        chrono::Utc::now() + chrono::Duration::hours(1),
    ))
}

#[tokio::test]
#[serial]
async fn iam_outbox_uow_and_dispatcher_regressions() {
    let descriptor = Arc::new(IAMRustyTestDescriptor);
    let fixture = TestFixture::new(descriptor)
        .await
        .expect("failed to create test fixture");
    let db = fixture.db();
    let db_pool = fixture.database.as_ref().unwrap().pool.clone();
    let uow = IamOutboxUnitOfWorkImpl::new(db_pool.clone(), OutboxRecorder::new());

    let pending_event = password_reset_requested_event();
    let pending_event_id = pending_event.event_id();
    let pending_aggregate_id = pending_event.aggregate_id();
    uow.record_event(pending_event.into())
        .await
        .expect("outbox recording should succeed");

    let pending_row = OutboxEvents::find()
        .filter(OutboxColumn::EventId.eq(pending_event_id))
        .one(db.as_ref())
        .await
        .expect("outbox lookup should succeed")
        .expect("outbox row should exist");
    assert_eq!(pending_row.status, STATUS_PENDING);
    assert_eq!(pending_row.aggregate_id, pending_aggregate_id);
    assert_eq!(pending_row.event_type, "password_reset_requested");

    let bad_event_id = Uuid::new_v4();
    let error = uow
        .record_event(Box::new(BadEvent {
            event_id: bad_event_id,
        }))
        .await
        .expect_err("bad event should fail outbox recording");
    assert!(
        error.to_string().contains("forced serialization failure"),
        "unexpected error: {error}"
    );
    let failed_recording_row = OutboxEvents::find()
        .filter(OutboxColumn::EventId.eq(bad_event_id))
        .one(db.as_ref())
        .await
        .expect("outbox lookup should succeed");
    assert!(
        failed_recording_row.is_none(),
        "failed recording should roll back"
    );

    OutboxDispatcher::new(
        db_pool.clone(),
        Arc::new(TestPublisher::success()),
        OutboxConfig::default(),
    )
    .dispatch_once()
    .await
    .expect("dispatch should succeed");
    let published_row = OutboxEvents::find()
        .filter(OutboxColumn::EventId.eq(pending_event_id))
        .one(db.as_ref())
        .await
        .expect("outbox lookup should succeed")
        .expect("outbox row should exist");
    assert_eq!(published_row.status, STATUS_PUBLISHED);

    let retry_event = password_reset_requested_event();
    let retry_event_id = retry_event.event_id();
    uow.record_event(retry_event.into())
        .await
        .expect("outbox recording should succeed");

    OutboxDispatcher::new(
        db_pool,
        Arc::new(TestPublisher::failure()),
        OutboxConfig::default(),
    )
    .dispatch_once()
    .await
    .expect("dispatch failure should be recorded, not returned");
    let retry_row = OutboxEvents::find()
        .filter(OutboxColumn::EventId.eq(retry_event_id))
        .one(db.as_ref())
        .await
        .expect("outbox lookup should succeed")
        .expect("outbox row should exist");
    assert_eq!(retry_row.status, STATUS_FAILED);
    assert_eq!(retry_row.attempts, 1);
    assert!(retry_row.last_error.is_some());
}
