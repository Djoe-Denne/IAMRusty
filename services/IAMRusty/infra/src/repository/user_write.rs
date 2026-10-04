use super::entity::users;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use iam_domain::entity::user::User as DomainUser;
use iam_domain::port::repository::UserWriteRepository;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr, Set, TransactionTrait};
use std::sync::Arc;
use tracing::debug;

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
    fn to_active_model(user: &DomainUser) -> users::ActiveModel {
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
        let model = Self::to_active_model(&user);

        let res = model.insert(self.db.as_ref()).await?;

        Ok(Self::to_domain(res))
    }

    async fn update(&self, user: DomainUser) -> Result<DomainUser, Self::Error> {
        debug!("Updating user with ID: {}", user.id);
        let tx = self.db.begin().await?;
        let existing = super::authentication_session::lock_user(&tx, user.id).await?;
        if user.password_hash.is_some() && user.password_hash != existing.password_hash {
            return Err(DbErr::Custom(
                "password mutations require the authentication session writer".into(),
            ));
        }

        let mut model = users::ActiveModel::from(existing);

        if user.username.is_some() {
            model.username = Set(user.username.clone());
        }
        if user.avatar_url.is_some() {
            model.avatar_url = Set(user.avatar_url.clone());
        }
        // Generic profile updates never mutate credentials or restore a replica snapshot.
        model.updated_at = Set(user.updated_at.naive_utc());

        let updated = model.update(&tx).await?;
        tx.commit().await?;

        Ok(Self::to_domain(updated))
    }
}
