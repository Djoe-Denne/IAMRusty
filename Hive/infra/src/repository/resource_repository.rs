//! ResourceRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::entity::{Resource, ResourceType};
use hive_domain::error::DomainError;
use hive_domain::port::repository::ResourceRepository;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, 
    QueryFilter, Set
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
            created_at: model.created_at,
        })
    }

    /// Convert a domain resource to a database active model
    fn to_active_model(resource: &Resource) -> resources::ActiveModel {
        resources::ActiveModel {
            id: ActiveValue::Set(resource.id),
            resource_type: ActiveValue::Set(resource.resource_type.as_str().to_string()),
            name: ActiveValue::Set(resource.name.clone()),
            description: ActiveValue::Set(resource.description.clone()),
            created_at: ActiveValue::Set(resource.created_at),
        }
    }
}

#[async_trait]
impl ResourceRepository for ResourceRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Resource>, DomainError> {
        debug!("Finding resource by ID: {}", id);
        
        let resource = Resources::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

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
            .map_err(DomainError::from)?;

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
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in resources {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn save(&self, resource: &Resource) -> Result<Resource, DomainError> {
        debug!("Saving resource with ID: {}", resource.id);
        
        let active_model = Self::to_active_model(resource);
        
        let result = active_model
            .save(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        // Convert the saved active model back to domain model
        let saved_model = resources::Model {
            id: result.id.unwrap(),
            resource_type: result.resource_type.unwrap(),
            name: result.name.unwrap(),
            description: result.description.unwrap(),
            created_at: result.created_at.unwrap(),
        };

        Self::to_domain(saved_model)
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting resource by ID: {}", id);
        
        let result = Resources::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found("Resource", &id.to_string()));
        }

        Ok(())
    }
} 