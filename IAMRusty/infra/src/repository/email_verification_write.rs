use async_trait::async_trait;
use domain::entity::email_verification::EmailVerification;
use domain::error::DomainError;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::sync::Arc;
use uuid::Uuid;

use super::entity::user_email_verification;

/// Write-only repository trait for email verification
#[async_trait]
pub trait EmailVerificationWriteRepository: Send + Sync {
    async fn create(&self, verification: &EmailVerification) -> Result<(), DomainError>;
    async fn delete_by_email(&self, email: &str) -> Result<(), DomainError>;
    async fn delete_by_id(&self, id: Uuid) -> Result<(), DomainError>;
}

/// SeaORM implementation of EmailVerificationWriteRepository
pub struct SeaOrmEmailVerificationWriteRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmEmailVerificationWriteRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert domain EmailVerification to SeaORM ActiveModel
    fn to_active_model(&self, verification: &EmailVerification) -> user_email_verification::ActiveModel {
        user_email_verification::ActiveModel {
            id: ActiveValue::Set(verification.id),
            email: ActiveValue::Set(verification.email.clone()),
            verification_token: ActiveValue::Set(verification.verification_token.clone()),
            expires_at: ActiveValue::Set(verification.expires_at.into()),
            created_at: ActiveValue::Set(verification.created_at.into()),
        }
    }
}

#[async_trait]
impl EmailVerificationWriteRepository for SeaOrmEmailVerificationWriteRepository {
    async fn create(&self, verification: &EmailVerification) -> Result<(), DomainError> {
        let active_model = self.to_active_model(verification);
        
        active_model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(format!("Failed to create email verification: {}", e)))?;

        Ok(())
    }

    async fn delete_by_email(&self, email: &str) -> Result<(), DomainError> {
        user_email_verification::Entity::delete_many()
            .filter(user_email_verification::Column::Email.eq(email))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(format!("Failed to delete email verification: {}", e)))?;

        Ok(())
    }

    async fn delete_by_id(&self, id: Uuid) -> Result<(), DomainError> {
        user_email_verification::Entity::delete_many()
            .filter(user_email_verification::Column::Id.eq(id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(format!("Failed to delete email verification by id: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_verification_model() {
        // Simple test to verify the EmailVerification entity works
        let verification = EmailVerification::new(
            "test@example.com".to_string(),
            "token123".to_string(),
            24,
        );

        assert_eq!(verification.email, "test@example.com");
        assert_eq!(verification.verification_token, "token123");
        assert!(!verification.is_expired());
    }
} 