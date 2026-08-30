//! `ResourceRepository` `SeaORM` implementation

use async_trait::async_trait;
use hive_domain::entity::Resource;
use hive_domain::port::repository::{ResourceReadRepository, ResourceRepository};
use rustycog::core::error::DomainError;
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{prelude::Resources, resources};

pub struct ResourceMapper;

impl ResourceMapper {
    /// Maps a persisted resource row to the domain entity.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the row cannot be mapped to [`Resource`].
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
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Loads every resource row.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the query fails or a row cannot be mapped to [`Resource`].
    pub async fn find_all_with_connection<C>(db: &C) -> Result<Vec<Resource>, DomainError>
    where
        C: ConnectionTrait,
    {
        let resources = Resources::find()
            .all(db)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in resources {
            result.push(ResourceMapper::to_domain(model)?);
        }
        Ok(result)
    }
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

    async fn find_by_type(&self, resource_type: &str) -> Result<Option<Resource>, DomainError> {
        debug!("Finding resource by type: {}", resource_type);

        let resource = Resources::find()
            .filter(resources::Column::ResourceType.eq(resource_type))
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
        Self::find_all_with_connection(self.db.as_ref()).await
    }
}

#[derive(Clone)]
pub struct ResourceRepositoryImpl {
    read_repo: Arc<dyn ResourceReadRepository>,
}

impl ResourceRepositoryImpl {
    pub fn new(read_repo: Arc<dyn ResourceReadRepository>) -> Self {
        Self { read_repo }
    }
}

#[async_trait]
impl ResourceReadRepository for ResourceRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Resource>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_type(&self, resource_type: &str) -> Result<Option<Resource>, DomainError> {
        self.read_repo.find_by_type(resource_type).await
    }

    async fn find_all(&self) -> Result<Vec<Resource>, DomainError> {
        self.read_repo.find_all().await
    }
}

#[async_trait]
impl ResourceRepository for ResourceRepositoryImpl {}
