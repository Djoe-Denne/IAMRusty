use async_trait::async_trait;
use iam_domain::entity::provider::{Provider, ProviderTokens};
use iam_domain::entity::provider_link::ProviderLink;
use iam_domain::port::repository::TokenReadRepository;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{prelude::ProviderTokens as ProviderTokensEntity, provider_tokens};

/// `SeaORM` implementation of `TokenReadRepository`
#[derive(Clone)]
pub struct TokenReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl TokenReadRepositoryImpl {
    /// Create a new `TokenReadRepositoryImpl`
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
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
impl TokenReadRepository for TokenReadRepositoryImpl {
    type Error = DbErr;

    async fn get_provider_tokens(
        &self,
        user_id: Uuid,
        provider: &Provider,
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
        provider: &Provider,
    ) -> Result<Option<ProviderLink>, Self::Error> {
        debug!(user_id = %user_id, provider = %provider.as_str(), "Reading provider link");

        let token = ProviderTokensEntity::find()
            .filter(provider_tokens::Column::UserId.eq(user_id))
            .filter(provider_tokens::Column::Provider.eq(provider.as_str()))
            .one(self.db.as_ref())
            .await?;

        Ok(token
            .map(super::provider_link_map::to_provider_link)
            .transpose()?)
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

        tokens
            .into_iter()
            .map(super::provider_link_map::to_provider_link)
            .collect::<Result<Vec<_>, _>>()
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
