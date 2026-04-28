use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Password reset token domain entity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordResetToken {
    /// Unique identifier for the reset token
    pub id: Uuid,
    /// User ID this token belongs to
    pub user_id: Uuid,
    /// SHA-256 hash of the raw token
    pub token_hash: String,
    /// When the token expires
    pub expires_at: DateTime<Utc>,
    /// When the token was created
    pub created_at: DateTime<Utc>,
    /// When the token was used (None if not used)
    pub used_at: Option<DateTime<Utc>>,
}

impl PasswordResetToken {
    /// Create a new password reset token
    #[must_use]
    pub fn new(user_id: Uuid, raw_token: &str, expiration_hours: i64) -> Self {
        let now = Utc::now();
        let expires_at = now + Duration::hours(expiration_hours);

        Self {
            id: Uuid::new_v4(),
            user_id,
            token_hash: Self::hash_token(raw_token),
            expires_at,
            created_at: now,
            used_at: None,
        }
    }

    /// Hash a raw token using SHA-256
    #[must_use]
    pub fn hash_token(raw_token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(raw_token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify if a raw token matches this token's hash
    #[must_use]
    pub fn verify_token(&self, raw_token: &str) -> bool {
        self.token_hash == Self::hash_token(raw_token)
    }

    /// Check if the token has expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the token has been used
    #[must_use]
    pub const fn is_used(&self) -> bool {
        self.used_at.is_some()
    }

    /// Check if the token is valid (not expired and not used)
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_used()
    }

    /// Mark the token as used
    pub fn mark_as_used(&mut self) {
        self.used_at = Some(Utc::now());
    }

    /// Generate a secure random token string
    #[must_use]
    pub fn generate_raw_token() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        const TOKEN_LEN: usize = 32;

        let mut rng = rand::thread_rng();
        (0..TOKEN_LEN)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_token() {
        let user_id = Uuid::new_v4();
        let raw_token = "test_token_123";
        let token = PasswordResetToken::new(user_id, raw_token, 24);

        assert_eq!(token.user_id, user_id);
        assert!(token.verify_token(raw_token));
        assert!(!token.is_expired());
        assert!(!token.is_used());
        assert!(token.is_valid());
    }

    #[test]
    fn test_token_verification() {
        let user_id = Uuid::new_v4();
        let raw_token = "test_token_123";
        let token = PasswordResetToken::new(user_id, raw_token, 24);

        assert!(token.verify_token(raw_token));
        assert!(!token.verify_token("wrong_token"));
    }

    #[test]
    fn test_mark_as_used() {
        let user_id = Uuid::new_v4();
        let raw_token = "test_token_123";
        let mut token = PasswordResetToken::new(user_id, raw_token, 24);

        assert!(!token.is_used());
        assert!(token.is_valid());

        token.mark_as_used();

        assert!(token.is_used());
        assert!(!token.is_valid());
    }

    #[test]
    fn test_token_hash_consistency() {
        let raw_token = "test_token_123";
        let hash1 = PasswordResetToken::hash_token(raw_token);
        let hash2 = PasswordResetToken::hash_token(raw_token);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, PasswordResetToken::hash_token("different_token"));
    }

    #[test]
    fn test_generate_raw_token() {
        let token1 = PasswordResetToken::generate_raw_token();
        let token2 = PasswordResetToken::generate_raw_token();

        assert_eq!(token1.len(), 32);
        assert_eq!(token2.len(), 32);
        assert_ne!(token1, token2); // Should be different

        // Should only contain alphanumeric characters
        assert!(token1.chars().all(|c| c.is_alphanumeric()));
        assert!(token2.chars().all(|c| c.is_alphanumeric()));
    }
}
