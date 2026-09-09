use async_trait::async_trait;
use manifesto_application::{ApplicationError, ProjectAuthorizationUnitOfWork};
use manifesto_domain::entity::{Permission, Project, ProjectMember, Resource, RolePermission};
use manifesto_domain::value_objects::PermissionLevel;
use rustycog::core::error::DomainError;
use rustycog::db::DbConnectionPool;
use rustycog::events::DomainEvent;
use rustycog::outbox::OutboxRecorder;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Statement,
};
use std::str::FromStr;

use crate::repository::entity::{prelude::ProjectMembers, project_members};
use crate::repository::{
    MemberMapper, MemberWriteRepositoryImpl, PermissionReadRepositoryImpl,
    ProjectMemberRolePermissionWriteRepositoryImpl, ProjectWriteRepositoryImpl,
    ResourceReadRepositoryImpl, ResourceWriteRepositoryImpl, RolePermissionReadRepositoryImpl,
    RolePermissionWriteRepositoryImpl,
};

#[derive(Clone)]
pub struct ProjectAuthorizationUnitOfWorkImpl {
    db: DbConnectionPool,
    outbox: OutboxRecorder,
}

impl ProjectAuthorizationUnitOfWorkImpl {
    pub const fn new(db: DbConnectionPool, outbox: OutboxRecorder) -> Self {
        Self { db, outbox }
    }
}

