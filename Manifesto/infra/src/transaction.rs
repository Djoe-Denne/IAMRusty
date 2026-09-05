use async_trait::async_trait;
use manifesto_application::{ApplicationError, ProjectAuthorizationUnitOfWork};
use manifesto_domain::entity::{Permission, Project, ProjectMember, Resource, RolePermission};
use manifesto_domain::value_objects::PermissionLevel;
use rustycog::core::error::DomainError;
use rustycog::db::DbConnectionPool;
use rustycog::events::DomainEvent;
use rustycog::outbox::OutboxRecorder;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, PaginatorTrait, QueryFilter, Statement,
};
use std::str::FromStr;

use crate::repository::entity::{prelude::ProjectMembers, project_members};
use crate::repository::{
    MemberWriteRepositoryImpl, PermissionReadRepositoryImpl,
    ProjectMemberRolePermissionWriteRepositoryImpl, ProjectWriteRepositoryImpl,
    ResourceReadRepositoryImpl, RolePermissionReadRepositoryImpl,
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
            lock_project_revision(&txn, project.id, project.revision).await?;
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
}

async fn lock_project_revision<C>(
    db: &C,
    project_id: uuid::Uuid,
    revision: i64,
) -> Result<(), ApplicationError>
where
    C: ConnectionTrait,
{
    if revision == 0 {
        return Ok(());
    }
    let expected_revision = revision.checked_sub(1).ok_or_else(|| {
        ApplicationError::Internal("project revision underflow before persistence".to_string())
    })?;
    let locked = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id FROM projects WHERE id = $1 AND revision = $2 FOR UPDATE",
            [project_id.into(), expected_revision.into()],
        ))
        .await
        .map_err(|error| {
            ApplicationError::Internal(format!("failed to lock project for update: {error}"))
        })?;
    if locked.is_none() {
        return Err(ApplicationError::Validation(
            "Project was modified concurrently; reload it before updating".to_string(),
        ));
    }
    Ok(())
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
