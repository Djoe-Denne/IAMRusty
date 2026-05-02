use async_trait::async_trait;
use manifesto_application::{ApplicationError, ProjectCreationUnitOfWork};
use manifesto_domain::entity::{Permission, Project, ProjectMember, Resource, RolePermission};
use manifesto_domain::value_objects::PermissionLevel;
use rustycog_core::error::DomainError;
use rustycog_db::DbConnectionPool;
use rustycog_events::DomainEvent;
use rustycog_outbox::OutboxRecorder;

use crate::repository::{
    MemberWriteRepositoryImpl, PermissionReadRepositoryImpl,
    ProjectMemberRolePermissionWriteRepositoryImpl, ProjectWriteRepositoryImpl,
    ResourceReadRepositoryImpl, RolePermissionReadRepositoryImpl,
    RolePermissionWriteRepositoryImpl,
};

#[derive(Clone)]
pub struct ProjectCreationUnitOfWorkImpl {
    db: DbConnectionPool,
    outbox: OutboxRecorder,
}

impl ProjectCreationUnitOfWorkImpl {
    pub const fn new(db: DbConnectionPool, outbox: OutboxRecorder) -> Self {
        Self { db, outbox }
    }
}

#[async_trait]
impl ProjectCreationUnitOfWork for ProjectCreationUnitOfWorkImpl {
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
        permission: normalize_permission(permission)?,
        resource: normalize_resource(resource),
        created_at: None,
    };

    RolePermissionWriteRepositoryImpl::create_with_connection(db, &role_permission).await
}

fn normalize_permission(permission: Permission) -> Result<Permission, DomainError> {
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
