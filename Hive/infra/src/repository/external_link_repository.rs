//! ExternalLinkRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::entity::{ExternalLink, SyncStatus};
use hive_domain::error::DomainError;
use hive_domain::port::repository::ExternalLinkRepository;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, 
    QueryFilter, Set
};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{
    prelude::ExternalLinks,
    external_links,
};

/// SeaORM implementation of ExternalLinkRepository
#[derive(Clone)]
pub struct ExternalLinkRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ExternalLinkRepositoryImpl {
    /// Create a new ExternalLinkRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a database model to a domain external link
    fn to_domain(model: external_links::Model) -> Result<ExternalLink, DomainError> {
        let last_sync_status = match model.last_sync_status {
            Some(status_str) => Some(SyncStatus::from_str(&status_str)?),
            None => None,
        };

        Ok(ExternalLink {
            id: model.id,
            organization_id: model.organization_id,
            provider_id: model.provider_id,
            provider_config: model.provider_config,
            sync_enabled: model.sync_enabled,
            sync_settings: model.sync_settings,
            last_sync_at: model.last_sync_at,
            last_sync_status,
            sync_error: model.sync_error,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }

    /// Convert a domain external link to a database active model
    fn to_active_model(link: &ExternalLink) -> external_links::ActiveModel {
        let last_sync_status_str = link.last_sync_status.as_ref()
            .map(|s| s.as_str().to_string());

        external_links::ActiveModel {
            id: ActiveValue::Set(link.id),
            organization_id: ActiveValue::Set(link.organization_id),
            provider_id: ActiveValue::Set(link.provider_id),
            provider_config: ActiveValue::Set(link.provider_config.clone()),
            sync_enabled: ActiveValue::Set(link.sync_enabled),
            sync_settings: ActiveValue::Set(link.sync_settings.clone()),
            last_sync_at: ActiveValue::Set(link.last_sync_at),
            last_sync_status: ActiveValue::Set(last_sync_status_str),
            sync_error: ActiveValue::Set(link.sync_error.clone()),
            created_at: ActiveValue::Set(link.created_at),
            updated_at: ActiveValue::Set(link.updated_at),
        }
    }
}

#[async_trait]
impl ExternalLinkRepository for ExternalLinkRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ExternalLink>, DomainError> {
        debug!("Finding external link by ID: {}", id);
        
        let link = ExternalLinks::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        match link {
            Some(model) => Ok(Some(Self::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_organization(&self, organization_id: &Uuid) -> Result<Vec<ExternalLink>, DomainError> {
        debug!("Finding external links by organization: {}", organization_id);
        
        let links = ExternalLinks::find()
            .filter(external_links::Column::OrganizationId.eq(*organization_id))
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in links {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_by_organization_and_provider(
        &self,
        organization_id: &Uuid,
        provider_id: &Uuid,
    ) -> Result<Option<ExternalLink>, DomainError> {
        debug!("Finding external link by org {} and provider {}", organization_id, provider_id);
        
        let link = ExternalLinks::find()
            .filter(external_links::Column::OrganizationId.eq(*organization_id))
            .filter(external_links::Column::ProviderId.eq(*provider_id))
            .one(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        match link {
            Some(model) => Ok(Some(Self::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_sync_enabled(&self) -> Result<Vec<ExternalLink>, DomainError> {
        debug!("Finding sync enabled external links");
        
        let links = ExternalLinks::find()
            .filter(external_links::Column::SyncEnabled.eq(true))
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in links {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_needing_sync(&self, max_age_hours: i64) -> Result<Vec<ExternalLink>, DomainError> {
        debug!("Finding external links needing sync (max age: {} hours)", max_age_hours);
        
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        let links = ExternalLinks::find()
            .filter(external_links::Column::SyncEnabled.eq(true))
            .filter(
                external_links::Column::LastSyncAt.is_null()
                    .or(external_links::Column::LastSyncAt.lt(cutoff))
            )
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let mut result = Vec::new();
        for model in links {
            result.push(Self::to_domain(model)?);
        }
        Ok(result)
    }

    async fn save(&self, link: &ExternalLink) -> Result<ExternalLink, DomainError> {
        debug!("Saving external link with ID: {}", link.id);
        
        let active_model = Self::to_active_model(link);
        
        let result = active_model
            .save(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        let saved_model = external_links::Model {
            id: result.id.unwrap(),
            organization_id: result.organization_id.unwrap(),
            provider_id: result.provider_id.unwrap(),
            provider_config: result.provider_config.unwrap(),
            sync_enabled: result.sync_enabled.unwrap(),
            sync_settings: result.sync_settings.unwrap(),
            last_sync_at: result.last_sync_at.unwrap(),
            last_sync_status: result.last_sync_status.unwrap(),
            sync_error: result.sync_error.unwrap(),
            created_at: result.created_at.unwrap(),
            updated_at: result.updated_at.unwrap(),
        };

        Self::to_domain(saved_model)
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting external link by ID: {}", id);
        
        let result = ExternalLinks::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found("ExternalLink", &id.to_string()));
        }

        Ok(())
    }

    async fn count_by_organization(&self, organization_id: &Uuid) -> Result<i64, DomainError> {
        debug!("Counting external links in organization: {}", organization_id);
        
        let count = ExternalLinks::find()
            .filter(external_links::Column::OrganizationId.eq(*organization_id))
            .count(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;

        Ok(count as i64)
    }
} 