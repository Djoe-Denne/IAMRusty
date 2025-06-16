use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;
use std::sync::Arc;
use domain::service::{RefreshTokenService, RefreshTokenResponse as DomainRefreshTokenResponse};
use domain::entity::token::JwkSet;
use domain::error::DomainError;

/// Token usecase error
#[derive(Debug, Error)]
pub enum TokenError {
    /// Domain service error
    #[error("Domain service error: {0}")]
    DomainError(#[from] DomainError),
    
    /// Repository error
    #[error("Repository error: {0}")]
    RepositoryError(String),
    
    /// Token service error
    #[error("Token service error: {0}")]
    TokenServiceError(String),
    
    /// Token not found
    #[error("Token not found")]
    TokenNotFound,
    
    /// Token invalid
    #[error("Token invalid")]
    TokenInvalid,
    
    /// Token expired
    #[error("Token expired")]
    TokenExpired,
}

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

impl From<DomainRefreshTokenResponse> for RefreshTokenResponse {
    fn from(domain_response: DomainRefreshTokenResponse) -> Self {
        Self {
            access_token: domain_response.access_token,
            expires_in: domain_response.expires_in,
            refresh_token: domain_response.refresh_token,
            refresh_expires_in: domain_response.refresh_expires_in,
        }
    }
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
    
    /// Get the JSON Web Key Set (JWKS) for token verification
    fn get_jwks(&self) -> JwkSet;
}

/// Token use case implementation - thin orchestration layer
pub struct TokenUseCaseImpl<RTS>
where
    RTS: RefreshTokenService,
{
    refresh_token_service: Arc<RTS>,
}

impl<RTS> TokenUseCaseImpl<RTS>
where
    RTS: RefreshTokenService,
{
    /// Create a new TokenUseCaseImpl
    pub fn new(refresh_token_service: Arc<RTS>) -> Self {
        Self {
            refresh_token_service,
        }
    }
}

#[async_trait]
impl<RTS> TokenUseCase for TokenUseCaseImpl<RTS>
where
    RTS: RefreshTokenService + Send + Sync,
{
    async fn refresh_token(&self, refresh_token: String) -> Result<RefreshTokenResponse, TokenError> {
        // Delegate to domain service
        let domain_response = self.refresh_token_service
            .refresh_token(refresh_token)
            .await?;

        // Convert domain result to use case DTO
        Ok(RefreshTokenResponse::from(domain_response))
    }
    
    async fn revoke_token(&self, refresh_token: String) -> Result<(), TokenError> {
        // Delegate to domain service
        self.refresh_token_service
            .revoke_token(refresh_token)
            .await
            .map_err(Into::into)
    }
    
    async fn revoke_all_tokens(&self, user_id: Uuid) -> Result<u64, TokenError> {
        // Delegate to domain service
        self.refresh_token_service
            .revoke_all_tokens(user_id)
            .await
            .map_err(Into::into)
    }

    fn get_jwks(&self) -> JwkSet {
        // Delegate to domain service
        self.refresh_token_service.get_jwks()
    }
} 