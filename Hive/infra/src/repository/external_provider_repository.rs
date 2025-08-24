//! ExternalProviderRepository SeaORM implementation

use async_trait::async_trait;
use hive_domain::entity::ExternalProvider;
use rustycog_core::error::DomainError;
use hive_domain::port::repository::{
    ExternalProviderReadRepository, ExternalProviderRepository, ExternalProviderWriteRepository,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, 
    QueryFilter, Set
};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{
    prelude::ExternalProviders,
    external_providers,
};

pub struct ExternalProviderMapper;

impl ExternalProviderMapper {
    pub fn to_domain(model: external_providers::Model) -> Result<ExternalProvider, DomainError> {

        Ok(ExternalProvider {
            id: model.id,
            provider_source: model.provider_type,
            name: model.name,
            config_schema: model.config_schema,
            is_active: model.is_active,
            created_at: model.created_at,
        })
    }

    pub fn to_active_model(provider: &ExternalProvider) -> external_providers::ActiveModel {
        external_providers::ActiveModel {
            id: ActiveValue::Set(provider.id),
            provider_type: ActiveValue::Set(provider.provider_source.clone()),
            name: ActiveValue::Set(provider.name.clone()),
            config_schema: ActiveValue::Set(provider.config_schema.clone()),
            is_active: ActiveValue::Set(provider.is_active),
            created_at: ActiveValue::Set(provider.created_at),
        }
    }
}

/// Read repository
#[derive(Clone)]
pub struct ExternalProviderReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ExternalProviderReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self { Self { db } }
}

#[async_trait]
impl ExternalProviderReadRepository for ExternalProviderReadRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ExternalProvider>, DomainError> {
        debug!("Finding external provider by ID: {}", id);
        
        let provider = ExternalProviders::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match provider {
            Some(model) => Ok(Some(ExternalProviderMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_source(&self, provider_source: &String) -> Result<Option<ExternalProvider>, DomainError> {
        debug!("Finding external provider by source: {:?}", provider_source);
        
        let provider = ExternalProviders::find()
            .filter(external_providers::Column::ProviderType.eq(provider_source.clone()))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match provider {
            Some(model) => Ok(Some(ExternalProviderMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<ExternalProvider>, DomainError> {
        debug!("Finding all external providers");
        
        let providers = ExternalProviders::find()
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in providers {
            result.push(ExternalProviderMapper::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_active(&self) -> Result<Vec<ExternalProvider>, DomainError> {
        debug!("Finding active external providers");
        
        let providers = ExternalProviders::find()
            .filter(external_providers::Column::IsActive.eq(true))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in providers {
            result.push(ExternalProviderMapper::to_domain(model)?);
        }
        Ok(result)
    }

}

/// Write repository
#[derive(Clone)]
pub struct ExternalProviderWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ExternalProviderWriteRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self { Self { db } }
}

#[async_trait]
impl ExternalProviderWriteRepository for ExternalProviderWriteRepositoryImpl {
    async fn save(&self, provider: &ExternalProvider) -> Result<ExternalProvider, DomainError> {
        debug!("Saving external provider with ID: {}", provider.id);
        
        let active_model = ExternalProviderMapper::to_active_model(provider);
        
        let result = active_model
            .save(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let saved_model = external_providers::Model {
            id: result.id.unwrap(),
            provider_type: result.provider_type.unwrap(),
            name: result.name.unwrap(),
            config_schema: result.config_schema.unwrap(),
            is_active: result.is_active.unwrap(),
            created_at: result.created_at.unwrap(),
        };

        ExternalProviderMapper::to_domain(saved_model)
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting external provider by ID: {}", id);
        
        let result = ExternalProviders::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found("ExternalProvider", &id.to_string()));
        }

        Ok(())
    }
}

/// Combined delegator
#[derive(Clone)]
pub struct ExternalProviderRepositoryImpl {
    read_repo: Arc<dyn ExternalProviderReadRepository>,
    write_repo: Arc<dyn ExternalProviderWriteRepository>,
}

impl ExternalProviderRepositoryImpl {
    pub fn new(
        read_repo: Arc<dyn ExternalProviderReadRepository>,
        write_repo: Arc<dyn ExternalProviderWriteRepository>,
    ) -> Self {
        Self { read_repo, write_repo }
    }
}

#[async_trait]
impl ExternalProviderReadRepository for ExternalProviderRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ExternalProvider>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_source(&self, provider_source: &String) -> Result<Option<ExternalProvider>, DomainError> {
        self.read_repo.find_by_source(provider_source).await
    }

    async fn find_all(&self) -> Result<Vec<ExternalProvider>, DomainError> {
        self.read_repo.find_all().await
    }

    async fn find_active(&self) -> Result<Vec<ExternalProvider>, DomainError> {
        self.read_repo.find_active().await
    }
}

#[async_trait]
impl ExternalProviderWriteRepository for ExternalProviderRepositoryImpl {
    async fn save(&self, provider: &ExternalProvider) -> Result<ExternalProvider, DomainError> {
        self.write_repo.save(provider).await
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.delete_by_id(id).await
    }
}

impl ExternalProviderRepository for ExternalProviderRepositoryImpl {}