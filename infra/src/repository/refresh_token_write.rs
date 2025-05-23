use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
};
use std::sync::Arc;
use uuid::Uuid;
use domain::entity::token::RefreshToken as DomainRefreshToken;
use domain::port::repository::RefreshTokenWriteRepository;
use tracing::debug;

use super::entity::{refresh_token, prelude::RefreshToken};

/// SeaORM implementation of RefreshTokenWriteRepository
#[derive(Clone)]
pub struct RefreshTokenWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl RefreshTokenWriteRepositoryImpl {
    /// Create a new RefreshTokenWriteRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a domain refresh token to a database model
    fn to_model(token: &DomainRefreshToken) -> refresh_token::ActiveModel {
        refresh_token::ActiveModel {
            id: Set(token.id),
            user_id: Set(token.user_id),
            token: Set(token.token.clone()),
            is_valid: Set(token.is_valid),
            created_at: Set(token.created_at.into()),
            expires_at: Set(token.expires_at.into()),
        }
    }

    /// Convert a database model to a domain refresh token
    fn to_domain(model: refresh_token::Model) -> DomainRefreshToken {
        DomainRefreshToken {
            id: model.id,
            user_id: model.user_id,
            token: model.token,
            is_valid: model.is_valid,
            created_at: model.created_at.into(),
            expires_at: model.expires_at.into(),
        }
    }
}

#[async_trait]
impl RefreshTokenWriteRepository for RefreshTokenWriteRepositoryImpl {
    type Error = DbErr;

    async fn create(&self, token: DomainRefreshToken) -> Result<DomainRefreshToken, Self::Error> {
        debug!("Creating new refresh token for user ID: {}", token.user_id);
        
        let model = Self::to_model(&token);
        let res = model.insert(self.db.as_ref()).await?;
        
        Ok(Self::to_domain(res))
    }

    async fn update_validity(&self, token_id: Uuid, is_valid: bool) -> Result<(), Self::Error> {
        debug!("Updating refresh token validity: id={}, is_valid={}", token_id, is_valid);
        
        let token = RefreshToken::find_by_id(token_id)
            .one(self.db.as_ref())
            .await?;
            
        if let Some(token) = token {
            let mut model = refresh_token::ActiveModel::from(token);
            model.is_valid = Set(is_valid);
            
            model.update(self.db.as_ref()).await?;
            debug!("Updated refresh token validity");
        } else {
            debug!("Refresh token not found for update: {}", token_id);
        }
        
        Ok(())
    }

    async fn delete_by_user_id(&self, user_id: Uuid) -> Result<u64, Self::Error> {
        debug!("Deleting all refresh tokens for user ID: {}", user_id);
        
        let result = RefreshToken::delete_many()
            .filter(refresh_token::Column::UserId.eq(user_id))
            .exec(self.db.as_ref())
            .await?;
            
        debug!("Deleted {} refresh tokens", result.rows_affected);
        
        Ok(result.rows_affected)
    }
} 