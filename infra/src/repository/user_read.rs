use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, DbErr};
use std::sync::Arc;
use uuid::Uuid;
use domain::entity::{
    provider::Provider,
    user::User as DomainUser,
};
use domain::port::repository::UserReadRepository;
use tracing::debug;
use chrono::{DateTime, Utc};

use super::entity::{users, prelude::Users, provider_tokens, prelude::ProviderTokens as ProviderTokensEntity, user_emails, prelude::UserEmails};

/// SeaORM implementation of UserReadRepository
#[derive(Clone)]
pub struct UserReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl UserReadRepositoryImpl {
    /// Create a new UserReadRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a database model to a domain user
    fn to_domain(model: users::Model) -> DomainUser {
        DomainUser {
            id: model.id,
            username: model.username,
            password_hash: model.password_hash,
            avatar_url: model.avatar_url,
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(model.created_at, Utc),
            updated_at: DateTime::<Utc>::from_naive_utc_and_offset(model.updated_at, Utc),
        }
    }
}

#[async_trait]
impl UserReadRepository for UserReadRepositoryImpl {
    type Error = DbErr;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<DomainUser>, Self::Error> {
        debug!("Reading user by ID: {}", id);
        let user = Users::find_by_id(id)
            .one(self.db.as_ref())
            .await?;
        
        Ok(user.map(Self::to_domain))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<DomainUser>, Self::Error> {
        debug!("Reading user by email: {}", email);
        let user_email = UserEmails::find()
            .filter(user_emails::Column::Email.eq(email))
            .one(self.db.as_ref())
            .await?;
        
        if let Some(email_record) = user_email {
            self.find_by_id(email_record.user_id).await
        } else {
            Ok(None)
        }
    }

    async fn find_by_provider_user_id(
        &self,
        provider: Provider,
        provider_user_id: &str,
    ) -> Result<Option<DomainUser>, Self::Error> {
        debug!("Reading user by provider ID: {}_{}", provider.as_str(), provider_user_id);
        
        // Find user through provider_tokens table
        let provider_token = ProviderTokensEntity::find()
            .filter(provider_tokens::Column::Provider.eq(provider.as_str()))
            .filter(provider_tokens::Column::ProviderUserId.eq(provider_user_id))
            .one(self.db.as_ref())
            .await?;
        
        if let Some(token) = provider_token {
            self.find_by_id(token.user_id).await
        } else {
            Ok(None)
        }
    }
} 