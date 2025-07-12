//! Password reset use case module

use async_trait::async_trait;
use iam_domain::entity::{
    events::{DomainEvent, PasswordResetRequestedEvent},
    password_reset_token::PasswordResetToken,
};
use iam_domain::port::{
    event_publisher::EventPublisher,
    repository::{PasswordResetTokenRepository, UserEmailRepository, UserRepository},
    service::AuthTokenService,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Password reset use case errors
#[derive(Debug, Error)]
pub enum PasswordResetError {
    #[error("User not found")]
    UserNotFound,

    #[error("Invalid reset token")]
    InvalidResetToken,

    #[error("Reset token expired")]
    ResetTokenExpired,

    #[error("Reset token already used")]
    ResetTokenAlreadyUsed,

    #[error("User does not have password authentication")]
    NoPasswordAuth,

    #[error("Current password is incorrect")]
    IncorrectCurrentPassword,

    #[error("Weak password")]
    WeakPassword,

    #[error("Repository error: {0}")]
    RepositoryError(String),

    #[error("Event publishing error: {0}")]
    EventPublishingError(String),

    #[error("Service error: {0}")]
    ServiceError(String),
}

/// Request password reset request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPasswordResetRequest {
    pub email: String,
}

/// Request password reset response (always success to prevent enumeration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPasswordResetResponse {
    pub message: String,
}

/// Validate reset token request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResetTokenRequest {
    pub reset_token: String,
}

/// Validate reset token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResetTokenResponse {
    pub valid: bool,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub message: Option<String>,
    pub email: Option<String>,
}

/// Reset password request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordRequest {
    pub email: Option<String>, // Required for unauthenticated mode
    pub reset_token: Option<String>, // Required for unauthenticated mode
    pub new_password: String,
}

/// Reset password response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordResponse {
    pub message: String,
}

/// Password reset use case trait
#[async_trait]
pub trait PasswordResetUseCase: Send + Sync {
    /// Request a password reset (always returns success to prevent enumeration)
    async fn request_password_reset(
        &self,
        request: RequestPasswordResetRequest,
    ) -> Result<RequestPasswordResetResponse, PasswordResetError>;

    /// Validate a reset token
    async fn validate_reset_token(
        &self,
        request: ValidateResetTokenRequest,
    ) -> Result<ValidateResetTokenResponse, PasswordResetError>;

    /// Reset password (authenticated mode - user ID from JWT)
    async fn reset_password_authenticated(
        &self,
        user_id: Uuid,
        current_password: String,
        new_password: String,
    ) -> Result<ResetPasswordResponse, PasswordResetError>;

    /// Reset password (unauthenticated mode - using token)
    async fn reset_password_unauthenticated(
        &self,
        request: ResetPasswordRequest,
    ) -> Result<ResetPasswordResponse, PasswordResetError>;
}

/// Password reset use case implementation
pub struct PasswordResetUseCaseImpl<UR, UER, PRTR, TS, EP, PS>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    PRTR: PasswordResetTokenRepository,
    TS: AuthTokenService,
    EP: EventPublisher,
    PS: PasswordService,
{
    user_repository: Arc<UR>,
    user_email_repository: Arc<UER>,
    password_reset_token_repository: Arc<PRTR>,
    token_service: Arc<TS>,
    event_publisher: Arc<EP>,
    password_service: Arc<PS>,
}

/// Password service trait for dependency injection
#[async_trait]
pub trait PasswordService: Send + Sync {
    async fn hash_password(&self, password: &str) -> Result<String, PasswordResetError>;
    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, PasswordResetError>;
}

impl<UR, UER, PRTR, TS, EP, PS> PasswordResetUseCaseImpl<UR, UER, PRTR, TS, EP, PS>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    PRTR: PasswordResetTokenRepository,
    TS: AuthTokenService,
    EP: EventPublisher,
    PS: PasswordService,
{
    /// Create a new password reset use case
    pub fn new(
        user_repository: Arc<UR>,
        user_email_repository: Arc<UER>,
        password_reset_token_repository: Arc<PRTR>,
        token_service: Arc<TS>,
        event_publisher: Arc<EP>,
        password_service: Arc<PS>,
    ) -> Self {
        Self {
            user_repository,
            user_email_repository,
            password_reset_token_repository,
            token_service,
            event_publisher,
            password_service,
        }
    }
}

#[async_trait]
impl<UR, UER, PRTR, TS, EP, PS> PasswordResetUseCase
    for PasswordResetUseCaseImpl<UR, UER, PRTR, TS, EP, PS>
where
    UR: UserRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    PRTR: PasswordResetTokenRepository + Send + Sync,
    TS: AuthTokenService + Send + Sync,
    EP: EventPublisher + Send + Sync,
    PS: PasswordService + Send + Sync,