#[async_trait]
impl ProjectAuthorizationUnitOfWork for ProjectAuthorizationUnitOfWorkImpl {
    async fn create_project_with_owner_permissions(
        &self,
        project: Project,
        owner_member: ProjectMember,
        owner_resource_names: &[&str],
        event: Box<dyn DomainEvent>,
    ) -> Result<(Project, ProjectMember), ApplicationError> {
        let txn =
            self.db.begin_write_transaction().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to begin transaction: {e}"))
            })?;

        let result = async {
            let created_project =
                ProjectWriteRepositoryImpl::save_with_connection(&txn, &project).await?;
            let owner_member =
                MemberWriteRepositoryImpl::save_with_connection(&txn, &owner_member).await?;

            for resource_name in owner_resource_names {
                let role_permission =
                    get_or_create_role_permission(&txn, created_project.id, resource_name, "owner")
                        .await?;

                ProjectMemberRolePermissionWriteRepositoryImpl::grant_known_with_connection(
                    &txn,
                    &owner_member.id,
                    &role_permission,
                )
                .await?;
            }

            self.outbox
                .record(&txn, event.as_ref())
                .await
                .map_err(|e| {
                    ApplicationError::Internal(format!(
                        "failed to record ProjectCreated outbox event: {e}"
                    ))
                })?;

            Ok::<_, ApplicationError>((created_project, owner_member))
        }
        .await;

        match result {
            Ok(created) => {
                txn.commit().await.map_err(|e| {
                    ApplicationError::Internal(format!("failed to commit transaction: {e}"))
                })?;
                Ok(created)
            }
            Err(error) => {
                if let Err(rollback_error) = txn.rollback().await {
                    tracing::error!(
                        "failed to rollback Manifesto project creation transaction: {}",
                        rollback_error
                    );
                }
                Err(error)
            }
        }
    }

    async fn save_project_with_events(
        &self,
        project: Project,
        events: Vec<Box<dyn DomainEvent>>,
    ) -> Result<Project, ApplicationError> {
        let txn =
            self.db.begin_write_transaction().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to begin transaction: {e}"))
            })?;

        let result = async {
            let expected_revision = expected_persisted_revision(project.revision);
            lock_project_revision(&txn, project.id, expected_revision).await?;
            let saved = ProjectWriteRepositoryImpl::save_with_connection(&txn, &project).await?;
            for event in &events {
                self.outbox
                    .record(&txn, event.as_ref())
                    .await
                    .map_err(|e| {
                        ApplicationError::Internal(format!(
                            "failed to record AuthZ outbox event: {e}"
                        ))
                    })?;
            }
            Ok::<_, ApplicationError>(saved)
        }
        .await;

        match result {
            Ok(saved) => {
                txn.commit().await.map_err(|e| {
                    ApplicationError::Internal(format!("failed to commit transaction: {e}"))
                })?;
                Ok(saved)
            }
            Err(error) => {
                if let Err(rollback_error) = txn.rollback().await {
                    tracing::error!(
                        "failed to rollback Manifesto project AuthZ transaction: {}",
                        rollback_error
                    );
                }
                Err(error)
            }
        }
    }

    async fn save_member_with_permission_and_event(
        &self,
        member: ProjectMember,
        resource_name: &str,
        permission: &str,
        member_limit: u32,
        event: Box<dyn DomainEvent>,
    ) -> Result<ProjectMember, ApplicationError> {
        let txn =
            self.db.begin_write_transaction().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to begin transaction: {e}"))
            })?;

        let result = async {
            lock_project_for_member_change(&txn, member.project_id, member.user_id, member_limit)
                .await?;
            let saved_member =
                MemberWriteRepositoryImpl::save_with_connection(&txn, &member).await?;
            let role_permission = get_or_create_role_permission(
                &txn,
                saved_member.project_id,
                resource_name,
                permission,
            )
            .await?;
            ProjectMemberRolePermissionWriteRepositoryImpl::grant_known_with_connection(
                &txn,
                &saved_member.id,
                &role_permission,
            )
            .await?;
            self.outbox
                .record(&txn, event.as_ref())
                .await
                .map_err(|e| {
                    ApplicationError::Internal(format!(
                        "failed to record MemberAdded outbox event: {e}"
                    ))
                })?;
            Ok::<_, ApplicationError>(saved_member)
        }
        .await;

        match result {
            Ok(saved) => {
                txn.commit().await.map_err(|e| {
                    ApplicationError::Internal(format!("failed to commit transaction: {e}"))
                })?;
                Ok(saved)
            }
            Err(error) => {
                if let Err(rollback_error) = txn.rollback().await {
                    tracing::error!(
                        "failed to rollback Manifesto member AuthZ transaction: {}",
                        rollback_error
                    );
                }
                Err(error)
            }
        }
    }

    async fn restore_or_insert_member_with_permission_and_event(
        &self,
        member: ProjectMember,
        resource_name: &str,
        permission: &str,
        member_limit: u32,
        added_by: uuid::Uuid,
    ) -> Result<ProjectMember, ApplicationError> {
        let txn =
            self.db.begin_write_transaction().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to begin transaction: {e}"))
            })?;
        let result = async {
            lock_project_row(&txn, member.project_id).await?;
            if let Some(active) =
                find_active_member_row(&txn, member.project_id, member.user_id).await?
            {
                return Err(ApplicationError::AlreadyExists(format!(
                    "User {} is already a member of project {}",
                    active.user_id, active.project_id
                )));
            }
            let saved = if let Some(mut restorable) =
                find_restorable_member(&txn, member.project_id, member.user_id).await?
            {
                enforce_member_quota(&txn, member.project_id, member_limit).await?;
                restorable
                    .restore(chrono::Utc::now())
                    .map_err(ApplicationError::from)?;
                let saved =
                    MemberWriteRepositoryImpl::save_with_connection(&txn, &restorable).await?;
                replace_member_grants(&txn, &saved, resource_name, permission).await?;
                saved
            } else {
                enforce_member_quota(&txn, member.project_id, member_limit).await?;
                let saved = MemberWriteRepositoryImpl::save_with_connection(&txn, &member).await?;
                replace_member_grants(&txn, &saved, resource_name, permission).await?;
                saved
            };
            let event = manifesto_events::ManifestoDomainEvent::MemberAdded(
                manifesto_events::MemberAddedEvent::new(
                    saved.project_id,
                    saved.id,
                    saved.user_id,
                    permission.to_string(),
                    resource_name.to_string(),
                    added_by,
                    saved.added_at,
                ),
            );
            let boxed: Box<dyn DomainEvent> = event.into();
            record_events(&self.outbox, &txn, std::slice::from_ref(&boxed)).await?;
            load_member_permissions(&txn, saved).await
        }
        .await;
        finish_txn(txn, result).await
    }

    async fn delete_project_with_events(
        &self,
        project: Project,
        events: Vec<Box<dyn DomainEvent>>,
    ) -> Result<(), ApplicationError> {
        let txn =
            self.db.begin_write_transaction().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to begin transaction: {e}"))
            })?;
        let result = async {
            lock_project_revision(&txn, project.id, project.revision).await?;
            record_events(&self.outbox, &txn, &events).await?;
            ProjectWriteRepositoryImpl::delete_by_id_with_connection(&txn, &project.id).await?;
            Ok::<_, ApplicationError>(())
        }
        .await;
        finish_txn(txn, result).await
    }

    async fn replace_member_permissions_with_events(
        &self,
        member: ProjectMember,
        grants: &[(String, String)],
        events: Vec<Box<dyn DomainEvent>>,
    ) -> Result<ProjectMember, ApplicationError> {
        let txn =
            self.db.begin_write_transaction().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to begin transaction: {e}"))
            })?;
        let result = async {
            lock_project_row(&txn, member.project_id).await?;
            let member =
                require_active_locked_member(&txn, member.project_id, member.user_id).await?;
            ProjectMemberRolePermissionWriteRepositoryImpl::revoke_all_for_member_with_connection(
                &txn, &member.id,
            )
            .await?;
            for (resource_name, permission) in grants {
                let role_permission = get_or_create_role_permission(
                    &txn,
                    member.project_id,
                    resource_name,
                    permission,
                )
                .await?;
                ProjectMemberRolePermissionWriteRepositoryImpl::grant_known_with_connection(
                    &txn,
                    &member.id,
                    &role_permission,
                )
                .await?;
            }
            let saved = MemberWriteRepositoryImpl::save_with_connection(&txn, &member).await?;
            record_events(&self.outbox, &txn, &events).await?;
            load_member_permissions(&txn, saved).await
        }
        .await;
        finish_txn(txn, result).await
    }

    async fn remove_member_with_events(
        &self,
        member: ProjectMember,
        grace_period_days: i64,
        events: Vec<Box<dyn DomainEvent>>,
    ) -> Result<(), ApplicationError> {
        let txn =
            self.db.begin_write_transaction().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to begin transaction: {e}"))
            })?;
        let result = async {
            lock_project_row(&txn, member.project_id).await?;
            let mut member =
                require_active_locked_member(&txn, member.project_id, member.user_id).await?;
            member.remove(None, Some(grace_period_days));
            MemberWriteRepositoryImpl::save_with_connection(&txn, &member).await?;
            record_events(&self.outbox, &txn, &events).await?;
            Ok::<_, ApplicationError>(())
        }
        .await;
        finish_txn(txn, result).await
    }

    async fn grant_permission_with_events(
        &self,
        member: ProjectMember,
        resource_name: &str,
        permission: &str,
        events: Vec<Box<dyn DomainEvent>>,
    ) -> Result<ProjectMember, ApplicationError> {
        let txn =
            self.db.begin_write_transaction().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to begin transaction: {e}"))
            })?;
        let result = async {
            lock_project_row(&txn, member.project_id).await?;
            let member =
                require_active_locked_member(&txn, member.project_id, member.user_id).await?;
            let role_permission =
                get_or_create_role_permission(&txn, member.project_id, resource_name, permission)
                    .await?;
            ProjectMemberRolePermissionWriteRepositoryImpl::grant_known_with_connection(
                &txn,
                &member.id,
                &role_permission,
            )
            .await?;
            let saved = MemberWriteRepositoryImpl::save_with_connection(&txn, &member).await?;
            record_events(&self.outbox, &txn, &events).await?;
            load_member_permissions(&txn, saved).await
        }
        .await;
        finish_txn(txn, result).await
    }

    async fn revoke_permission_with_events(
        &self,
        member: ProjectMember,
        role_permission_id: uuid::Uuid,
        events: Vec<Box<dyn DomainEvent>>,
    ) -> Result<(), ApplicationError> {
        let txn =
            self.db.begin_write_transaction().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to begin transaction: {e}"))
            })?;
        let result = async {
            lock_project_row(&txn, member.project_id).await?;
            let member =
                require_active_locked_member(&txn, member.project_id, member.user_id).await?;
            ProjectMemberRolePermissionWriteRepositoryImpl::revoke_with_connection(
                &txn,
                &member.id,
                &role_permission_id,
            )
            .await?;
            record_events(&self.outbox, &txn, &events).await?;
            Ok::<_, ApplicationError>(())
        }
        .await;
        finish_txn(txn, result).await
    }

    async fn transfer_ownership_with_events(
        &self,
        project: Project,
        current_owner: ProjectMember,
        new_owner: ProjectMember,
        personal_project_limit: u32,
        events: Vec<Box<dyn DomainEvent>>,
    ) -> Result<(Project, ProjectMember, ProjectMember), ApplicationError> {
        let txn =
            self.db.begin_write_transaction().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to begin transaction: {e}"))
            })?;
        let result = async {
            lock_project_revision(
                &txn,
                project.id,
                expected_persisted_revision(project.revision),
            )
            .await?;
            if project.owner_type == manifesto_domain::value_objects::OwnerType::Personal {
                enforce_personal_quota_for_transfer(
                    &txn,
                    project.id,
                    new_owner.user_id,
                    personal_project_limit,
                )
                .await?;
            }
            let mut current_owner =
                require_active_locked_member(&txn, project.id, current_owner.user_id).await?;
            let mut new_owner =
                require_active_locked_member(&txn, project.id, new_owner.user_id).await?;
            current_owner.is_owner = false;
            new_owner.is_owner = true;
            demote_owner_permissions(&txn, &current_owner).await?;
            for resource in ["project", "component", "member"] {
                let role_permission =
                    get_or_create_role_permission(&txn, project.id, resource, "owner").await?;
                let already_has = new_owner.role_permissions.iter().any(|rp| {
                    rp.role_permission
                        .resource
                        .name
                        .eq_ignore_ascii_case(resource)
                        && rp.role_permission.permission.level == PermissionLevel::Owner
                });
                if !already_has {
                    ProjectMemberRolePermissionWriteRepositoryImpl::grant_known_with_connection(
                        &txn,
                        &new_owner.id,
                        &role_permission,
                    )
                    .await?;
                }
            }
            let saved_project =
                ProjectWriteRepositoryImpl::save_with_connection(&txn, &project).await?;
            let current_owner =
                MemberWriteRepositoryImpl::save_with_connection(&txn, &current_owner).await?;
            let new_owner =
                MemberWriteRepositoryImpl::save_with_connection(&txn, &new_owner).await?;
            record_events(&self.outbox, &txn, &events).await?;
            Ok::<_, ApplicationError>((
                saved_project,
                load_member_permissions(&txn, current_owner).await?,
                load_member_permissions(&txn, new_owner).await?,
            ))
        }
        .await;
        finish_txn(txn, result).await
    }

    async fn save_component_with_events(
        &self,
        project_id: uuid::Uuid,
        component: manifesto_domain::entity::ProjectComponent,
        create_acl: bool,
        events: Vec<Box<dyn DomainEvent>>,
    ) -> Result<manifesto_domain::entity::ProjectComponent, ApplicationError> {
        let txn =
            self.db.begin_write_transaction().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to begin transaction: {e}"))
            })?;
        let result = async {
            lock_project_row(&txn, project_id).await?;
            if create_acl {
                ResourceWriteRepositoryImpl::create_for_component_instance_with_connection(
                    &txn,
                    &component.id,
                )
                .await?;
            }
            let saved = crate::repository::ComponentWriteRepositoryImpl::save_with_connection(
                &txn, &component,
            )
            .await?;
            record_events(&self.outbox, &txn, &events).await?;
            Ok::<_, ApplicationError>(saved)
        }
        .await;
        finish_txn(txn, result).await
    }

    async fn delete_component_with_events(
        &self,
        project_id: uuid::Uuid,
        component_id: uuid::Uuid,
        events: Vec<Box<dyn DomainEvent>>,
    ) -> Result<(), ApplicationError> {
        let txn =
            self.db.begin_write_transaction().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to begin transaction: {e}"))
            })?;
        let result = async {
            lock_project_row(&txn, project_id).await?;
            crate::repository::ComponentWriteRepositoryImpl::delete_with_connection(
                &txn,
                &component_id,
            )
            .await?;
            ResourceWriteRepositoryImpl::delete_by_id_with_connection(
                &txn,
                &component_id.to_string(),
            )
            .await?;
            record_events(&self.outbox, &txn, &events).await?;
            Ok::<_, ApplicationError>(())
        }
        .await;
        finish_txn(txn, result).await
    }
}

