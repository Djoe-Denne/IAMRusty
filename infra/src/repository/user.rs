use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, Set, ActiveModelTrait, QueryFilter, ColumnTrait, DbErr};
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDateTime};
use domain::entity::{
    provider::Provider,
    user::User as DomainUser,
};
use domain::port::repository::{UserReadRepository, UserWriteRepository};
use tracing::error;

use super::entity::{users, prelude::Users};

/// SeaORM implementation of UserRepository
pub struct UserRepositoryImpl {
    db: DatabaseConnection,
}

impl UserRepositoryImpl {
    /// Create a new UserRepositoryImpl
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Convert a domain user to a database model
    fn to_model(user: &DomainUser) -> users::ActiveModel {
        users::ActiveModel {
            id: Set(user.id),
            provider_user_id: Set(user.provider_user_id.clone()),
            username: Set(user.username.clone()),
            email: Set(user.email.clone()),
            avatar_url: Set(user.avatar_url.clone()),
            created_at: Set(user.created_at.naive_utc()),
            updated_at: Set(user.updated_at.naive_utc()),
        }
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
impl UserReadRepository for UserRepositoryImpl {
    type Error = DbErr;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<DomainUser>, Self::Error> {
        let user = Users::find_by_id(id)
            .one(&self.db)
            .await?;
        
        Ok(user.map(Self::to_domain))
    }

    async fn find_by_provider_user_id(
        &self,
        provider: Provider,
        provider_user_id: &str,
    ) -> Result<Option<DomainUser>, Self::Error> {
        let full_id = format!("{}_{}", provider.as_str(), provider_user_id);
        
        let user = Users::find()
            .filter(users::Column::ProviderUserId.eq(full_id))
            .one(&self.db)
            .await?;
        
        Ok(user.map(Self::to_domain))
    }
}

#[async_trait]
impl UserWriteRepository for UserRepositoryImpl {
    type Error = DbErr;

    async fn create(&self, user: DomainUser) -> Result<DomainUser, Self::Error> {
        let model = Self::to_model(&user);
        
        let res = model.insert(&self.db).await?;
        
        Ok(Self::to_domain(res))
    }

    async fn update(&self, user: DomainUser) -> Result<DomainUser, Self::Error> {
        let existing = Users::find_by_id(user.id)
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                error!("User not found for update: {}", user.id);
                DbErr::RecordNotFound("User not found".to_string())
            })?;
        
        let mut model = users::ActiveModel::from(existing);
        
        model.username = Set(user.username.clone());
        model.email = Set(user.email.clone());
        model.avatar_url = Set(user.avatar_url.clone());
        model.updated_at = Set(user.updated_at.naive_utc());
        
        let updated = model.update(&self.db).await?;
        
        Ok(Self::to_domain(updated))
    }
} 