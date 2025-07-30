use super::PasswordService;
use async_trait::async_trait;
use iam_application::usecase::password_reset::PasswordResetError;
use iam_application::usecase::password_reset::PasswordService as PasswordResetPasswordService;
use std::sync::Arc;

/// Adapter that bridges the password reset use case's PasswordService trait with the infrastructure implementation
pub struct PasswordResetServiceAdapter {
    password_service: Arc<PasswordService>,
}

impl PasswordResetServiceAdapter {
    pub fn new(password_service: Arc<PasswordService>) -> Self {
        Self { password_service }
    }
}

#[async_trait]
impl PasswordResetPasswordService for PasswordResetServiceAdapter {
    async fn hash_password(&self, password: &str) -> Result<String, PasswordResetError> {
        self.password_service
            .hash_password(password)
            .map_err(|e| PasswordResetError::ServiceError(e.to_string()))
    }

    async fn verify_password(
        &self,
        password: &str,
        hash: &str,
    ) -> Result<bool, PasswordResetError> {
        self.password_service
            .verify_password(password, hash)
            .map_err(|e| PasswordResetError::ServiceError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iam_domain::error::DomainError;

    #[tokio::test]
    async fn test_hash_password() {
        let password_service = Arc::new(PasswordService::new());
        let adapter = PasswordResetServiceAdapter::new(password_service);

        let password = "test_password_123";
        let result = adapter.hash_password(password).await;

        assert!(result.is_ok());
        let hash = result.unwrap();
        assert!(!hash.is_empty());
        assert_ne!(hash, password); // Should be hashed, not plain text
    }

    #[tokio::test]
    async fn test_verify_password() {
        let password_service = Arc::new(PasswordService::new());
        let adapter = PasswordResetServiceAdapter::new(password_service.clone());

        let password = "test_password_123";

        // First hash the password using the underlying service
        let hash = password_service.hash_password(password).unwrap();

        // Then verify it using the adapter
        let result = adapter.verify_password(password, &hash).await;

        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test with wrong password
        let wrong_result = adapter.verify_password("wrong_password", &hash).await;
        assert!(wrong_result.is_ok());
        assert!(!wrong_result.unwrap());
    }
}
