use super::entity::{prelude::Users, users};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use iam_domain::entity::user::User as DomainUser;
use iam_domain::port::repository::UserWriteRepository;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr, EntityTrait, Set};
use std::sync::Arc;
use tracing::{debug, error};

/// `SeaORM` implementation of `UserWriteRepository`
#[derive(Clone)]
pub struct UserWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl UserWriteRepositoryImpl {
    /// Create a new `UserWriteRepositoryImpl`
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a domain user to a database model
    fn to_active_model(&self, user: &DomainUser) -> users::ActiveModel {
        users::ActiveModel {
            id: ActiveValue::Set(user.id),
            username: ActiveValue::Set(user.username.clone()),
            password_hash: ActiveValue::Set(user.password_hash.clone()),
            avatar_url: ActiveValue::Set(user.avatar_url.clone()),
            created_at: ActiveValue::Set(user.created_at.naive_utc()),
            updated_at: ActiveValue::Set(user.updated_at.naive_utc()),
        }
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
impl UserWriteRepository for UserWriteRepositoryImpl {
    type Error = DbErr;

    async fn create(&self, user: DomainUser) -> Result<DomainUser, Self::Error> {
        debug!("Creating new user with ID: {}", user.id);
        let model = self.to_active_model(&user);

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

        if user.username.is_some() {
            model.username = Set(user.username.clone());
        }
        if user.avatar_url.is_some() {
            model.avatar_url = Set(user.avatar_url.clone());
        }
        if user.password_hash.is_some() {
            model.password_hash = Set(user.password_hash.clone());
        }
        model.updated_at = Set(user.updated_at.naive_utc());

        let updated = model.update(self.db.as_ref()).await?;

        Ok(Self::to_domain(updated))
    }
}
