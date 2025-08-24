//! ResourceRepository SeaORM implementation

use async_trait::async_trait;
use hive_domain::entity::{Resource};
use rustycog_core::error::DomainError;
use hive_domain::port::repository::{ResourceReadRepository, ResourceRepository};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, 
    QueryFilter
};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{
    prelude::Resources,
    resources,
};

pub struct ResourceMapper;

impl ResourceMapper {
    
    pub fn to_domain(model: resources::Model) -> Result<Resource, DomainError> {
        Ok(Resource {
            name: model.name,
            description: model.description,
            created_at: Some(model.created_at),
        })
    }
}

/// Read repository (resources are read-only)
#[derive(Clone)]
pub struct ResourceReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ResourceReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self { Self { db } }


}

#[async_trait]
impl ResourceReadRepository for ResourceReadRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Resource>, DomainError> {
        debug!("Finding resource by ID: {}", id);
        
        let resource = Resources::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match resource {
            Some(model) => Ok(Some(ResourceMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_type(&self, resource_type: &String) -> Result<Option<Resource>, DomainError> {
        debug!("Finding resource by type: {}", resource_type.as_str());
        
        let resource = Resources::find()
            .filter(resources::Column::ResourceType.eq(resource_type.as_str()))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match resource {
            Some(model) => Ok(Some(ResourceMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<Resource>, DomainError> {
        debug!("Finding all resources");
        
        let resources = Resources::find()
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in resources {
            result.push(ResourceMapper::to_domain(model)?);
        }
        Ok(result)
    }
} 

#[derive(Clone)]
pub struct ResourceRepositoryImpl {
    read_repo: Arc<dyn ResourceReadRepository>,
}

impl ResourceRepositoryImpl {
    pub fn new(read_repo: Arc<dyn ResourceReadRepository>) -> Self { Self { read_repo } }
}

#[async_trait]
impl ResourceReadRepository for ResourceRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Resource>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_type(&self, resource_type: &String) -> Result<Option<Resource>, DomainError> {
        self.read_repo.find_by_type(resource_type).await
    }

    async fn find_all(&self) -> Result<Vec<Resource>, DomainError> {
        self.read_repo.find_all().await
    }
}

#[async_trait]
impl ResourceRepository for ResourceRepositoryImpl {}