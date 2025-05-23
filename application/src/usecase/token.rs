use async_trait::async_trait;
use thiserror::Error;
use chrono::Utc;
use uuid::Uuid;
use std::sync::Arc;
use domain::port::{
    repository::RefreshTokenRepository,
    service::TokenService,
};

/// Token usecase error
#[derive(Debug, Error)]
pub enum TokenError {
    /// Repository error
    #[error("Repository error: {0}")]
    RepositoryError(Box<dyn std::error::Error + Send + Sync>),

    /// Token service error
    #[error("Token service error: {0}")]
    TokenServiceError(Box<dyn std::error::Error + Send + Sync>),

    /// Token not found
    #[error("Refresh token not found")]
    TokenNotFound,

    /// Token is invalid (revoked)
    #[error("Refresh token is invalid")]
    TokenInvalid,

    /// Token is expired
    #[error("Refresh token is expired")]
    TokenExpired,
}

/// Response for token refresh
#[derive(Debug)]
pub struct RefreshTokenResponse {
    /// New access token
    pub access_token: String,
    /// Expiration time in seconds
    pub expires_in: u64,
}

/// Token use case interface
#[async_trait]
pub trait TokenUseCase: Send + Sync {
    /// Refresh an access token using a refresh token
    async fn refresh_token(&self, refresh_token: String) -> Result<RefreshTokenResponse, TokenError>;
    
    /// Revoke a refresh token
    async fn revoke_token(&self, refresh_token: String) -> Result<(), TokenError>;
    
    /// Revoke all refresh tokens for a user
    async fn revoke_all_tokens(&self, user_id: Uuid) -> Result<u64, TokenError>;
}

/// Token use case implementation
pub struct TokenUseCaseImpl<R, T>
where
    R: RefreshTokenRepository,
    T: TokenService,
{
    refresh_token_repo: Arc<R>,
    token_service: Arc<T>,
}

impl<R, T> TokenUseCaseImpl<R, T>
where
    R: RefreshTokenRepository,
    T: TokenService,
{
    /// Create a new TokenUseCaseImpl
    pub fn new(refresh_token_repo: Arc<R>, token_service: Arc<T>) -> Self {
        Self {
            refresh_token_repo,
            token_service,
        }
    }
}

#[async_trait]
impl<R, T> TokenUseCase for TokenUseCaseImpl<R, T>
where
    R: RefreshTokenRepository + Send + Sync,
    T: TokenService + Send + Sync,
    <R as RefreshTokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    async fn refresh_token(&self, refresh_token: String) -> Result<RefreshTokenResponse, TokenError> {
        // Find the refresh token
        let token = self.refresh_token_repo
            .find_by_token(&refresh_token)
            .await
            .map_err(|e| TokenError::RepositoryError(Box::new(e)))?
            .ok_or(TokenError::TokenNotFound)?;
        
        // Check if the token is valid
        if !token.is_valid {
            return Err(TokenError::TokenInvalid);
        }
        
        // Check if the token is expired
        let now = Utc::now();
        if token.expires_at < now {
            // Invalidate the expired token
            self.refresh_token_repo
                .update_validity(token.id, false)
                .await
                .map_err(|e| TokenError::RepositoryError(Box::new(e)))?;
                
            return Err(TokenError::TokenExpired);
        }
        
        // Generate a new access token
        let new_token = self.token_service
            .generate_access_token(token.user_id)
            .await
            .map_err(|e| TokenError::TokenServiceError(Box::new(e)))?;
        
        // Calculate expiration time in seconds
        let expires_in = (new_token.expires_at - now)
            .num_seconds()
            .max(0) as u64;
        
        Ok(RefreshTokenResponse {
            access_token: new_token.token,
            expires_in,
        })
    }
    
    async fn revoke_token(&self, refresh_token: String) -> Result<(), TokenError> {
        // Find the refresh token
        let token = self.refresh_token_repo
            .find_by_token(&refresh_token)
            .await
            .map_err(|e| TokenError::RepositoryError(Box::new(e)))?
            .ok_or(TokenError::TokenNotFound)?;
        
        // Invalidate the token
        self.refresh_token_repo
            .update_validity(token.id, false)
            .await
            .map_err(|e| TokenError::RepositoryError(Box::new(e)))?;
            
        Ok(())
    }
    
    async fn revoke_all_tokens(&self, user_id: Uuid) -> Result<u64, TokenError> {
        // Delete all refresh tokens for the user
        let count = self.refresh_token_repo
            .delete_by_user_id(user_id)
            .await
            .map_err(|e| TokenError::RepositoryError(Box::new(e)))?;
            
        Ok(count)
    }
} 