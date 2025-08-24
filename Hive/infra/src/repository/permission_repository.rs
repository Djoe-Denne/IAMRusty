//! PermissionRepository SeaORM implementation

use async_trait::async_trait;
use hive_domain::entity::{Permission, PermissionLevel};
use rustycog_core::error::DomainError;
use hive_domain::port::repository::{PermissionReadRepository, PermissionRepository};
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

pub struct PermissionMapper;

impl PermissionMapper {
    
    pub fn to_domain(model: permissions::Model) -> Result<Permission, DomainError> {
        Ok(Permission::new(
            PermissionLevel::from_str(&model.level)?,
            model.description,
            Some(model.created_at),
        ))
    }
}

/// Read repository (permissions are read-only)
#[derive(Clone)]
pub struct PermissionReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl PermissionReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self { Self { db } }

}

#[async_trait]
impl PermissionReadRepository for PermissionReadRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Permission>, DomainError> {
        debug!("Finding permission by ID: {}", id);
        
        let permission = Permissions::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match permission {
            Some(model) => Ok(Some(PermissionMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_level(&self, level: &PermissionLevel) -> Result<Option<Permission>, DomainError> {
        debug!("Finding permission by level: {}", level.to_str());
        
        let permission = Permissions::find()
            .filter(permissions::Column::Level.eq(level.to_str()))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match permission {
            Some(model) => Ok(Some(PermissionMapper::to_domain(model)?)),
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
            result.push(PermissionMapper::to_domain(model)?);
        }
        Ok(result)
    }

} 

#[derive(Clone)]
pub struct PermissionRepositoryImpl {
    read_repo: Arc<dyn PermissionReadRepository>,
}

impl PermissionRepositoryImpl {
    pub fn new(read_repo: Arc<dyn PermissionReadRepository>) -> Self { Self { read_repo } }
}

#[async_trait]
impl PermissionReadRepository for PermissionRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Permission>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_level(&self, level: &PermissionLevel) -> Result<Option<Permission>, DomainError> {
        self.read_repo.find_by_level(level).await
    }

    async fn find_all(&self) -> Result<Vec<Permission>, DomainError> {
        self.read_repo.find_all().await
    }
}

#[async_trait]
impl PermissionRepository for PermissionRepositoryImpl {}