{
    async fn request_password_reset(
        &self,
        request: RequestPasswordResetRequest,
    ) -> Result<RequestPasswordResetResponse, PasswordResetError> {
        // Always return success to prevent user enumeration
        let response = RequestPasswordResetResponse {
            message: "If your email is registered and has password authentication, you will receive a password reset link.".to_string(),
        };

        // Silently try to process the request
        if let Ok(Some(user_email)) = self
            .user_email_repository
            .find_by_email(&request.email)
            .await
        {
            if let Ok(Some(user)) = self.user_repository.find_by_id(user_email.user_id).await {
                // Only proceed if user has password authentication
                if user.password_hash.is_some() {
                    // Generate reset token
                    let raw_token = PasswordResetToken::generate_raw_token();
                    let reset_token = PasswordResetToken::new(user.id, &raw_token, 24); // 24 hours

                    // Store the token
                    if let Ok(()) = self
                        .password_reset_token_repository
                        .create(&reset_token)
                        .await
                    {
                        // Publish event
                        let event = DomainEvent::PasswordResetRequested(
                            PasswordResetRequestedEvent::new(
                                user.id,
                                request.email.clone(),
                                raw_token,
                                reset_token.expires_at,
                            ),
                        );

                        if let Err(e) = self.event_publisher.publish(event).await {
                            tracing::warn!(
                                "Failed to publish password reset requested event: {}",
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(response)
    }

    async fn validate_reset_token(
        &self,
        request: ValidateResetTokenRequest,
    ) -> Result<ValidateResetTokenResponse, PasswordResetError> {
        // Hash the provided token
        let token_hash = PasswordResetToken::hash_token(&request.reset_token);

        // Find the token by hash alone
        let token = self
            .password_reset_token_repository
            .find_by_token_hash(&token_hash)
            .await
            .map_err(|e| PasswordResetError::RepositoryError(e.to_string()))?
            .ok_or(PasswordResetError::InvalidResetToken)?;

        // Check token validity
        if token.is_used() {
            return Err(PasswordResetError::ResetTokenAlreadyUsed);
        }

        if token.is_expired() {
            return Err(PasswordResetError::ResetTokenExpired);
        }

        // Get user email for response
        let user_email = if let Ok(Some(user_email)) = self
            .user_email_repository
            .find_primary_by_user_id(token.user_id)
            .await
        {
            Some(user_email.email)
        } else {
            None
        };

        Ok(ValidateResetTokenResponse {
            valid: true,
            expires_at: Some(token.expires_at),
            message: None,
            email: user_email,
        })
    }

    async fn reset_password_authenticated(
        &self,
        user_id: Uuid,
        current_password: String,
        new_password: String,
    ) -> Result<ResetPasswordResponse, PasswordResetError> {
        // Find user
        let mut user = self
            .user_repository
            .find_by_id(user_id)
            .await
            .map_err(|e| PasswordResetError::RepositoryError(e.to_string()))?
            .ok_or(PasswordResetError::UserNotFound)?;

        // Verify current password
        if let Some(password_hash) = user.password_hash {
            if !self.password_service.verify_password(&current_password, &password_hash).await? {
                return Err(PasswordResetError::IncorrectCurrentPassword);
            }
        } else {
            return Err(PasswordResetError::NoPasswordAuth);
        }

        // Hash new password
        let password_hash = self.password_service.hash_password(&new_password).await?;

        // Update user password
        user.password_hash = Some(password_hash);
        self.user_repository
            .update(user)
            .await
            .map_err(|e| PasswordResetError::RepositoryError(e.to_string()))?;

        // Invalidate all existing reset tokens for this user
        if let Err(e) = self
            .password_reset_token_repository
            .delete_all_for_user(user_id)
            .await
        {
            tracing::warn!(
                "Failed to delete reset tokens for user {}: {}",
                user_id,
                e
            );
        }

        Ok(ResetPasswordResponse {
            message: "Password has been successfully changed".to_string(),
        })
    }

    async fn reset_password_unauthenticated(
        &self,
        request: ResetPasswordRequest,
    ) -> Result<ResetPasswordResponse, PasswordResetError> {
        let reset_token = request.reset_token.ok_or(PasswordResetError::InvalidResetToken)?;

        // Hash the provided token
        let token_hash = PasswordResetToken::hash_token(&reset_token);

        // Find and validate the token by hash
        let mut token = self
            .password_reset_token_repository
            .find_by_token_hash(&token_hash)
            .await
            .map_err(|e| PasswordResetError::RepositoryError(e.to_string()))?
            .ok_or(PasswordResetError::InvalidResetToken)?;

        // Check token validity
        if token.is_used() {
            return Err(PasswordResetError::ResetTokenAlreadyUsed);
        }

        if token.is_expired() {
            return Err(PasswordResetError::ResetTokenExpired);
        }

        // Find user by the user_id from the token
        let mut user = self
            .user_repository
            .find_by_id(token.user_id)
            .await
            .map_err(|e| PasswordResetError::RepositoryError(e.to_string()))?
            .ok_or(PasswordResetError::UserNotFound)?;

        // Hash new password
        let password_hash = self.password_service.hash_password(&request.new_password).await?;

        // Update user password
        user.password_hash = Some(password_hash);
        self.user_repository
            .update(user)
            .await
            .map_err(|e| PasswordResetError::RepositoryError(e.to_string()))?;

        // Mark token as used
        token.mark_as_used();
        self.password_reset_token_repository
            .update(&token)
            .await
            .map_err(|e| PasswordResetError::RepositoryError(e.to_string()))?;

        // Delete all other reset tokens for this user
        if let Err(e) = self
            .password_reset_token_repository
            .delete_all_for_user(token.user_id)
            .await
        {
            tracing::warn!(
                "Failed to delete other reset tokens for user {}: {}",
                token.user_id,
                e
            );
        }

        Ok(ResetPasswordResponse {
            message: "Password has been successfully reset".to_string(),
        })
    }
} 