const fn expected_persisted_revision(in_memory_revision: i64) -> i64 {
    if in_memory_revision == 0 {
        0
    } else {
        in_memory_revision - 1
    }
}

async fn lock_project_revision<C>(
    db: &C,
    project_id: uuid::Uuid,
    expected_revision: i64,
) -> Result<(), ApplicationError>
where
    C: ConnectionTrait,
{
    let locked = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id FROM projects WHERE id = $1 AND revision = $2 FOR UPDATE",
            [project_id.into(), expected_revision.into()],
        ))
        .await
        .map_err(|error| {
            ApplicationError::Internal(format!("failed to lock project at revision: {error}"))
        })?;
    if locked.is_some() {
        return Ok(());
    }
    let existing = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id FROM projects WHERE id = $1 FOR UPDATE",
            [project_id.into()],
        ))
        .await
        .map_err(|error| ApplicationError::Internal(format!("failed to lock project: {error}")))?;
    if existing.is_none() && expected_revision == 0 {
        return Ok(());
    }
    Err(ApplicationError::Conflict(
        "Project was modified concurrently; reload it before updating".to_string(),
    ))
}

async fn lock_project_for_member_change<C>(
    db: &C,
    project_id: uuid::Uuid,
    user_id: uuid::Uuid,
    member_limit: u32,
) -> Result<(), ApplicationError>
where
    C: ConnectionTrait,
{
    let locked = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id FROM projects WHERE id = $1 FOR UPDATE",
            [project_id.into()],
        ))
        .await
        .map_err(|error| {
            ApplicationError::Internal(format!("failed to lock project for member change: {error}"))
        })?;
    if locked.is_none() {
        return Err(ApplicationError::from(DomainError::entity_not_found(
            "Project",
            &project_id.to_string(),
        )));
    }

    let existing_member = ProjectMembers::find()
        .filter(project_members::Column::ProjectId.eq(project_id))
        .filter(project_members::Column::UserId.eq(user_id))
        .filter(project_members::Column::RemovedAt.is_null())
        .one(db)
        .await
        .map_err(|error| ApplicationError::Internal(format!("failed to check member: {error}")))?;
    if existing_member.is_some() {
        return Err(ApplicationError::AlreadyExists(format!(
            "User {user_id} is already a member of project {project_id}"
        )));
    }

    let active_members = ProjectMembers::find()
        .filter(project_members::Column::ProjectId.eq(project_id))
        .filter(project_members::Column::RemovedAt.is_null())
        .count(db)
        .await
        .map_err(|error| {
            ApplicationError::Internal(format!("failed to count active project members: {error}"))
        })?;
    if active_members >= u64::from(member_limit) {
        return Err(ApplicationError::Validation(format!(
            "Project {project_id} has reached the maximum number of members ({member_limit})"
        )));
    }
    Ok(())
}

