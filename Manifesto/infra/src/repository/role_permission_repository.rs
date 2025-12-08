use async_trait::async_trait;
use manifesto_domain::entity::{Permission, Resource, RolePermission};
use manifesto_domain::port::{
    RolePermissionReadRepository, RolePermissionRepository, RolePermissionWriteRepository,
};
use manifesto_domain::value_objects::PermissionLevel;
use rustycog_core::error::DomainError;
use sea_orm::{
    ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait,
};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{
    permissions, prelude::*, resources, role_permissions,
};

pub struct RolePermissionMapper;

impl RolePermissionMapper {
    pub fn to_domain(
        role_model: role_permissions::Model,
        permission_model: permissions::Model,
        resource_model: resources::Model,
    ) -> Result<RolePermission, DomainError> {
        let permission_level = PermissionLevel::from_str(&permission_model.level)?;
        let permission = Permission {
            level: permission_level,
            created_at: Some(permission_model.created_at.naive_utc().and_utc()),
        };

        let resource = Resource {
            name: resource_model.name,
            created_at: Some(resource_model.created_at.naive_utc().and_utc()),
        };

        Ok(RolePermission {
            id: Some(role_model.id),
            name: role_model.name,
            project_id: role_model.project_id,
            permission,
            resource,
            created_at: Some(role_model.created_at.naive_utc().and_utc()),
        })
    }

    pub fn to_active_model(
        role_permission: &RolePermission,
        insert: bool,
    ) -> role_permissions::ActiveModel {
        let id = if insert {
            ActiveValue::NotSet
        } else {
            ActiveValue::Set(role_permission.id.unwrap_or_else(Uuid::new_v4))
        };

        role_permissions::ActiveModel {
            id,
            name: ActiveValue::Set(role_permission.name.clone()),
            project_id: ActiveValue::Set(role_permission.project_id),
            permission_id: ActiveValue::Set(role_permission.permission.level.to_str().to_string()),
            resource_id: ActiveValue::Set(role_permission.resource.name.clone()),
            created_at: ActiveValue::NotSet,
        }
    }
}

#[derive(Clone)]
pub struct RolePermissionReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl RolePermissionReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    async fn load_with_relations(
        &self,
        role_model: role_permissions::Model,
    ) -> Result<RolePermission, DomainError> {
        // Load permission
        let permission = Permissions::find_by_id(role_model.permission_id.clone())
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .ok_or_else(|| DomainError::entity_not_found("Permission", "unknown"))?;

        // Load resource
        let resource = Resources::find_by_id(role_model.resource_id.clone())
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .ok_or_else(|| DomainError::entity_not_found("Resource", "unknown"))?;

        RolePermissionMapper::to_domain(role_model, permission, resource)
    }
}

#[async_trait]
impl RolePermissionReadRepository for RolePermissionReadRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<RolePermission>, DomainError> {
        debug!("Finding role permission by ID: {}", id);

        let role = RolePermissions::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match role {
            Some(r) => Ok(Some(self.load_with_relations(r).await?)),
            None => Ok(None),
        }
    }

    async fn find_by_project(&self, project_id: &Uuid) -> Result<Vec<RolePermission>, DomainError> {
        debug!("Finding role permissions for project: {}", project_id);

        let roles = RolePermissions::find()
            .filter(role_permissions::Column::ProjectId.eq(*project_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for role in roles {
            result.push(self.load_with_relations(role).await?);
        }

        Ok(result)
    }

    async fn find_by_project_resource_permission(
        &self,
        project_id: &Uuid,
        resource_name: &str,
        permission_level: &str,
    ) -> Result<Option<RolePermission>, DomainError> {
        debug!(
            "Finding role permission for project: {}, resource: {}, permission: {}",
            project_id, resource_name, permission_level
        );

        let role = RolePermissions::find()
            .filter(role_permissions::Column::ProjectId.eq(*project_id))
            .filter(role_permissions::Column::ResourceId.eq(resource_name))
            .filter(role_permissions::Column::PermissionId.eq(permission_level))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match role {
            Some(r) => Ok(Some(self.load_with_relations(r).await?)),
            None => Ok(None),
        }
    }

}

#[derive(Clone)]
pub struct RolePermissionWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl RolePermissionWriteRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    async fn load_with_relations(
        &self,
        role_model: role_permissions::Model,
    ) -> Result<RolePermission, DomainError> {
        // Load permission
        let permission = Permissions::find_by_id(role_model.permission_id.clone())
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .ok_or_else(|| DomainError::entity_not_found("Permission", "unknown"))?;

        // Load resource
        let resource = Resources::find_by_id(role_model.resource_id.clone())
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .ok_or_else(|| DomainError::entity_not_found("Resource", "unknown"))?;

        RolePermissionMapper::to_domain(role_model, permission, resource)
    }
}

#[async_trait]
impl RolePermissionWriteRepository for RolePermissionWriteRepositoryImpl {
    async fn create(&self, role_permission: &RolePermission) -> Result<RolePermission, DomainError> {
        debug!("Creating role permission");

        let active_model = RolePermissionMapper::to_active_model(role_permission, true);

        let model = active_model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        self.load_with_relations(model).await
    }

    async fn delete(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting role permission: {}", id);

        RolePermissions::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct RolePermissionRepositoryImpl {
    read_repo: Arc<RolePermissionReadRepositoryImpl>,
    write_repo: Arc<RolePermissionWriteRepositoryImpl>,
}

impl RolePermissionRepositoryImpl {
    pub fn new(
        read_repo: Arc<RolePermissionReadRepositoryImpl>,
        write_repo: Arc<RolePermissionWriteRepositoryImpl>,
    ) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl RolePermissionReadRepository for RolePermissionRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<RolePermission>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_project(&self, project_id: &Uuid) -> Result<Vec<RolePermission>, DomainError> {
        self.read_repo.find_by_project(project_id).await
    }

    async fn find_by_project_resource_permission(
        &self,
        project_id: &Uuid,
        resource_name: &str,
        permission_level: &str,
    ) -> Result<Option<RolePermission>, DomainError> {
        self.read_repo
            .find_by_project_resource_permission(project_id, resource_name, permission_level)
            .await
    }
}

#[async_trait]
impl RolePermissionWriteRepository for RolePermissionRepositoryImpl {
    async fn create(&self, role_permission: &RolePermission) -> Result<RolePermission, DomainError> {
        self.write_repo.create(role_permission).await
    }

    async fn delete(&self, id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.delete(id).await
    }
}

impl RolePermissionRepository for RolePermissionRepositoryImpl {}

