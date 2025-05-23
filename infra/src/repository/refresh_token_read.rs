use async_trait::async_trait;
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter,
};
use std::sync::Arc;
use uuid::Uuid;
use domain::entity::token::RefreshToken as DomainRefreshToken;
use domain::port::repository::RefreshTokenReadRepository;
use tracing::debug;

use super::entity::{refresh_token, prelude::RefreshToken};

/// SeaORM implementation of RefreshTokenReadRepository
#[derive(Clone)]
pub struct RefreshTokenReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl RefreshTokenReadRepositoryImpl {
    /// Create a new RefreshTokenReadRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
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
impl RefreshTokenReadRepository for RefreshTokenReadRepositoryImpl {
    type Error = DbErr;

    async fn find_by_token(&self, token: &str) -> Result<Option<DomainRefreshToken>, Self::Error> {
        debug!("Looking up refresh token");
        
        let refresh_token = RefreshToken::find()
            .filter(refresh_token::Column::Token.eq(token))
            .one(self.db.as_ref())
            .await?;

        Ok(refresh_token.map(Self::to_domain))
    }
    
    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<DomainRefreshToken>, Self::Error> {
        debug!("Finding refresh tokens for user ID: {}", user_id);
        
        let tokens = RefreshToken::find()
            .filter(refresh_token::Column::UserId.eq(user_id))
            .all(self.db.as_ref())
            .await?;
            
        Ok(tokens.into_iter().map(Self::to_domain).collect())
    }
} 