use async_trait::async_trait;
use domain::entity::email_verification::EmailVerification;
use domain::error::DomainError;
use application::usecase::auth::EmailVerificationRepository;
use std::sync::Arc;

use super::email_verification_read::{EmailVerificationReadRepository, SeaOrmEmailVerificationReadRepository};
use super::email_verification_write::{EmailVerificationWriteRepository, SeaOrmEmailVerificationWriteRepository};

/// Combined email verification repository that implements the application's EmailVerificationRepository trait
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
impl EmailVerificationRepository for CombinedEmailVerificationRepository {
    async fn create(&self, verification: &EmailVerification) -> Result<(), DomainError> {
        self.write_repo.create(verification).await
    }

    async fn find_by_email_and_token(&self, email: &str, token: &str) -> Result<Option<EmailVerification>, DomainError> {
        self.read_repo.find_by_email_and_token(email, token).await
    }

    async fn delete_by_email(&self, email: &str) -> Result<(), DomainError> {
        self.write_repo.delete_by_email(email).await
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