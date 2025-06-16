use async_trait::async_trait;
use chrono::Utc;
use domain::entity::provider::{Provider, ProviderTokens};
use domain::entity::provider_link::ProviderLink;
use domain::port::repository::TokenReadRepository;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{prelude::ProviderTokens as ProviderTokensEntity, provider_tokens};

/// SeaORM implementation of TokenReadRepository
#[derive(Clone)]
pub struct TokenReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl TokenReadRepositoryImpl {
    /// Create a new TokenReadRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a database model to domain ProviderTokens
    fn to_domain(model: provider_tokens::Model) -> ProviderTokens {
        ProviderTokens {
            access_token: model.access_token,
            refresh_token: model.refresh_token,
            expires_in: model.expires_in.map(|e| e as u64),
        }
    }

    /// Convert a database model to domain ProviderLink
    fn to_provider_link(model: provider_tokens::Model) -> ProviderLink {
        ProviderLink {
            user_id: model.user_id,
            provider: Provider::from_str(&model.provider).unwrap_or(Provider::GitHub),
            provider_user_id: model.provider_user_id,
            linked_at: chrono::DateTime::<Utc>::from_naive_utc_and_offset(model.created_at, Utc),
        }
    }
}

#[async_trait]
impl TokenReadRepository for TokenReadRepositoryImpl {
    type Error = DbErr;

    async fn get_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<Option<ProviderTokens>, Self::Error> {
        debug!(user_id = %user_id, provider = %provider.as_str(), "Reading provider tokens");

        let tokens = ProviderTokensEntity::find()
            .filter(provider_tokens::Column::UserId.eq(user_id))
            .filter(provider_tokens::Column::Provider.eq(provider.as_str()))
            .one(self.db.as_ref())
            .await?;

        Ok(tokens.map(Self::to_domain))
    }

    async fn get_provider_link(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<Option<ProviderLink>, Self::Error> {
        debug!(user_id = %user_id, provider = %provider.as_str(), "Reading provider link");

        let token = ProviderTokensEntity::find()
            .filter(provider_tokens::Column::UserId.eq(user_id))
            .filter(provider_tokens::Column::Provider.eq(provider.as_str()))
            .one(self.db.as_ref())
            .await?;

        Ok(token.map(Self::to_provider_link))
    }

    async fn get_user_provider_links(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ProviderLink>, Self::Error> {
        debug!(user_id = %user_id, "Reading all provider links for user");

        let tokens = ProviderTokensEntity::find()
            .filter(provider_tokens::Column::UserId.eq(user_id))
            .all(self.db.as_ref())
            .await?;

        Ok(tokens.into_iter().map(Self::to_provider_link).collect())
    }
}
