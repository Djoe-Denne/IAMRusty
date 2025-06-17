use async_trait::async_trait;
use chrono::Utc;
use domain::entity::password_reset_token::PasswordResetToken;
use domain::port::repository::PasswordResetTokenWriteRepository;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use sea_orm::prelude::Expr;
use std::sync::Arc;
use uuid::Uuid;

use crate::repository::entity::{password_reset_tokens, prelude::*};

/// SeaORM implementation of PasswordResetTokenWriteRepository
pub struct PasswordResetTokenWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl PasswordResetTokenWriteRepositoryImpl {
    /// Create a new instance
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert domain entity to SeaORM active model for insertion
    fn to_active_model_insert(token: &PasswordResetToken) -> password_reset_tokens::ActiveModel {
        password_reset_tokens::ActiveModel {
            id: Set(token.id),
            user_id: Set(token.user_id),
            token_hash: Set(token.token_hash.clone()),
            expires_at: Set(token.expires_at),
            created_at: Set(token.created_at),
            used_at: Set(token.used_at),
        }
    }

    /// Convert domain entity to SeaORM active model for update
    fn to_active_model_update(token: &PasswordResetToken) -> password_reset_tokens::ActiveModel {
        password_reset_tokens::ActiveModel {
            id: Set(token.id),
            user_id: Set(token.user_id),
            token_hash: Set(token.token_hash.clone()),
            expires_at: Set(token.expires_at),
            created_at: Set(token.created_at),
            used_at: Set(token.used_at),
        }
    }
}

#[async_trait]
impl PasswordResetTokenWriteRepository for PasswordResetTokenWriteRepositoryImpl {
    type Error = sea_orm::DbErr;

    async fn create(&self, token: &PasswordResetToken) -> Result<(), Self::Error> {
        let active_model = Self::to_active_model_insert(token);
        active_model.insert(self.db.as_ref()).await?;
        Ok(())
    }

    async fn update(&self, token: &PasswordResetToken) -> Result<(), Self::Error> {
        let active_model = Self::to_active_model_update(token);
        active_model.update(self.db.as_ref()).await?;
        Ok(())
    }

    async fn mark_as_used(&self, token_id: Uuid) -> Result<(), Self::Error> {
        let now = Utc::now();
        
        PasswordResetTokens::update_many()
            .filter(password_reset_tokens::Column::Id.eq(token_id))
            .col_expr(password_reset_tokens::Column::UsedAt, Expr::value(Some(now)))
            .exec(self.db.as_ref())
            .await?;
        
        Ok(())
    }

    async fn delete_expired(&self) -> Result<u64, Self::Error> {
        let now = Utc::now();
        
        let result = PasswordResetTokens::delete_many()
            .filter(password_reset_tokens::Column::ExpiresAt.lt(now))
            .exec(self.db.as_ref())
            .await?;
        
        Ok(result.rows_affected)
    }

    async fn delete_all_for_user(&self, user_id: Uuid) -> Result<u64, Self::Error> {
        let result = PasswordResetTokens::delete_many()
            .filter(password_reset_tokens::Column::UserId.eq(user_id))
            .exec(self.db.as_ref())
            .await?;
        
        Ok(result.rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};

    #[tokio::test]
    async fn test_create() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let repo = PasswordResetTokenWriteRepositoryImpl::new(Arc::new(db));
        
        let token = PasswordResetToken::new(
            Uuid::new_v4(),
            "test_token",
            24, // 24 hours
        );
        
        let result = repo.create(&token).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mark_as_used() {
        let token_id = Uuid::new_v4();
        
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let repo = PasswordResetTokenWriteRepositoryImpl::new(Arc::new(db));
        let result = repo.mark_as_used(token_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_expired() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 1,
                rows_affected: 5, // 5 expired tokens deleted
            }])
            .into_connection();

        let repo = PasswordResetTokenWriteRepositoryImpl::new(Arc::new(db));
        let result = repo.delete_expired().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
    }
} 