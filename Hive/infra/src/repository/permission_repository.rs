//! PermissionRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::entity::{Permission, PermissionLevel};
use hive_domain::error::DomainError;
use hive_domain::port::repository::PermissionRepository;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, 
    QueryFilter, Set
};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{
    prelude::Permissions,
    permissions,
};

/// SeaORM implementation of PermissionRepository
#[derive(Clone)]
pub struct PermissionRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl PermissionRepositoryImpl {
    /// Create a new PermissionRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a database model to a domain permission
    fn to_domain(model: permissions::Model) -> Result<Permission, DomainError> {
        let level = PermissionLevel::from_str(&model.level)?;
        Ok(Permission::new(
            PermissionLevel::from_str(level),
            model.description,
            model.created_at,
        ))
    }

    /// Convert a domain permission to a database active model
    fn to_active_model(permission: &Permission) -> permissions::ActiveModel {
        permissions::ActiveModel {
            id: ActiveValue::Set(permission.id),
            level: ActiveValue::Set(permission.level.as_str().to_string()),
            description: ActiveValue::Set(permission.description.clone()),
            created_at: ActiveValue::Set(permission.created_at),
        }
    }
}

#[async_trait]
impl PermissionRepository for PermissionRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Permission>, DomainError> {
        debug!("Finding permission by ID: {}", id);
        
        let permission = Permissions::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        match permission {
            Some(model) => Ok(Some(Self::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_level(&self, level: &PermissionLevel) -> Result<Option<Permission>, DomainError> {
        debug!("Finding permission by level: {}", level.as_str());
        
        let permission = Permissions::find()
            .filter(permissions::Column::Level.eq(level.as_str()))
            .one(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        match permission {
            Some(model) => Ok(Some(Self::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<Permission>, DomainError> {
        debug!("Finding all permissions");
        
        let permissions = Permissions::find()
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in permissions {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn save(&self, permission: &Permission) -> Result<Permission, DomainError> {
        debug!("Saving permission with ID: {}", permission.id);
        
        let active_model = Self::to_active_model(permission);
        
        let result = active_model
            .save(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        // Convert the saved active model back to domain model
        let saved_model = permissions::Model {
            id: result.id.unwrap(),
            level: result.level.unwrap(),
            description: result.description.unwrap(),
            created_at: result.created_at.unwrap(),
        };

        Self::to_domain(saved_model)
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting permission by ID: {}", id);
        
        let result = Permissions::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found("Permission", &id.to_string()));
        }

        Ok(())
    }
} 