async fn get_or_create_role_permission<C>(
    db: &C,
    project_id: uuid::Uuid,
    resource_name: &str,
    permission_level: &str,
) -> Result<RolePermission, DomainError>
where
    C: sea_orm::ConnectionTrait,
{
    if let Some(existing) =
        RolePermissionReadRepositoryImpl::find_by_project_resource_permission_with_connection(
            db,
            &project_id,
            resource_name,
            permission_level,
        )
        .await?
    {
        return Ok(existing);
    }

    let permission =
        PermissionReadRepositoryImpl::find_by_level_with_connection(db, permission_level)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Permission", permission_level))?;
    let resource = ResourceReadRepositoryImpl::find_by_id_with_connection(db, resource_name)
        .await?
        .ok_or_else(|| DomainError::entity_not_found("Resource", resource_name))?;

    let role_permission = RolePermission {
        id: None,
        name: None,
        project_id,
        permission: normalize_permission(&permission)?,
        resource: normalize_resource(resource),
        created_at: None,
    };

    RolePermissionWriteRepositoryImpl::create_with_connection(db, &role_permission).await
}

fn normalize_permission(permission: &Permission) -> Result<Permission, DomainError> {
    // Keep construction explicit so malformed seeded data fails before write.
    let level = PermissionLevel::from_str(permission.level.to_str())?;
    Ok(Permission {
        level,
        created_at: permission.created_at,
    })
}

