use async_trait::async_trait;
use manifesto_domain::entity::{ProjectMemberRolePermission, RolePermission};
use manifesto_domain::port::{
    ProjectMemberRolePermissionReadRepository, ProjectMemberRolePermissionRepository,
    ProjectMemberRolePermissionWriteRepository, RolePermissionReadRepository,
};
use rustycog_core::error::DomainError;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{prelude::*, project_member_role_permissions};
use super::role_permission_repository::RolePermissionReadRepositoryImpl;

pub struct ProjectMemberRolePermissionMapper;

impl ProjectMemberRolePermissionMapper {
    pub fn to_domain(
        model: project_member_role_permissions::Model,
        role_permission: RolePermission,
    ) -> ProjectMemberRolePermission {
        ProjectMemberRolePermission {
            id: Some(model.id),
            member_id: model.member_id,
            role_permission,
            created_at: model.created_at.naive_utc().and_utc(),
        }
    }

    pub fn to_active_model(
        pmrp: &ProjectMemberRolePermission,
        insert: bool,
    ) -> project_member_role_permissions::ActiveModel {
        let id = if insert {
            ActiveValue::NotSet
        } else {
            ActiveValue::Set(pmrp.id.unwrap_or_else(Uuid::new_v4))
        };

        project_member_role_permissions::ActiveModel {
            id,
            member_id: ActiveValue::Set(pmrp.member_id),
            role_permission_id: ActiveValue::Set(pmrp.role_permission.id.unwrap()),
            created_at: ActiveValue::NotSet,
        }
    }
}

#[derive(Clone)]
pub struct ProjectMemberRolePermissionReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
    role_permission_repo: Arc<RolePermissionReadRepositoryImpl>,
}

impl ProjectMemberRolePermissionReadRepositoryImpl {
    pub fn new(
        db: Arc<DatabaseConnection>,
        role_permission_repo: Arc<RolePermissionReadRepositoryImpl>,
    ) -> Self {
        Self {
            db,
            role_permission_repo,
        }
    }

    async fn load_with_relations(
        &self,
        model: project_member_role_permissions::Model,
    ) -> Result<ProjectMemberRolePermission, DomainError> {
        // Load role permission
        let role_permission = self
            .role_permission_repo
            .find_by_id(&model.role_permission_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("RolePermission", "unknown"))?;

        Ok(ProjectMemberRolePermissionMapper::to_domain(
            model,
            role_permission,
        ))
    }
}

#[async_trait]
impl ProjectMemberRolePermissionReadRepository for ProjectMemberRolePermissionReadRepositoryImpl {
    async fn find_by_member(
        &self,
        member_id: &Uuid,
    ) -> Result<Vec<ProjectMemberRolePermission>, DomainError> {
        debug!("Finding role permissions for member: {}", member_id);

        let models = ProjectMemberRolePermissions::find()
            .filter(project_member_role_permissions::Column::MemberId.eq(*member_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in models {
            result.push(self.load_with_relations(model).await?);
        }

        Ok(result)
    }
}

#[derive(Clone)]
pub struct ProjectMemberRolePermissionWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
    role_permission_repo: Arc<RolePermissionReadRepositoryImpl>,
}

impl ProjectMemberRolePermissionWriteRepositoryImpl {
    pub fn new(
        db: Arc<DatabaseConnection>,
        role_permission_repo: Arc<RolePermissionReadRepositoryImpl>,
    ) -> Self {
        Self {
            db,
            role_permission_repo,
        }
    }

    async fn load_with_relations(
        &self,
        model: project_member_role_permissions::Model,
    ) -> Result<ProjectMemberRolePermission, DomainError> {
        // Load role permission
        let role_permission = self
            .role_permission_repo
            .find_by_id(&model.role_permission_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("RolePermission", "unknown"))?;

        Ok(ProjectMemberRolePermissionMapper::to_domain(
            model,
            role_permission,
        ))
    }

