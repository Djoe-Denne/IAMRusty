mod common;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use common::{ManifestoTestDescriptor, TestFixture};
use manifesto_application::ProjectCreationUnitOfWork;
use manifesto_domain::{
    value_objects::{MemberSource, OwnerType, Visibility},
    Project, ProjectMember,
};
use manifesto_events::{ManifestoDomainEvent, ProjectCreatedEvent};
use manifesto_infra::{
    repository::entity::{prelude::*, project_members, role_permissions},
    ProjectCreationUnitOfWorkImpl,
};
use rustycog_core::error::DomainError;
use rustycog_events::{DomainEvent, EventPublisher};
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
    fn success() -> Self {
        Self {
            should_fail: false,
            published_event_ids: Mutex::new(Vec::new()),
        }
    }

    fn failure() -> Self {
        Self {
            should_fail: true,
            published_event_ids: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl EventPublisher<DomainError> for TestPublisher {
    async fn publish(&self, event: &Box<dyn DomainEvent>) -> Result<(), DomainError> {
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

    async fn publish_batch(&self, events: &Vec<Box<dyn DomainEvent>>) -> Result<(), DomainError> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }

    async fn health_check(&self) -> Result<(), DomainError> {
        Ok(())
    }
}

#[tokio::test]
#[serial]
async fn project_creation_unit_of_work_rolls_back_on_permission_failure() {
    let descriptor = Arc::new(ManifestoTestDescriptor);
    let fixture = TestFixture::new(descriptor)
        .await
        .expect("failed to create test fixture");
    let db = fixture.db();
    let db_pool = fixture.database.as_ref().unwrap().pool.clone();

    let owner_id = Uuid::new_v4();
    let project = Project::builder()
        .name("Transactional Rollback Project".to_string())
        .owner_type(OwnerType::Personal)
        .owner_id(owner_id)
        .created_by(owner_id)
        .visibility(Visibility::Private)
        .build()
        .expect("valid project");
    let project_id = project.id;
    let owner_member =
        ProjectMember::new(project_id, owner_id, MemberSource::Direct, Some(owner_id));

    let event = ManifestoDomainEvent::ProjectCreated(ProjectCreatedEvent::new(
        project.id,
        project.name.clone(),
        project.owner_type.as_str().to_string(),
        project.owner_id,
        owner_id,
        project.visibility.as_str().to_string(),
        project.created_at,
    ));
    let uow = ProjectCreationUnitOfWorkImpl::new(db_pool, OutboxRecorder::new());
    let error = uow
        .create_project_with_owner_permissions(
            project,
            owner_member,
            &["project", "missing-transaction-test-resource"],
            event.into(),
        )
        .await
        .expect_err("missing resource should fail transaction");

    assert!(
        error.to_string().contains("Resource"),
        "unexpected error: {error}"
    );

    let persisted_project = Projects::find_by_id(project_id)
        .one(db.as_ref())
        .await
        .expect("project lookup should succeed");
    assert!(persisted_project.is_none(), "project row should roll back");

    let persisted_members = ProjectMembers::find()
        .filter(project_members::Column::ProjectId.eq(project_id))
        .all(db.as_ref())
        .await
        .expect("member lookup should succeed");
    assert!(
        persisted_members.is_empty(),
        "owner member row should roll back"
    );

    let persisted_role_permissions = RolePermissions::find()
        .filter(role_permissions::Column::ProjectId.eq(project_id))
        .all(db.as_ref())
        .await
        .expect("role permission lookup should succeed");
    assert!(
        persisted_role_permissions.is_empty(),
        "role permission rows should roll back"
    );

    let persisted_outbox_rows = OutboxEvents::find()
        .filter(OutboxColumn::AggregateId.eq(project_id))
        .all(db.as_ref())
        .await
        .expect("outbox lookup should succeed");
    assert!(
        persisted_outbox_rows.is_empty(),
        "outbox row should roll back with domain rows"
    );
}

#[tokio::test]
#[serial]
async fn project_creation_unit_of_work_records_project_created_in_outbox() {
    let descriptor = Arc::new(ManifestoTestDescriptor);
    let fixture = TestFixture::new(descriptor)
        .await
        .expect("failed to create test fixture");
    let db = fixture.db();
    let db_pool = fixture.database.as_ref().unwrap().pool.clone();

    let owner_id = Uuid::new_v4();
    let project = Project::builder()
        .name("Outbox Recorded Project".to_string())
        .owner_type(OwnerType::Personal)
        .owner_id(owner_id)
        .created_by(owner_id)
        .visibility(Visibility::Private)
        .build()
        .expect("valid project");
    let project_id = project.id;
    let event = ManifestoDomainEvent::ProjectCreated(ProjectCreatedEvent::new(
        project.id,
        project.name.clone(),
        project.owner_type.as_str().to_string(),
        project.owner_id,
        owner_id,
        project.visibility.as_str().to_string(),
        project.created_at,
    ));
    let event_id = event.event_id();
    let owner_member =
        ProjectMember::new(project_id, owner_id, MemberSource::Direct, Some(owner_id));

    let uow = ProjectCreationUnitOfWorkImpl::new(db_pool, OutboxRecorder::new());
    uow.create_project_with_owner_permissions(
        project,
        owner_member,
        &["project", "component", "member"],
        event.into(),
    )
    .await
    .expect("project creation should succeed");

    let outbox_row = OutboxEvents::find()
        .filter(OutboxColumn::EventId.eq(event_id))
        .one(db.as_ref())
        .await
        .expect("outbox lookup should succeed")
        .expect("outbox row should exist");

    assert_eq!(outbox_row.status, STATUS_PENDING);
    assert_eq!(outbox_row.aggregate_id, project_id);
    assert_eq!(outbox_row.event_type, "project_created");
}

#[tokio::test]
#[serial]
async fn outbox_dispatcher_publish_failure_keeps_project_and_schedules_retry() {
    let descriptor = Arc::new(ManifestoTestDescriptor);
    let fixture = TestFixture::new(descriptor)
        .await
        .expect("failed to create test fixture");
    let db = fixture.db();
    let db_pool = fixture.database.as_ref().unwrap().pool.clone();

    let owner_id = Uuid::new_v4();
    let project = Project::builder()
        .name("Outbox Retry Project".to_string())
        .owner_type(OwnerType::Personal)
        .owner_id(owner_id)
        .created_by(owner_id)
        .visibility(Visibility::Private)
        .build()
        .expect("valid project");
    let project_id = project.id;
    let event = ManifestoDomainEvent::ProjectCreated(ProjectCreatedEvent::new(
        project.id,
        project.name.clone(),
        project.owner_type.as_str().to_string(),
        project.owner_id,
        owner_id,
        project.visibility.as_str().to_string(),
        project.created_at,
    ));
    let event_id = event.event_id();
    let owner_member =
        ProjectMember::new(project_id, owner_id, MemberSource::Direct, Some(owner_id));

    ProjectCreationUnitOfWorkImpl::new(db_pool.clone(), OutboxRecorder::new())
        .create_project_with_owner_permissions(
            project,
            owner_member,
            &["project", "component", "member"],
            event.into(),
        )
        .await
        .expect("project creation should succeed");

    let dispatcher = OutboxDispatcher::new(
        db_pool,
        Arc::new(TestPublisher::failure()),
        OutboxConfig::default(),
    );

    dispatcher
        .dispatch_once()
        .await
        .expect("dispatch cycle should complete");

    let persisted_project = Projects::find_by_id(project_id)
        .one(db.as_ref())
        .await
        .expect("project lookup should succeed");
    assert!(persisted_project.is_some(), "project should stay committed");

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
async fn outbox_dispatcher_publish_success_marks_outbox_row_published() {
    let descriptor = Arc::new(ManifestoTestDescriptor);
    let fixture = TestFixture::new(descriptor)
        .await
        .expect("failed to create test fixture");
    let db = fixture.db();
    let db_pool = fixture.database.as_ref().unwrap().pool.clone();

    let owner_id = Uuid::new_v4();
    let project = Project::builder()
        .name("Outbox Published Project".to_string())
        .owner_type(OwnerType::Personal)
        .owner_id(owner_id)
        .created_by(owner_id)
        .visibility(Visibility::Private)
        .build()
        .expect("valid project");
    let project_id = project.id;
    let event = ManifestoDomainEvent::ProjectCreated(ProjectCreatedEvent::new(
        project.id,
        project.name.clone(),
        project.owner_type.as_str().to_string(),
        project.owner_id,
        owner_id,
        project.visibility.as_str().to_string(),
        project.created_at,
    ));
    let event_id = event.event_id();
    let owner_member =
        ProjectMember::new(project_id, owner_id, MemberSource::Direct, Some(owner_id));

    ProjectCreationUnitOfWorkImpl::new(db_pool.clone(), OutboxRecorder::new())
        .create_project_with_owner_permissions(
            project,
            owner_member,
            &["project", "component", "member"],
            event.into(),
        )
        .await
        .expect("project creation should succeed");

    let publisher = Arc::new(TestPublisher::success());
    let dispatcher = OutboxDispatcher::new(db_pool, publisher.clone(), OutboxConfig::default());

    dispatcher
        .dispatch_once()
        .await
        .expect("dispatch cycle should complete");

    let outbox_row = OutboxEvents::find()
        .filter(OutboxColumn::EventId.eq(event_id))
        .one(db.as_ref())
        .await
        .expect("outbox lookup should succeed")
        .expect("outbox row should exist");
    assert_eq!(outbox_row.status, STATUS_PUBLISHED);

    let published_event_ids = publisher
        .published_event_ids
        .lock()
        .expect("published event lock poisoned");
    assert_eq!(published_event_ids.as_slice(), &[event_id]);
}
