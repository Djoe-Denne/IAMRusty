use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use crate::entity::token::JwkSet;
use crate::error::DomainError;
use crate::port::{
    repository::RefreshTokenRepository,
    service::{AuthTokenService, JwtTokenEncoder},
};

/// Response for token refresh
#[derive(Debug)]
pub struct RefreshTokenResponse {
    /// New access token
    pub access_token: String,
    /// Access token expiration time in seconds
    pub expires_in: u64,
    /// New refresh token (replaces the old one)
    pub refresh_token: String,
    /// Refresh token expiration time in seconds
    pub refresh_expires_in: u64,
}

/// Refresh token domain service trait
#[async_trait]
pub trait RefreshTokenService: Send + Sync {
    /// Refresh an access token using a refresh token
    async fn refresh_token(
        &self,
        refresh_token: String,
    ) -> Result<RefreshTokenResponse, DomainError>;

    /// Revoke a refresh token
    async fn revoke_token(&self, refresh_token: String) -> Result<(), DomainError>;

    /// Revoke all refresh tokens for a user
    async fn revoke_all_tokens(&self, user_id: Uuid) -> Result<u64, DomainError>;

    /// Get the JSON Web Key Set (JWKS) for token verification
    fn get_jwks(&self) -> JwkSet;
}

/// Refresh token domain service implementation
pub struct RefreshTokenServiceImpl<R, T>
where
    R: RefreshTokenRepository,
    T: AuthTokenService + JwtTokenEncoder,
{
    refresh_token_repo: Arc<R>,
    token_service: Arc<T>,
}

impl<R, T> RefreshTokenServiceImpl<R, T>
where
    R: RefreshTokenRepository,
    T: AuthTokenService + JwtTokenEncoder,
{
    /// Create a new RefreshTokenServiceImpl
    pub fn new(refresh_token_repo: Arc<R>, token_service: Arc<T>) -> Self {
        Self {
            refresh_token_repo,
            token_service,
        }
    }
}

#[async_trait]
impl<R, T> RefreshTokenService for RefreshTokenServiceImpl<R, T>
where
    R: RefreshTokenRepository + Send + Sync,
    T: AuthTokenService + JwtTokenEncoder + Send + Sync,
    <R as RefreshTokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    async fn refresh_token(
        &self,
        refresh_token: String,
    ) -> Result<RefreshTokenResponse, DomainError> {
        // Find the refresh token
        let old_token = self
            .refresh_token_repo
            .find_by_token(&refresh_token)
            .await
            .map_err(|e| {
                debug!("Error finding refresh token: {}", e);
                DomainError::RepositoryError(e.to_string())
            })?
            .ok_or(DomainError::TokenNotFound)?;

        // Check if the token is valid
        if !old_token.is_valid {
            debug!("Invalid refresh token: {}", refresh_token);
            return Err(DomainError::InvalidToken);
        }

        // Check if the token is expired
        let now = Utc::now();
        if old_token.expires_at < now {
            // Invalidate the expired token
            self.refresh_token_repo
                .update_validity(old_token.id, false)
                .await
                .map_err(|e| {
                    debug!("Error updating refresh token validity: {}", e);
                    DomainError::RepositoryError(e.to_string())
                })?;

            return Err(DomainError::TokenExpired);
        }

        // Generate a new access token
        let new_access_token = self
            .token_service
            .generate_access_token(old_token.user_id)
            .await
            .map_err(|e| {
                debug!("Error generating access token: {}", e);
                DomainError::TokenServiceError(e.to_string())
            })?;

        // Generate a new refresh token
        let new_refresh_token = self
            .token_service
            .generate_refresh_token(old_token.user_id)
            .await
            .map_err(|e| {
                debug!("Error generating refresh token: {}", e);
                DomainError::TokenServiceError(e.to_string())
            })?;

        // Store the new refresh token and delete the old token atomically.
        self.refresh_token_repo
            .rotate(old_token.id, new_refresh_token.clone())
            .await
            .map_err(|e| {
                debug!("Error rotating refresh token: {}", e);
                DomainError::RepositoryError(e.to_string())
            })?;

        // Calculate expiration times in seconds
        let access_expires_in = (new_access_token.expires_at - now).num_seconds().max(0) as u64;

        let refresh_expires_in = (new_refresh_token.expires_at - now).num_seconds().max(0) as u64;

        Ok(RefreshTokenResponse {
            access_token: new_access_token.token,
            expires_in: access_expires_in,
            refresh_token: new_refresh_token.token,
            refresh_expires_in,
        })
    }

    async fn revoke_token(&self, refresh_token: String) -> Result<(), DomainError> {
        // Find the refresh token
        let token = self
            .refresh_token_repo
            .find_by_token(&refresh_token)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .ok_or(DomainError::TokenNotFound)?;

        // Invalidate the token
        self.refresh_token_repo
            .update_validity(token.id, false)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        Ok(())
    }

    async fn revoke_all_tokens(&self, user_id: Uuid) -> Result<u64, DomainError> {
        // Delete all refresh tokens for the user
        let count = self
            .refresh_token_repo
            .delete_by_user_id(user_id)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        Ok(count)
    }

    fn get_jwks(&self) -> JwkSet {
        self.token_service.jwks()
    }
}