    pub async fn grant_known_with_connection<C>(
        db: &C,
        member_id: &Uuid,
        role_permission: &RolePermission,
    ) -> Result<ProjectMemberRolePermission, DomainError>
    where
        C: ConnectionTrait,
    {
        let role_permission_id = role_permission.id.ok_or_else(|| {
            DomainError::internal_error("role permission must have an id before grant")
        })?;

        let active_model = project_member_role_permissions::ActiveModel {
            id: ActiveValue::NotSet,
            member_id: ActiveValue::Set(*member_id),
            role_permission_id: ActiveValue::Set(role_permission_id),
            created_at: ActiveValue::NotSet,
        };

        let model = active_model
            .insert(db)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(ProjectMemberRolePermissionMapper::to_domain(
            model,
            role_permission.clone(),
        ))
    }
}

#[async_trait]
impl ProjectMemberRolePermissionWriteRepository for ProjectMemberRolePermissionWriteRepositoryImpl {
    async fn grant(
        &self,
        member_id: &Uuid,
        role_permission_id: &Uuid,
    ) -> Result<ProjectMemberRolePermission, DomainError> {
        debug!(
            "Granting role permission {} to member {}",
            role_permission_id, member_id
        );

        let active_model = project_member_role_permissions::ActiveModel {
            id: ActiveValue::NotSet,
            member_id: ActiveValue::Set(*member_id),
            role_permission_id: ActiveValue::Set(*role_permission_id),
            created_at: ActiveValue::NotSet,
        };

        let model = active_model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        self.load_with_relations(model).await
    }

    async fn revoke(&self, member_id: &Uuid, role_permission_id: &Uuid) -> Result<(), DomainError> {
        debug!(
            "Revoking role permission {} from member {}",
            role_permission_id, member_id
        );

        ProjectMemberRolePermissions::delete_many()
            .filter(project_member_role_permissions::Column::MemberId.eq(*member_id))
            .filter(project_member_role_permissions::Column::RolePermissionId.eq(*role_permission_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(())
    }

    async fn revoke_all_for_member(&self, member_id: &Uuid) -> Result<(), DomainError> {
        debug!("Revoking all role permissions for member {}", member_id);

        ProjectMemberRolePermissions::delete_many()
            .filter(project_member_role_permissions::Column::MemberId.eq(*member_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct ProjectMemberRolePermissionRepositoryImpl {
    read_repo: Arc<ProjectMemberRolePermissionReadRepositoryImpl>,
    write_repo: Arc<ProjectMemberRolePermissionWriteRepositoryImpl>,
}

impl ProjectMemberRolePermissionRepositoryImpl {
    pub fn new(
        read_repo: Arc<ProjectMemberRolePermissionReadRepositoryImpl>,
        write_repo: Arc<ProjectMemberRolePermissionWriteRepositoryImpl>,
    ) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl ProjectMemberRolePermissionReadRepository for ProjectMemberRolePermissionRepositoryImpl {
    async fn find_by_member(
        &self,
        member_id: &Uuid,
    ) -> Result<Vec<ProjectMemberRolePermission>, DomainError> {
        self.read_repo.find_by_member(member_id).await
    }
}

#[async_trait]
impl ProjectMemberRolePermissionWriteRepository for ProjectMemberRolePermissionRepositoryImpl {
    async fn grant(
        &self,
        member_id: &Uuid,
        role_permission_id: &Uuid,
    ) -> Result<ProjectMemberRolePermission, DomainError> {
        self.write_repo.grant(member_id, role_permission_id).await
    }

    async fn revoke(&self, member_id: &Uuid, role_permission_id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.revoke(member_id, role_permission_id).await
    }

    async fn revoke_all_for_member(&self, member_id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.revoke_all_for_member(member_id).await
    }
}

impl ProjectMemberRolePermissionRepository for ProjectMemberRolePermissionRepositoryImpl {}

