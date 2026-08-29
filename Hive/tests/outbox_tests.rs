mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use common::{HiveTestDescriptor, HiveTestFixture};
use hive_application::HiveOutboxUnitOfWork;
use hive_domain::Organization;
use hive_events::{HiveDomainEvent, OrganizationCreatedEvent};
use hive_infra::repository::entity::prelude::Organizations;
use hive_infra::HiveOutboxUnitOfWorkImpl;
use rustycog::core::error::{DomainError, ServiceError};
use rustycog::events::{DomainEvent, EventPublisher};
use rustycog::outbox::{
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
            return Err(DomainError::external_service_error(
                "test-publisher",
                "forced publish failure",
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
        "bad_hive_test_event"
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

fn organization_created_event() -> HiveDomainEvent {
    let organization_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    HiveDomainEvent::OrganizationCreated(OrganizationCreatedEvent::new(
        organization_id,
        "Hive Outbox Test".to_string(),
        format!("hive-outbox-{}", &organization_id.to_string()[..8]),
        owner_id,
        chrono::Utc::now(),
    ))
}

#[tokio::test]
#[serial]
async fn hive_outbox_uow_records_pending_event() {
    let descriptor = Arc::new(HiveTestDescriptor);
    let fixture = HiveTestFixture::new(descriptor)
        .await
        .expect("failed to create test fixture");
    let db = fixture.db();
    let db_pool = fixture.fixture.database.as_ref().unwrap().pool.clone();

    let event = organization_created_event();
    let event_id = event.event_id();
    let aggregate_id = event.aggregate_id();

    HiveOutboxUnitOfWorkImpl::new(db_pool, OutboxRecorder::new())
        .record_event(event.into())
        .await
        .expect("outbox recording should succeed");

    let outbox_row = OutboxEvents::find()
        .filter(OutboxColumn::EventId.eq(event_id))
        .one(db.as_ref())
        .await
        .expect("outbox lookup should succeed")
        .expect("outbox row should exist");

    assert_eq!(outbox_row.status, STATUS_PENDING);
    assert_eq!(outbox_row.aggregate_id, aggregate_id);
    assert_eq!(outbox_row.event_type, "organization_created");
}

#[tokio::test]
#[serial]
async fn hive_outbox_uow_rolls_back_failed_recording() {
    let descriptor = Arc::new(HiveTestDescriptor);
    let fixture = HiveTestFixture::new(descriptor)
        .await
        .expect("failed to create test fixture");
    let db = fixture.db();
    let db_pool = fixture.fixture.database.as_ref().unwrap().pool.clone();

    let event_id = Uuid::new_v4();
    let error = HiveOutboxUnitOfWorkImpl::new(db_pool, OutboxRecorder::new())
        .record_event(Box::new(BadEvent { event_id }))
        .await
        .expect_err("bad event should fail outbox recording");
    assert!(
        error.to_string().contains("forced serialization failure"),
        "unexpected error: {error}"
    );

    let outbox_row = OutboxEvents::find()
        .filter(OutboxColumn::EventId.eq(event_id))
        .one(db.as_ref())
        .await
        .expect("outbox lookup should succeed");
    assert!(outbox_row.is_none(), "failed recording should roll back");
}

#[tokio::test]
#[serial]
async fn hive_outbox_dispatcher_marks_success_published() {
    let descriptor = Arc::new(HiveTestDescriptor);
    let fixture = HiveTestFixture::new(descriptor)
        .await
        .expect("failed to create test fixture");
    let db = fixture.db();
    let db_pool = fixture.fixture.database.as_ref().unwrap().pool.clone();

    let event = organization_created_event();
    let event_id = event.event_id();
    HiveOutboxUnitOfWorkImpl::new(db_pool.clone(), OutboxRecorder::new())
        .record_event(event.into())
        .await
        .expect("outbox recording should succeed");

    OutboxDispatcher::new(
        db_pool,
        Arc::new(TestPublisher::success()),
        OutboxConfig::default(),
    )
    .dispatch_once()
    .await
    .expect("dispatch should succeed");

    let outbox_row = OutboxEvents::find()
        .filter(OutboxColumn::EventId.eq(event_id))
        .one(db.as_ref())
        .await
        .expect("outbox lookup should succeed")
        .expect("outbox row should exist");
    assert_eq!(outbox_row.status, STATUS_PUBLISHED);
}

#[tokio::test]
#[serial]
async fn hive_outbox_dispatcher_marks_failure_retryable() {
    let descriptor = Arc::new(HiveTestDescriptor);
    let fixture = HiveTestFixture::new(descriptor)
        .await
        .expect("failed to create test fixture");
    let db = fixture.db();
    let db_pool = fixture.fixture.database.as_ref().unwrap().pool.clone();

    let event = organization_created_event();
    let event_id = event.event_id();
    HiveOutboxUnitOfWorkImpl::new(db_pool.clone(), OutboxRecorder::new())
        .record_event(event.into())
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

    let outbox_row = OutboxEvents::find()
        .filter(OutboxColumn::EventId.eq(event_id))
        .one(db.as_ref())
        .await
        .expect("outbox lookup should succeed")
        .expect("outbox row should exist");
    assert_eq!(outbox_row.status, STATUS_FAILED);
    assert_eq!(outbox_row.attempts, 1);
    assert!(outbox_row.last_error.is_some());
}

#[tokio::test]
#[serial]
async fn hive_create_organization_rolls_back_when_outbox_fails() {
    let descriptor = Arc::new(HiveTestDescriptor);
    let fixture = HiveTestFixture::new(descriptor)
        .await
        .expect("failed to create test fixture");
    let db = fixture.db();
    let db_pool = fixture.fixture.database.as_ref().unwrap().pool.clone();

    let organization = Organization::new(
        "Atomic Rollback Org".to_string(),
        format!("atomic-rollback-{}", &Uuid::new_v4().to_string()[..8]),
        None,
        Uuid::new_v4(),
    )
    .expect("valid organization");
    let organization_id = organization.id;
    let event_id = Uuid::new_v4();

    let error = HiveOutboxUnitOfWorkImpl::new(db_pool, OutboxRecorder::new())
        .create_organization(organization, Box::new(BadEvent { event_id }))
        .await
        .expect_err("bad event should fail the whole transaction");
    assert!(
        error.to_string().contains("forced serialization failure"),
        "unexpected error: {error}"
    );

    let persisted_org = Organizations::find_by_id(organization_id)
        .one(db.as_ref())
        .await
        .expect("organization lookup should succeed");
    assert!(
        persisted_org.is_none(),
        "organization row should roll back with the outbox"
    );

    let persisted_outbox = OutboxEvents::find()
        .filter(OutboxColumn::EventId.eq(event_id))
        .one(db.as_ref())
        .await
        .expect("outbox lookup should succeed");
    assert!(
        persisted_outbox.is_none(),
        "outbox row should roll back with the organization"
    );
}

#[tokio::test]
#[serial]
async fn hive_create_organization_records_domain_and_outbox_together() {
    let descriptor = Arc::new(HiveTestDescriptor);
    let fixture = HiveTestFixture::new(descriptor)
        .await
        .expect("failed to create test fixture");
    let db = fixture.db();
    let db_pool = fixture.fixture.database.as_ref().unwrap().pool.clone();

    let organization = Organization::new(
        "Atomic Success Org".to_string(),
        format!("atomic-success-{}", &Uuid::new_v4().to_string()[..8]),
        None,
        Uuid::new_v4(),
    )
    .expect("valid organization");
    let organization_id = organization.id;
    let event = HiveDomainEvent::OrganizationCreated(OrganizationCreatedEvent::new(
        organization.id,
        organization.name.clone(),
        organization.slug.clone(),
        organization.owner_user_id,
        organization.created_at,
    ));
    let event_id = event.event_id();

    HiveOutboxUnitOfWorkImpl::new(db_pool, OutboxRecorder::new())
        .create_organization(organization, event.into())
        .await
        .expect("organization and outbox should commit together");

    let persisted_org = Organizations::find_by_id(organization_id)
        .one(db.as_ref())
        .await
        .expect("organization lookup should succeed")
        .expect("organization row should exist");
    assert_eq!(persisted_org.id, organization_id);

    let outbox_row = OutboxEvents::find()
        .filter(OutboxColumn::EventId.eq(event_id))
        .one(db.as_ref())
        .await
        .expect("outbox lookup should succeed")
        .expect("outbox row should exist");
    assert_eq!(outbox_row.status, STATUS_PENDING);
    assert_eq!(outbox_row.aggregate_id, organization_id);
    assert_eq!(outbox_row.event_type, "organization_created");
}
