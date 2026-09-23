use async_trait::async_trait;
use chrono::Utc;
use iam_domain::entity::provider::{Provider, ProviderTokens};
use iam_domain::entity::provider_link::ProviderLink;
use iam_domain::port::repository::{TokenReadRepository, TokenWriteRepository};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, Set,
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
        provider: &Provider,
        provider_user_id: String,
        tokens: &ProviderTokens,
    ) -> provider_tokens::ActiveModel {
        provider_tokens::ActiveModel {
            id: ActiveValue::default(), // Auto-generated
            user_id: Set(user_id),
            provider: Set(provider.as_str().to_string()),
            provider_user_id: Set(provider_user_id),
            access_token: Set(tokens.access_token.clone()),
            refresh_token: Set(tokens.refresh_token.clone()),
            expires_in: Set(tokens.expires_in.and_then(|e| i32::try_from(e).ok())),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
        }
    }

    /// Convert a database model to domain `ProviderTokens`
    fn to_domain(model: provider_tokens::Model) -> ProviderTokens {
        ProviderTokens {
            access_token: model.access_token,
            refresh_token: model.refresh_token,
            expires_in: model.expires_in.and_then(|e| u64::try_from(e).ok()),
        }
    }
}

#[async_trait]
impl TokenReadRepository for TokenRepositoryImpl {
    type Error = DbErr;

    async fn get_provider_tokens(
        &self,
        user_id: Uuid,
        provider: &Provider,
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
        provider: &Provider,
    ) -> Result<Option<ProviderLink>, Self::Error> {
        let token = ProviderTokensEntity::find()
            .filter(provider_tokens::Column::UserId.eq(user_id))
            .filter(provider_tokens::Column::Provider.eq(provider.as_str()))
            .one(&self.db)
            .await?;

        Ok(token
            .map(super::provider_link_map::to_provider_link)
            .transpose()?)
    }

    async fn get_user_provider_links(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ProviderLink>, Self::Error> {
        let tokens = ProviderTokensEntity::find()
            .filter(provider_tokens::Column::UserId.eq(user_id))
            .all(&self.db)
            .await?;

        tokens
            .into_iter()
            .map(super::provider_link_map::to_provider_link)
            .collect::<Result<Vec<_>, _>>()
    }
}

#[async_trait]
impl TokenWriteRepository for TokenRepositoryImpl {
    type Error = DbErr;

    async fn save_provider_tokens(
        &self,
        user_id: Uuid,
        provider: &Provider,
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
            model.expires_in = Set(tokens.expires_in.and_then(|e| i32::try_from(e).ok()));
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
        provider: &Provider,
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

#[cfg(test)]
mod provider_link_mapping_tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn model(provider: &str) -> provider_tokens::Model {
        provider_tokens::Model {
            id: 1,
            user_id: Uuid::nil(),
            provider: provider.to_string(),
            access_token: "tok".to_string(),
            refresh_token: None,
            expires_in: None,
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
            provider_user_id: "u1".to_string(),
        }
    }

    #[test]
    fn gitlab_row_is_not_github() {
        let link = crate::repository::provider_link_map::to_provider_link(model("gitlab"))
            .expect("gitlab slug");
        assert_eq!(link.provider.as_str(), "gitlab");
        let github = Provider::parse_slug("github").expect("github");
        assert_ne!(link.provider, github);
    }

    #[test]
    fn bitbucket_row_is_not_github() {
        let link = crate::repository::provider_link_map::to_provider_link(model("bitbucket"))
            .expect("bitbucket slug");
        assert_eq!(link.provider.as_str(), "bitbucket");
        let github = Provider::parse_slug("github").expect("github");
        assert_ne!(link.provider, github);
    }

    #[test]
    fn illegal_row_is_mapping_error() {
        let err = crate::repository::provider_link_map::to_provider_link(model("not-github"))
            .expect_err("illegal slug");
        assert!(err.to_string().contains("invalid provider slug"));
    }
}
