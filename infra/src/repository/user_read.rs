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

use super::entity::{users, prelude::Users};

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
            provider_user_id: model.provider_user_id,
            username: model.username,
            email: model.email,
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

    async fn find_by_provider_user_id(
        &self,
        provider: Provider,
        provider_user_id: &str,
    ) -> Result<Option<DomainUser>, Self::Error> {
        let full_id = format!("{}_{}", provider.as_str(), provider_user_id);
        debug!("Reading user by provider ID: {}", full_id);
        
        let user = Users::find()
            .filter(users::Column::ProviderUserId.eq(full_id))
            .one(self.db.as_ref())
            .await?;
        
        Ok(user.map(Self::to_domain))
    }
} 