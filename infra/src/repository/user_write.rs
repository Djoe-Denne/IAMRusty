use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, Set, ActiveModelTrait, DbErr};
use std::sync::Arc;
use domain::entity::user::User as DomainUser;
use domain::port::repository::UserWriteRepository;
use tracing::{debug, error};
use chrono::{DateTime, Utc};

use super::entity::{users, prelude::Users};

/// SeaORM implementation of UserWriteRepository
#[derive(Clone)]
pub struct UserWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl UserWriteRepositoryImpl {
    /// Create a new UserWriteRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
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
impl UserWriteRepository for UserWriteRepositoryImpl {
    type Error = DbErr;

    async fn create(&self, user: DomainUser) -> Result<DomainUser, Self::Error> {
        debug!("Creating new user with ID: {}", user.id);
        let model = Self::to_model(&user);
        
        let res = model.insert(self.db.as_ref()).await?;
        
        Ok(Self::to_domain(res))
    }

    async fn update(&self, user: DomainUser) -> Result<DomainUser, Self::Error> {
        debug!("Updating user with ID: {}", user.id);
        let existing = Users::find_by_id(user.id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| {
                error!(user_id = %user.id, "Failed to update user: User not found");
                DbErr::RecordNotFound("User not found".to_string())
            })?;
        
        let mut model = users::ActiveModel::from(existing);
        
        model.username = Set(user.username.clone());
        model.email = Set(user.email.clone());
        model.avatar_url = Set(user.avatar_url.clone());
        model.updated_at = Set(user.updated_at.naive_utc());
        
        let updated = model.update(self.db.as_ref()).await?;
        
        Ok(Self::to_domain(updated))
    }
} 