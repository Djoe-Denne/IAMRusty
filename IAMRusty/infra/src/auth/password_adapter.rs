use super::PasswordService;
use iam_application::usecase::login::PasswordService as AppPasswordService;
use async_trait::async_trait;
use iam_domain::service::auth_service::AuthError;
use std::sync::Arc;

/// Adapter that bridges the application's PasswordService trait with the infrastructure implementation
pub struct PasswordServiceAdapter {
    password_service: Arc<PasswordService>,
}

impl PasswordServiceAdapter {
    pub fn new(password_service: Arc<PasswordService>) -> Self {
        Self { password_service }
    }
}

#[async_trait]
impl AppPasswordService for PasswordServiceAdapter {
    async fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        self.password_service
            .hash_password(password)
            .map_err(|e| AuthError::PasswordHashingError(e.to_string()))
    }

    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError> {
        self.password_service
            .verify_password(password, hash)
            .map_err(|e| AuthError::PasswordHashingError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_password_hashing_and_verification() {
        let password_service = Arc::new(PasswordService::new());
        let adapter = PasswordServiceAdapter::new(password_service);

        let password = "test_password_123";

        // Hash the password
        let hash = adapter.hash_password(password).await.unwrap();

        // Verify correct password
        assert!(adapter.verify_password(password, &hash).await.unwrap());

        // Verify incorrect password
        assert!(!adapter
            .verify_password("wrong_password", &hash)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_different_passwords_produce_different_hashes() {
        let password_service = Arc::new(PasswordService::new());
        let adapter = PasswordServiceAdapter::new(password_service);

        let hash1 = adapter.hash_password("password1").await.unwrap();
        let hash2 = adapter.hash_password("password2").await.unwrap();

        assert_ne!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_same_password_produces_different_hashes_due_to_salt() {
        let password_service = Arc::new(PasswordService::new());
        let adapter = PasswordServiceAdapter::new(password_service);

        let password = "same_password";

        let hash1 = adapter.hash_password(password).await.unwrap();
        let hash2 = adapter.hash_password(password).await.unwrap();

        // Different salts should produce different hashes
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(adapter.verify_password(password, &hash1).await.unwrap());
        assert!(adapter.verify_password(password, &hash2).await.unwrap());
    }
}
