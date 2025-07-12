use async_trait::async_trait;
use chrono::{DateTime, Utc};
use iam_domain::entity::user_email::UserEmail as DomainUserEmail;
use iam_domain::port::repository::UserEmailWriteRepository;
use sea_orm::prelude::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{prelude::UserEmails, user_emails};

/// SeaORM implementation of UserEmailWriteRepository
#[derive(Clone)]
pub struct UserEmailWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl UserEmailWriteRepositoryImpl {
    /// Create a new UserEmailWriteRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a domain user email to a database model
    fn to_model(user_email: &DomainUserEmail) -> user_emails::ActiveModel {
        user_emails::ActiveModel {
            id: Set(user_email.id),
            user_id: Set(user_email.user_id),
            email: Set(user_email.email.clone()),
            is_primary: Set(user_email.is_primary),
            is_verified: Set(user_email.is_verified),
            created_at: Set(user_email.created_at.naive_utc()),
            updated_at: Set(user_email.updated_at.naive_utc()),
        }
    }

    /// Convert a database model to a domain user email
    fn to_domain(model: user_emails::Model) -> DomainUserEmail {
        DomainUserEmail {
            id: model.id,
            user_id: model.user_id,
            email: model.email,
            is_primary: model.is_primary,
            is_verified: model.is_verified,
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(model.created_at, Utc),
            updated_at: DateTime::<Utc>::from_naive_utc_and_offset(model.updated_at, Utc),
        }
    }
}

#[async_trait]
impl UserEmailWriteRepository for UserEmailWriteRepositoryImpl {
    type Error = DbErr;

    async fn create(&self, user_email: DomainUserEmail) -> Result<DomainUserEmail, Self::Error> {
        debug!("Creating new user email with ID: {}", user_email.id);
        let model = Self::to_model(&user_email);

        let res = model.insert(self.db.as_ref()).await?;

        Ok(Self::to_domain(res))
    }

    async fn update(&self, user_email: DomainUserEmail) -> Result<DomainUserEmail, Self::Error> {
        debug!("Updating user email with ID: {}", user_email.id);
        let existing = UserEmails::find_by_id(user_email.id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| {
                error!(user_email_id = %user_email.id, "Failed to update user email: User email not found");
                DbErr::RecordNotFound("User email not found".to_string())
            })?;

        let mut model = user_emails::ActiveModel::from(existing);

        model.email = Set(user_email.email.clone());
        model.is_primary = Set(user_email.is_primary);
        model.is_verified = Set(user_email.is_verified);
        model.updated_at = Set(user_email.updated_at.naive_utc());

        let updated = model.update(self.db.as_ref()).await?;

        Ok(Self::to_domain(updated))
    }

    async fn delete(&self, id: Uuid) -> Result<(), Self::Error> {
        debug!("Deleting user email with ID: {}", id);
        let result = UserEmails::delete_by_id(id).exec(self.db.as_ref()).await?;

        if result.rows_affected == 0 {
            return Err(DbErr::RecordNotFound("User email not found".to_string()));
        }

        Ok(())
    }

    async fn set_as_primary(&self, user_id: Uuid, email_id: Uuid) -> Result<(), Self::Error> {
        debug!("Setting email {} as primary for user {}", email_id, user_id);

        // First, unset all emails as non-primary for this user
        UserEmails::update_many()
            .col_expr(user_emails::Column::IsPrimary, Expr::value(false))
            .filter(user_emails::Column::UserId.eq(user_id))
            .exec(self.db.as_ref())
            .await?;

        // Then set the specified email as primary
        let result = UserEmails::update_many()
            .col_expr(user_emails::Column::IsPrimary, Expr::value(true))
            .col_expr(
                user_emails::Column::UpdatedAt,
                Expr::current_timestamp().into(),
            )
            .filter(user_emails::Column::Id.eq(email_id))
            .filter(user_emails::Column::UserId.eq(user_id)) // Ensure the email belongs to the user
            .exec(self.db.as_ref())
            .await?;

        if result.rows_affected == 0 {
            return Err(DbErr::RecordNotFound("User email not found".to_string()));
        }

        Ok(())
    }
}
