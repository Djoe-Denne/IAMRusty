use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::entity::password_reset_token::PasswordResetToken;
use crate::error::DomainError;

/// Repository interface for password reset tokens
#[async_trait]
pub trait PasswordResetTokenRepository: Send + Sync {
    /// Error type for repository operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new password reset token
    async fn create(&self, token: &PasswordResetToken) -> Result<(), Self::Error>;

    /// Find a token by user ID and token hash
    async fn find_by_user_and_token_hash(
        &self,
        user_id: Uuid,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, Self::Error>;

    /// Find a token by token hash alone (used when user is unknown)
    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, Self::Error>;

    /// Find the most recent valid token for a user
    async fn find_latest_valid_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<PasswordResetToken>, Self::Error>;

    /// Mark a token as used
    async fn mark_as_used(&self, token_id: Uuid) -> Result<(), Self::Error>;

    /// Delete expired tokens (cleanup operation)
    async fn delete_expired(&self) -> Result<u64, Self::Error>;

    /// Delete all tokens for a user (useful when password is successfully reset)
    async fn delete_all_for_user(&self, user_id: Uuid) -> Result<u64, Self::Error>;

    /// Find token by ID
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PasswordResetToken>, Self::Error>;

    /// Update a token (typically to mark as used)
    async fn update(&self, token: &PasswordResetToken) -> Result<(), Self::Error>;
}

/// Read operations for password reset tokens
#[async_trait]
pub trait PasswordResetTokenReadRepository: Send + Sync {
    /// Error type for repository operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Find a token by user ID and token hash
    async fn find_by_user_and_token_hash(
        &self,
        user_id: Uuid,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, Self::Error>;

    /// Find the most recent valid token for a user
    async fn find_latest_valid_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<PasswordResetToken>, Self::Error>;

    /// Find token by ID
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PasswordResetToken>, Self::Error>;

    /// Count valid tokens for a user
    async fn count_valid_for_user(&self, user_id: Uuid) -> Result<u64, Self::Error>;
}

/// Write operations for password reset tokens
#[async_trait]
pub trait PasswordResetTokenWriteRepository: Send + Sync {
    /// Error type for repository operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new password reset token
    async fn create(&self, token: &PasswordResetToken) -> Result<(), Self::Error>;

    /// Update a token (typically to mark as used)
    async fn update(&self, token: &PasswordResetToken) -> Result<(), Self::Error>;

    /// Mark a token as used
    async fn mark_as_used(&self, token_id: Uuid) -> Result<(), Self::Error>;

    /// Delete expired tokens (cleanup operation)
    async fn delete_expired(&self) -> Result<u64, Self::Error>;

    /// Delete all tokens for a user
    async fn delete_all_for_user(&self, user_id: Uuid) -> Result<u64, Self::Error>;
} 