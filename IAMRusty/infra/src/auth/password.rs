use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use domain::error::DomainError;
use tracing::error;

/// Password hashing service using Argon2
pub struct PasswordService {
    argon2: Argon2<'static>,
}

impl PasswordService {
    /// Create a new password service with default Argon2 configuration
    pub fn new() -> Self {
        // Use default Argon2 configuration which is secure for most applications
        Self {
            argon2: Argon2::default(),
        }
    }

    /// Hash a password using Argon2
    pub fn hash_password(&self, password: &str) -> Result<String, DomainError> {
        let salt = SaltString::generate(&mut OsRng);
        match self.argon2.hash_password(password.as_bytes(), &salt) {
            Ok(hash) => Ok(hash.to_string()),
            Err(e) => {
                error!("Password hashing failed: {}", e);
                Err(DomainError::RepositoryError(format!(
                    "Password hashing failed: {}",
                    e
                )))
            }
        }
    }

    /// Verify a password against its hash
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, DomainError> {
        let parsed_hash = PasswordHash::new(hash).map_err(|e| {
            error!("Invalid password hash format: {}", e);
            DomainError::RepositoryError(format!("Invalid password hash format: {}", e))
        })?;

        match self
            .argon2
            .verify_password(password.as_bytes(), &parsed_hash)
        {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false), // Password doesn't match
            Err(e) => Err(DomainError::RepositoryError(format!(
                "Password verification failed: {}",
                e
            ))),
        }
    }
}

impl Default for PasswordService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing_and_verification() {
        let service = PasswordService::new();
        let password = "test_password_123";

        // Hash the password
        let hash = service.hash_password(password).unwrap();

        // Verify correct password
        assert!(service.verify_password(password, &hash).unwrap());

        // Verify incorrect password
        assert!(!service.verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_different_passwords_produce_different_hashes() {
        let service = PasswordService::new();

        let hash1 = service.hash_password("password1").unwrap();
        let hash2 = service.hash_password("password2").unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_same_password_produces_different_hashes_due_to_salt() {
        let service = PasswordService::new();
        let password = "same_password";

        let hash1 = service.hash_password(password).unwrap();
        let hash2 = service.hash_password(password).unwrap();

        // Different salts should produce different hashes
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(service.verify_password(password, &hash1).unwrap());
        assert!(service.verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_invalid_hash_format() {
        let service = PasswordService::new();

        let result = service.verify_password("password", "invalid_hash");
        assert!(result.is_err());
    }
}
