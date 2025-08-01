//! RolePermissionRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::role_permission;
use hive_domain::{entity::{permission::Permission, resource::Resource, RolePermission}};
use hive_domain::entity::permission::PermissionLevel;
use hive_domain::error::DomainError;
use hive_domain::port::repository::RolePermissionRepository;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, 
    QueryFilter, TryIntoModel
};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{
    prelude::RolePermissions as OrganizationRolePermissions,
    role_permissions,
};

/// SeaORM implementation of RolePermissionRepository
#[derive(Clone)]
pub struct RolePermissionRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl RolePermissionRepositoryImpl {
    /// Create a new RolePermissionRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a database model to a domain role permission
    fn to_domain(model: role_permissions::Model) -> RolePermission {
        RolePermission::new(
            Some(model.id),
            Some(model.name.clone()),
            model.organization_id,
            &Permission::new(model.permission_id.into(), None, model.created_at),
            &Resource::new(model.resource_id.into(), None, Some(model.created_at)),
            Some(model.created_at)
        )
    }

    /// Convert a domain role permission to a database active model
    fn to_active_model(role_permission: &RolePermission) -> role_permissions::ActiveModel {
        role_permissions::ActiveModel {
            id: ActiveValue::Set(role_permission.id.unwrap_or(Uuid::new_v4())),
            organization_id: ActiveValue::Set(role_permission.organization_id),
            permission_id: ActiveValue::Set(role_permission.permission.level.as_str().to_string()),
            resource_id: ActiveValue::Set(role_permission.resource.name.clone()),
            description: ActiveValue::Set(role_permission.resource.description.clone()),
            created_at: ActiveValue::Set(role_permission.created_at.unwrap()),
            name: ActiveValue::Set(role_permission.name.clone().unwrap_or_default()),
        }
    }
}

#[async_trait]
impl RolePermissionRepository for RolePermissionRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<RolePermission>, DomainError> {
        debug!("Finding role permission by ID: {}", id);

        let role_permission = OrganizationRolePermissions::find().filter(role_permissions::Column::Id.eq(*id)).one(self.db.as_ref()).await.map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(role_permission.map(Self::to_domain))
    }

    async fn find_by_organization_role(&self, organization_id: &Uuid, resource_type: &str, permission: &str) -> Result<Option<RolePermission>, DomainError> {
        debug!("Finding role permissions by organization ID: {} and resource type: {} and permission ID: {}", organization_id, resource_type, permission);

        let role_permissions = OrganizationRolePermissions::find()
            .filter(role_permissions::Column::OrganizationId.eq(*organization_id))
            .filter(role_permissions::Column::ResourceId.eq(resource_type))
            .filter(role_permissions::Column::PermissionId.eq(permission))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(role_permissions.into_iter().map(Self::to_domain).next())
    }

    async fn find_by_organization_roles(&self, organization_id: &Uuid, role_permissions: &Vec<RolePermission>) -> Result<Vec<RolePermission>, DomainError> {
        debug!("Finding role permissions by organization ID: {} and role permissions: {:?}", organization_id, role_permissions);

        let role_permissions = OrganizationRolePermissions::find()
            .filter(role_permissions::Column::OrganizationId.eq(*organization_id))
            .filter(role_permissions::Column::ResourceId.is_in(role_permissions.iter().map(|role| role.resource.name.clone()).collect::<Vec<_>>()))
            .filter(role_permissions::Column::PermissionId.is_in(role_permissions.iter().map(|role| role.permission.level.as_str().to_string()).collect::<Vec<_>>()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(role_permissions.into_iter().map(Self::to_domain).collect())
    }

    async fn save(&self, organization_id: &Uuid, role_permission: &RolePermission) -> Result<RolePermission, DomainError> {
        debug!("Saving role permission with {} for organization {}", role_permission.name.as_ref().map(|n| n.as_str()).unwrap_or("Unknown"), organization_id);

        let active_model = Self::to_active_model(role_permission);

        let result = active_model.save(self.db.as_ref()).await.map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(Self::to_domain(result.try_into_model().map_err(|e| DomainError::internal_error(&e.to_string()))?))
    } 

    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting role permissions by organization ID: {}", organization_id);

        let _result = OrganizationRolePermissions::delete_many().filter(role_permissions::Column::OrganizationId.eq(*organization_id)).exec(self.db.as_ref()).await.map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(())
    }
} 