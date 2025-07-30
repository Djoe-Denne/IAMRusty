//! RolePermissionRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::role_permission;
use hive_domain::{entity::{permission::Permission, resource::Resource, RolePermission}};
use hive_domain::entity::permission::PermissionLevel;
use hive_domain::error::DomainError;
use hive_domain::port::repository::RolePermissionRepository;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, 
    QueryFilter, Set
};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{
    prelude::MemberRoles,
    member_roles,
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
        RolePermission {
            id: model.id,
            name: model.name,
            description: model.description,
            permission: Permission::new(
                PermissionLevel::from_str(&model.permission_id),
                model.description.clone(),
                model.created_at.naive_utc(),
            ),
            resource: Resource::new(
                model.resource_id.to_string(),
                model.description.clone(),
                model.created_at.naive_utc(),
            ),
            created_at: model.created_at.naive_utc(),
        }
    }

    /// Convert a domain role permission to a database active model
    fn to_active_model(role_permission: &RolePermission) -> role_permissions::ActiveModel {
        role_permissions::ActiveModel {
            id: ActiveValue::Set(role_permission.id),
            name: ActiveValue::Set(role_permission.name),
            description: ActiveValue::Set(role_permission.description),
            permission_id: ActiveValue::Set(role_permission.permission_id),
            resource_id: ActiveValue::Set(role_permission.resource_id),
            created_at: ActiveValue::Set(role_permission.created_at),
        }
    }
}

#[async_trait]
impl RolePermissionRepository for RolePermissionRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<RolePermission>, DomainError> {
        debug!("Finding role permission by ID: {}", id);

        let role_permission = RolePermissions::find().filter(role_permissions::Column::Id.eq(*id)).one(self.db.as_ref()).await.map_err(DomainError::from)?;

        Ok(role_permission.map(Self::to_domain))
    }

    async fn find_by_organization_resource_permission(&self, organization_id: &Uuid, role_permission: &RolePermission) -> Result<Vec<RolePermission>, DomainError> {
        debug!("Finding role permissions by organization ID: {} and resource type: {} and permission ID: {}", organization_id, role_permission.resource_id, role_permission.permission_id);

        let role_permissions = RolePermissions::find()
            .filter(role_permissions::Column::OrganizationId.eq(*organization_id))
            .filter(role_permissions::Column::ResourceId.eq(role_permission.resource_id))
            .filter(role_permissions::Column::PermissionId.eq(role_permission.permission_id))
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        Ok(role_permissions.into_iter().map(Self::to_domain).collect())
    }

    async fn save(&self, organization_id: &Uuid, role_permission: &RolePermission) -> Result<RolePermission, DomainError> {
        debug!("Saving role permission with ID: {}", role_permission.id);

        let active_model = Self::to_active_model(role_permission);

        let result = active_model.save(self.db.as_ref()).await.map_err(DomainError::from)?;

        Ok(Self::to_domain(result))
    } 

    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting role permissions by organization ID: {}", organization_id);

        let result = RolePermissions::delete_many().filter(role_permissions::Column::OrganizationId.eq(*organization_id)).exec(self.db.as_ref()).await.map_err(DomainError::from)?;

        Ok(())
    }
} 