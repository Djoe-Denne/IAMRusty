use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, DbErr};
use std::sync::Arc;
use uuid::Uuid;
use domain::entity::user_email::UserEmail as DomainUserEmail;
use domain::port::repository::UserEmailReadRepository;
use tracing::debug;
use chrono::{DateTime, Utc};

use super::entity::{user_emails, prelude::UserEmails};

/// SeaORM implementation of UserEmailReadRepository
#[derive(Clone)]
pub struct UserEmailReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl UserEmailReadRepositoryImpl {
    /// Create a new UserEmailReadRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
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
impl UserEmailReadRepository for UserEmailReadRepositoryImpl {
    type Error = DbErr;

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<DomainUserEmail>, Self::Error> {
        debug!("Reading user emails for user ID: {}", user_id);
        let emails = UserEmails::find()
            .filter(user_emails::Column::UserId.eq(user_id))
            .all(self.db.as_ref())
            .await?;
        
        Ok(emails.into_iter().map(Self::to_domain).collect())
    }
    
    async fn find_by_id(&self, id: Uuid) -> Result<Option<DomainUserEmail>, Self::Error> {
        debug!("Reading user email by ID: {}", id);
        let email = UserEmails::find_by_id(id)
            .one(self.db.as_ref())
            .await?;
        
        Ok(email.map(Self::to_domain))
    }
    
    async fn find_by_email(&self, email: &str) -> Result<Option<DomainUserEmail>, Self::Error> {
        debug!("Reading user email by email: {}", email);
        let user_email = UserEmails::find()
            .filter(user_emails::Column::Email.eq(email))
            .one(self.db.as_ref())
            .await?;
        
        Ok(user_email.map(Self::to_domain))
    }
    
    async fn find_primary_by_user_id(&self, user_id: Uuid) -> Result<Option<DomainUserEmail>, Self::Error> {
        debug!("Reading primary email for user ID: {}", user_id);
        let primary_email = UserEmails::find()
            .filter(user_emails::Column::UserId.eq(user_id))
            .filter(user_emails::Column::IsPrimary.eq(true))
            .one(self.db.as_ref())
            .await?;
        
        Ok(primary_email.map(Self::to_domain))
    }
} 