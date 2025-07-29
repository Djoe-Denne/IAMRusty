//! RolePermissionRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::entity::RolePermission;
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
    prelude::RolePermissions,
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
        RolePermission {
            id: model.id,
            organization_role_id: model.organization_role_id,
            permission_id: model.permission_id,
            resource_id: model.resource_id,
            created_at: model.created_at,
        }
    }

    /// Convert a domain role permission to a database active model
    fn to_active_model(role_permission: &RolePermission) -> role_permissions::ActiveModel {
        role_permissions::ActiveModel {
            id: ActiveValue::Set(role_permission.id),
            organization_role_id: ActiveValue::Set(role_permission.organization_role_id),
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
        
        let role_permission = RolePermissions::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        Ok(role_permission.map(Self::to_domain))
    }

    async fn find_by_organization(&self, organization_id: &Uuid) -> Result<Vec<RolePermission>, DomainError> {
        debug!("Finding role permissions by organization ID: {}", organization_id);
        
        let role_permissions = RolePermissions::find()
            .filter(role_permissions::Column::OrganizationId.eq(*organization_id))
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        Ok(role_permissions.into_iter().map(Self::to_domain).collect())
    }

    async fn find_by_organization_role_permission_and_resource(
        &self,
        organization_id: &Uuid,
        permission_id: &Uuid,
        resource_id: &Uuid,
    ) -> Result<Option<RolePermission>, DomainError> {
        debug!(
            "Finding role permission by organization ID: {}, permission ID: {}, resource ID: {}",
            organization_id, permission_id, resource_id
        );
        
        let role_permission = RolePermissions::find()
            .filter(role_permissions::Column::OrganizationId.eq(*organization_id))
            .filter(role_permissions::Column::PermissionId.eq(*permission_id))
            .filter(role_permissions::Column::ResourceId.eq(*resource_id))
            .one(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        Ok(role_permission.map(Self::to_domain))
    }

    async fn save(&self, role_permission: &RolePermission) -> Result<RolePermission, DomainError> {
        debug!("Saving role permission with ID: {}", role_permission.id);
        
        let active_model = Self::to_active_model(role_permission);
        
        let result = active_model
            .save(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        // Convert the saved active model back to domain model
        let saved_model = role_permissions::Model {
            id: result.id.unwrap(),
            organization_id: result.organization_id.unwrap(),
            permission_id: result.permission_id.unwrap(),
            resource_id: result.resource_id.unwrap(),
            created_at: result.created_at.unwrap(),
        };

        Ok(Self::to_domain(saved_model))
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting role permission by ID: {}", id);
        
        let result = RolePermissions::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found("RolePermission", &id.to_string()));
        }

        Ok(())
    }

    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting role permissions by organization ID: {}", organization_id);
        
        let result = RolePermissions::delete_many()
            .filter(role_permissions::Column::OrganizationId.eq(*organization_id))
            .exec(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        debug!("Deleted {} role permissions for organization {}", result.rows_affected, organization_id);
        
        Ok(())
    }
} 