const fn normalize_resource(resource: Resource) -> Resource {
    resource
}

async fn finish_txn<T>(
    txn: sea_orm::DatabaseTransaction,
    result: Result<T, ApplicationError>,
) -> Result<T, ApplicationError> {
    match result {
        Ok(value) => {
            txn.commit().await.map_err(|e| {
                ApplicationError::Internal(format!("failed to commit transaction: {e}"))
            })?;
            Ok(value)
        }
        Err(error) => {
            if let Err(rollback_error) = txn.rollback().await {
                tracing::error!("failed to rollback Manifesto AuthZ transaction: {rollback_error}");
            }
            Err(error)
        }
    }
}

async fn record_events<C>(
    outbox: &OutboxRecorder,
    db: &C,
    events: &[Box<dyn DomainEvent>],
) -> Result<(), ApplicationError>
where
    C: ConnectionTrait,
{
    for event in events {
        outbox.record(db, event.as_ref()).await.map_err(|e| {
            ApplicationError::Internal(format!("failed to record AuthZ outbox event: {e}"))
        })?;
    }
    Ok(())
}

async fn lock_project_row<C>(db: &C, project_id: uuid::Uuid) -> Result<(), ApplicationError>
where
    C: ConnectionTrait,
{
    let locked = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id FROM projects WHERE id = $1 FOR UPDATE",
            [project_id.into()],
        ))
        .await
        .map_err(|error| ApplicationError::Internal(format!("failed to lock project: {error}")))?;
    if locked.is_none() {
        return Err(ApplicationError::from(DomainError::entity_not_found(
            "Project",
            &project_id.to_string(),
        )));
    }
    Ok(())
}

