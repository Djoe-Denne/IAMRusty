use async_trait::async_trait;
use chrono::Utc;
use iam_domain::entity::password_reset_token::PasswordResetToken;
use iam_domain::port::repository::PasswordResetTokenReadRepository;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::repository::entity::{password_reset_tokens, prelude::*};

/// SeaORM implementation of PasswordResetTokenReadRepository
pub struct PasswordResetTokenReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl PasswordResetTokenReadRepositoryImpl {
    /// Create a new instance
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert SeaORM model to domain entity
    fn to_domain(model: password_reset_tokens::Model) -> PasswordResetToken {
        PasswordResetToken {
            id: model.id,
            user_id: model.user_id,
            token_hash: model.token_hash,
            expires_at: model.expires_at,
            created_at: model.created_at,
            used_at: model.used_at,
        }
    }
}

#[async_trait]
impl PasswordResetTokenReadRepository for PasswordResetTokenReadRepositoryImpl {
    type Error = sea_orm::DbErr;

    async fn find_by_user_and_token_hash(
        &self,
        user_id: Uuid,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, Self::Error> {
        let result = PasswordResetTokens::find()
            .filter(password_reset_tokens::Column::UserId.eq(user_id))
            .filter(password_reset_tokens::Column::TokenHash.eq(token_hash))
            .one(self.db.as_ref())
            .await?;

        Ok(result.map(Self::to_domain))
    }

    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, Self::Error> {
        let result = PasswordResetTokens::find()
            .filter(password_reset_tokens::Column::TokenHash.eq(token_hash))
            .one(self.db.as_ref())
            .await?;

        Ok(result.map(Self::to_domain))
    }

    async fn find_latest_valid_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<PasswordResetToken>, Self::Error> {
        let now = Utc::now();

        let result = PasswordResetTokens::find()
            .filter(password_reset_tokens::Column::UserId.eq(user_id))
            .filter(password_reset_tokens::Column::ExpiresAt.gt(now))
            .filter(password_reset_tokens::Column::UsedAt.is_null())
            .order_by_desc(password_reset_tokens::Column::CreatedAt)
            .one(self.db.as_ref())
            .await?;

        Ok(result.map(Self::to_domain))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PasswordResetToken>, Self::Error> {
        let result = PasswordResetTokens::find_by_id(id)
            .one(self.db.as_ref())
            .await?;

        Ok(result.map(Self::to_domain))
    }

    async fn count_valid_for_user(&self, user_id: Uuid) -> Result<u64, Self::Error> {
        let now = Utc::now();

        let count = PasswordResetTokens::find()
            .filter(password_reset_tokens::Column::UserId.eq(user_id))
            .filter(password_reset_tokens::Column::ExpiresAt.gt(now))
            .filter(password_reset_tokens::Column::UsedAt.is_null())
            .count(self.db.as_ref())
            .await?;

        Ok(count)
    }
}
