mod common;

use std::sync::Arc;

use common::{ManifestoTestDescriptor, TestFixture};
use manifesto_application::ProjectCreationUnitOfWork;
use manifesto_domain::{
    Project, ProjectMember,
    value_objects::{MemberSource, OwnerType, Visibility},
};
use manifesto_infra::{
    repository::entity::{prelude::*, project_members, role_permissions},
    ProjectCreationUnitOfWorkImpl,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serial_test::serial;
use uuid::Uuid;

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
    let owner_member = ProjectMember::new(project_id, owner_id, MemberSource::Direct, Some(owner_id));

    let uow = ProjectCreationUnitOfWorkImpl::new(db_pool);
    let error = uow
        .create_project_with_owner_permissions(
            project,
            owner_member,
            &["project", "missing-transaction-test-resource"],
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
}