async fn find_active_member_row<C>(
    db: &C,
    project_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> Result<Option<ProjectMember>, ApplicationError>
where
    C: ConnectionTrait,
{
    let model = ProjectMembers::find()
        .filter(project_members::Column::ProjectId.eq(project_id))
        .filter(project_members::Column::UserId.eq(user_id))
        .filter(project_members::Column::RemovedAt.is_null())
        .one(db)
        .await
        .map_err(|error| ApplicationError::Internal(format!("failed to check member: {error}")))?;
    match model {
        Some(model) => Ok(Some(
            load_member_permissions(db, MemberMapper::to_domain(model)?).await?,
        )),
        None => Ok(None),
    }
}

async fn find_restorable_member<C>(
    db: &C,
    project_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> Result<Option<ProjectMember>, ApplicationError>
where
    C: ConnectionTrait,
{
    let model = ProjectMembers::find()
        .filter(project_members::Column::ProjectId.eq(project_id))
        .filter(project_members::Column::UserId.eq(user_id))
        .filter(project_members::Column::RemovedAt.is_not_null())
        .filter(project_members::Column::GracePeriodEndsAt.gt(chrono::Utc::now()))
        .order_by_desc(project_members::Column::RemovedAt)
        .one(db)
        .await
        .map_err(|error| {
            ApplicationError::Internal(format!("failed to load restorable member: {error}"))
        })?;
    match model {
        Some(model) => Ok(Some(
            load_member_permissions(db, MemberMapper::to_domain(model)?).await?,
        )),
        None => Ok(None),
    }
}

async fn enforce_member_quota<C>(
    db: &C,
    project_id: uuid::Uuid,
    member_limit: u32,
) -> Result<(), ApplicationError>
where
    C: ConnectionTrait,
{
    let active_members = ProjectMembers::find()
        .filter(project_members::Column::ProjectId.eq(project_id))
        .filter(project_members::Column::RemovedAt.is_null())
        .count(db)
        .await
        .map_err(|error| {
            ApplicationError::Internal(format!("failed to count active project members: {error}"))
        })?;
    if active_members >= u64::from(member_limit) {
        return Err(ApplicationError::Validation(format!(
            "Project {project_id} has reached the maximum number of members ({member_limit})"
        )));
    }
    Ok(())
}

async fn load_member_permissions<C>(
    db: &C,
    mut member: ProjectMember,
) -> Result<ProjectMember, ApplicationError>
where
    C: ConnectionTrait,
{
    member.role_permissions =
        ProjectMemberRolePermissionWriteRepositoryImpl::find_by_member_with_connection(
            db, &member.id,
        )
        .await?;
    Ok(member)
}

async fn demote_owner_permissions<C>(db: &C, member: &ProjectMember) -> Result<(), ApplicationError>
where
    C: ConnectionTrait,
{
    for grant in &member.role_permissions {
        if grant.role_permission.permission.level != PermissionLevel::Owner {
            continue;
        }
        let Some(role_permission_id) = grant.role_permission.id else {
            continue;
        };
        ProjectMemberRolePermissionWriteRepositoryImpl::revoke_with_connection(
            db,
            &member.id,
            &role_permission_id,
        )
        .await?;
        let resource_name = grant.role_permission.resource.name.clone();
        let admin =
            get_or_create_role_permission(db, member.project_id, &resource_name, "admin").await?;
        ProjectMemberRolePermissionWriteRepositoryImpl::grant_known_with_connection(
            db, &member.id, &admin,
        )
        .await?;
    }
    Ok(())
}

async fn replace_member_grants<C>(
    db: &C,
    member: &ProjectMember,
    resource_name: &str,
    permission: &str,
) -> Result<(), ApplicationError>
where
    C: ConnectionTrait,
{
    ProjectMemberRolePermissionWriteRepositoryImpl::revoke_all_for_member_with_connection(
        db, &member.id,
    )
    .await?;
    let role_permission =
        get_or_create_role_permission(db, member.project_id, resource_name, permission).await?;
    ProjectMemberRolePermissionWriteRepositoryImpl::grant_known_with_connection(
        db,
        &member.id,
        &role_permission,
    )
    .await?;
    Ok(())
}

async fn require_active_locked_member<C>(
    db: &C,
    project_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> Result<ProjectMember, ApplicationError>
where
    C: ConnectionTrait,
{
    find_active_member_row(db, project_id, user_id)
        .await?
        .ok_or_else(|| {
            ApplicationError::from(DomainError::permission_denied(
                "Caller is not an active project member",
            ))
        })
}

async fn enforce_personal_quota_for_transfer<C>(
    db: &C,
    project_id: uuid::Uuid,
    new_owner_id: uuid::Uuid,
    personal_project_limit: u32,
) -> Result<(), ApplicationError>
where
    C: ConnectionTrait,
{
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r"
            SELECT id
            FROM projects
            WHERE owner_type = 'personal' AND owner_id = $1 AND id <> $2
            FOR UPDATE
            ",
            [new_owner_id.into(), project_id.into()],
        ))
        .await
        .map_err(|error| {
            ApplicationError::Internal(format!("failed to lock personal projects: {error}"))
        })?;
    if rows.len() >= personal_project_limit as usize {
        return Err(ApplicationError::Validation(format!(
            "Project quota exceeded for personal owner {new_owner_id}"
        )));
    }
    Ok(())
}
