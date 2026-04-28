use async_trait::async_trait;
use chrono::Utc;
use iam_domain::entity::provider::{Provider, ProviderTokens};
use iam_domain::entity::provider_link::ProviderLink;
use iam_domain::port::repository::{TokenReadRepository, TokenWriteRepository};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
};
use tracing::debug;
use uuid::Uuid;

use super::entity::{prelude::ProviderTokens as ProviderTokensEntity, provider_tokens};

/// `SeaORM` implementation of `TokenRepository`
pub struct TokenRepositoryImpl {
    db: DatabaseConnection,
}

impl TokenRepositoryImpl {
    /// Create a new `TokenRepositoryImpl`
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Convert domain `ProviderTokens` to a database model
    fn to_model(
        user_id: Uuid,
        provider: Provider,
        provider_user_id: String,
        tokens: &ProviderTokens,
    ) -> provider_tokens::ActiveModel {
        provider_tokens::ActiveModel {
            id: Default::default(), // Auto-generated
            user_id: Set(user_id),
            provider: Set(provider.as_str().to_string()),
            provider_user_id: Set(provider_user_id),
            access_token: Set(tokens.access_token.clone()),
            refresh_token: Set(tokens.refresh_token.clone()),
            expires_in: Set(tokens.expires_in.map(|e| e as i32)),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
        }
    }

    /// Convert a database model to domain `ProviderTokens`
    fn to_domain(model: provider_tokens::Model) -> ProviderTokens {
        ProviderTokens {
            access_token: model.access_token,
            refresh_token: model.refresh_token,
            expires_in: model.expires_in.map(|e| e as u64),
        }
    }

    /// Convert a database model to domain `ProviderLink`
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
impl TokenReadRepository for TokenRepositoryImpl {
    type Error = DbErr;

    async fn get_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<Option<ProviderTokens>, Self::Error> {
        let tokens = ProviderTokensEntity::find()
            .filter(provider_tokens::Column::UserId.eq(user_id))
            .filter(provider_tokens::Column::Provider.eq(provider.as_str()))
            .one(&self.db)
            .await?;

        Ok(tokens.map(Self::to_domain))
    }

    async fn get_provider_link(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<Option<ProviderLink>, Self::Error> {
        let token = ProviderTokensEntity::find()
            .filter(provider_tokens::Column::UserId.eq(user_id))
            .filter(provider_tokens::Column::Provider.eq(provider.as_str()))
            .one(&self.db)
            .await?;

        Ok(token.map(Self::to_provider_link))
    }

    async fn get_user_provider_links(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ProviderLink>, Self::Error> {
        let tokens = ProviderTokensEntity::find()
            .filter(provider_tokens::Column::UserId.eq(user_id))
            .all(&self.db)
            .await?;

        Ok(tokens.into_iter().map(Self::to_provider_link).collect())
    }
}

#[async_trait]
impl TokenWriteRepository for TokenRepositoryImpl {
    type Error = DbErr;

    async fn save_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
        provider_user_id: String,
        tokens: ProviderTokens,
    ) -> Result<(), Self::Error> {
        // Check if tokens already exist for this user and provider
        let existing = ProviderTokensEntity::find()
            .filter(provider_tokens::Column::UserId.eq(user_id))
            .filter(provider_tokens::Column::Provider.eq(provider.as_str()))
            .one(&self.db)
            .await?;

        if let Some(existing) = existing {
            // Update existing tokens
            let mut model = provider_tokens::ActiveModel::from(existing);
            model.provider_user_id = Set(provider_user_id);
            model.access_token = Set(tokens.access_token.clone());
            model.refresh_token = Set(tokens.refresh_token.clone());
            model.expires_in = Set(tokens.expires_in.map(|e| e as i32));
            model.updated_at = Set(Utc::now().naive_utc());

            model.update(&self.db).await?;

            debug!(user_id = %user_id, provider = %provider.as_str(), "Updated provider tokens");
        } else {
            // Insert new tokens
            let model = Self::to_model(user_id, provider, provider_user_id, &tokens);
            model.insert(&self.db).await?;

            debug!(user_id = %user_id, provider = %provider.as_str(), "Saved new provider tokens");
        }

        Ok(())
    }

    async fn delete_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<(), Self::Error> {
        let result = ProviderTokensEntity::delete_many()
            .filter(provider_tokens::Column::UserId.eq(user_id))
            .filter(provider_tokens::Column::Provider.eq(provider.as_str()))
            .exec(&self.db)
            .await?;

        debug!(
            user_id = %user_id,
            provider = %provider.as_str(),
            rows_affected = result.rows_affected,
            "Deleted provider tokens"
        );

        Ok(())
    }
}
