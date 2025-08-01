//! PermissionRepository SeaORM implementation

use async_trait::async_trait;
use hive_domain::entity::{Permission, PermissionLevel};
use hive_domain::error::DomainError;
use hive_domain::port::repository::PermissionRepository;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter
};
use std::sync::Arc;
use tracing::debug;
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
        Ok(Permission::new(
            PermissionLevel::from_str(&model.level)?,
            model.description,
            model.created_at,
        ))
    }

}

#[async_trait]
impl PermissionRepository for PermissionRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Permission>, DomainError> {
        debug!("Finding permission by ID: {}", id);
        
        let permission = Permissions::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

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
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

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
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in permissions {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

} 