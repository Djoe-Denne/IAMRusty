//! ExternalProviderRepository SeaORM implementation

use async_trait::async_trait;
use hive_domain::entity::{ExternalProvider, ProviderType};
use hive_domain::error::DomainError;
use hive_domain::port::repository::ExternalProviderRepository;
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

/// SeaORM implementation of ExternalProviderRepository
#[derive(Clone)]
pub struct ExternalProviderRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ExternalProviderRepositoryImpl {
    /// Create a new ExternalProviderRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a database model to a domain external provider
    fn to_domain(model: external_providers::Model) -> Result<ExternalProvider, DomainError> {
        let provider_type = ProviderType::from_str(&model.provider_type)?;

        Ok(ExternalProvider {
            id: model.id,
            provider_type,
            name: model.name,
            config_schema: model.config_schema,
            is_active: model.is_active,
            created_at: model.created_at,
        })
    }

    /// Convert a domain external provider to a database active model
    fn to_active_model(provider: &ExternalProvider) -> external_providers::ActiveModel {
        external_providers::ActiveModel {
            id: ActiveValue::Set(provider.id),
            provider_type: ActiveValue::Set(provider.provider_type.as_str().to_string()),
            name: ActiveValue::Set(provider.name.clone()),
            config_schema: ActiveValue::Set(provider.config_schema.clone()),
            is_active: ActiveValue::Set(provider.is_active),
            created_at: ActiveValue::Set(provider.created_at),
        }
    }
}

#[async_trait]
impl ExternalProviderRepository for ExternalProviderRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ExternalProvider>, DomainError> {
        debug!("Finding external provider by ID: {}", id);
        
        let provider = ExternalProviders::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match provider {
            Some(model) => Ok(Some(Self::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_type(&self, provider_type: &ProviderType) -> Result<Option<ExternalProvider>, DomainError> {
        debug!("Finding external provider by type: {:?}", provider_type);
        
        let provider = ExternalProviders::find()
            .filter(external_providers::Column::ProviderType.eq(provider_type.as_str()))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match provider {
            Some(model) => Ok(Some(Self::to_domain(model)?)),
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
            result.push(Self::to_domain(model)?);
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
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn save(&self, provider: &ExternalProvider) -> Result<ExternalProvider, DomainError> {
        debug!("Saving external provider with ID: {}", provider.id);
        
        let active_model = Self::to_active_model(provider);
        
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

        Self::to_domain(saved_model)
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