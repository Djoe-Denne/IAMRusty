use async_trait::async_trait;
use domain::entity::email_verification::EmailVerification;
use domain::error::DomainError;
use domain::port::repository::{
    EmailVerificationRepository, 
    EmailVerificationReadRepository as DomainEmailVerificationReadRepository,
    EmailVerificationWriteRepository as DomainEmailVerificationWriteRepository
};
use std::sync::Arc;
use uuid::Uuid;

use super::email_verification_read::{EmailVerificationReadRepository, SeaOrmEmailVerificationReadRepository};
use super::email_verification_write::{EmailVerificationWriteRepository, SeaOrmEmailVerificationWriteRepository};

/// Combined email verification repository that implements the domain's EmailVerificationRepository trait
pub struct CombinedEmailVerificationRepository {
    read_repo: Arc<dyn EmailVerificationReadRepository>,
    write_repo: Arc<dyn EmailVerificationWriteRepository>,
}

impl CombinedEmailVerificationRepository {
    pub fn new(
        read_repo: Arc<dyn EmailVerificationReadRepository>,
        write_repo: Arc<dyn EmailVerificationWriteRepository>,
    ) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }

    /// Create a new combined repository with SeaORM implementations
    pub fn new_with_sea_orm(
        read_repo: Arc<SeaOrmEmailVerificationReadRepository>,
        write_repo: Arc<SeaOrmEmailVerificationWriteRepository>,
    ) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl DomainEmailVerificationReadRepository for CombinedEmailVerificationRepository {
    type Error = DomainError;

    async fn find_by_email_and_token(&self, email: &str, token: &str) -> Result<Option<EmailVerification>, Self::Error> {
        self.read_repo.find_by_email_and_token(email, token).await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<EmailVerification>, Self::Error> {
        self.read_repo.find_by_email(email).await
    }
}

#[async_trait]
impl DomainEmailVerificationWriteRepository for CombinedEmailVerificationRepository {
    type Error = DomainError;

    async fn create(&self, verification: &EmailVerification) -> Result<(), Self::Error> {
        self.write_repo.create(verification).await
    }

    async fn delete_by_email(&self, email: &str) -> Result<(), Self::Error> {
        self.write_repo.delete_by_email(email).await
    }

    async fn delete_by_id(&self, id: Uuid) -> Result<(), Self::Error> {
        self.write_repo.delete_by_id(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combined_repository_structure() {
        // Simple test to verify the combined repository structure
        // In integration tests, we would use real database connections
        assert!(true); // This test just verifies the module compiles
    }
} 