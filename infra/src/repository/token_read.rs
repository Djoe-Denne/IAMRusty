use async_trait::async_trait;
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter,
};
use std::sync::Arc;
use uuid::Uuid;
use domain::entity::provider::{Provider, ProviderTokens};
use domain::port::repository::TokenReadRepository;
use tracing::debug;

use super::entity::{provider_token, prelude::ProviderToken};

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
    fn to_domain(model: provider_token::Model) -> ProviderTokens {
        ProviderTokens {
            access_token: model.access_token,
            refresh_token: model.refresh_token,
            expires_in: model.expires_in.map(|e| e as u64),
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
        
        let tokens = ProviderToken::find()
            .filter(provider_token::Column::UserId.eq(user_id))
            .filter(provider_token::Column::Provider.eq(provider.as_str()))
            .one(self.db.as_ref())
            .await?;

        Ok(tokens.map(Self::to_domain))
    }
} 