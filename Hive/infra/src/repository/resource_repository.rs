//! ResourceRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::entity::{Resource};
use hive_domain::error::DomainError;
use hive_domain::port::repository::ResourceRepository;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, 
    QueryFilter
};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{
    prelude::Resources,
    resources,
};

/// SeaORM implementation of ResourceRepository
#[derive(Clone)]
pub struct ResourceRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ResourceRepositoryImpl {
    /// Create a new ResourceRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a database model to a domain resource
    fn to_domain(model: resources::Model) -> Result<Resource, DomainError> {
        Ok(Resource {
            name: model.name,
            description: model.description,
            created_at: Some(model.created_at),
        })
    }
}

#[async_trait]
impl ResourceRepository for ResourceRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Resource>, DomainError> {
        debug!("Finding resource by ID: {}", id);
        
        let resource = Resources::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match resource {
            Some(model) => Ok(Some(Self::to_domain(model)?)),
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
            Some(model) => Ok(Some(Self::to_domain(model)?)),
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
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }
} 