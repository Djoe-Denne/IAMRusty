//! Provider use case module

use domain::entity::provider::{Provider, ProviderTokens};
use domain::service::oauth_service::OAuthService;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Provider use case error
#[derive(Debug, Error)]
pub enum ProviderError {
    /// User not found
    #[error("User not found")]
    UserNotFound,

    /// Provider not supported
    #[error("Provider not supported: {0}")]
    ProviderNotSupported(String),

    /// No token found for provider and user
    #[error("No token available for the user and provider")]
    NoTokenForProvider,

    /// Authentication error
    #[error("Authentication error: {0}")]
    AuthError(String),

    /// Database error from repository
    #[error("Database error: {0}")]
    DbError(Box<dyn std::error::Error + Send + Sync>),
}

/// Provider token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTokenResponse {
    /// Access token from the provider
    pub access_token: String,
    /// Token expiration in seconds (optional)
    pub expires_in: Option<u64>,
}

impl From<ProviderTokens> for ProviderTokenResponse {
    fn from(tokens: ProviderTokens) -> Self {
        Self {
            access_token: tokens.access_token,
            expires_in: tokens.expires_in,
        }
    }
}

/// Provider use case interface
#[async_trait]
pub trait ProviderUseCase: Send + Sync {
    /// Get provider access token for authenticated user
    async fn get_provider_token(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<ProviderTokenResponse, ProviderError>;
}

/// Provider use case implementation
pub struct ProviderUseCaseImpl<U, T> 
where
    U: domain::port::repository::UserRepository,
    T: domain::port::repository::TokenRepository,
{
    auth_service: Arc<OAuthService<U, T>>,
}

impl<U, T> ProviderUseCaseImpl<U, T>
where
    U: domain::port::repository::UserRepository,
    T: domain::port::repository::TokenRepository,
{
    /// Create a new ProviderUseCaseImpl
    pub fn new(auth_service: Arc<OAuthService<U, T>>) -> Self {
        Self {
            auth_service,
        }
    }
}

#[async_trait]
impl<U, T> ProviderUseCase for ProviderUseCaseImpl<U, T>
where
    U: domain::port::repository::UserRepository + Send + Sync,
    T: domain::port::repository::TokenRepository + Send + Sync,
    <U as domain::port::repository::UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    <T as domain::port::repository::TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    async fn get_provider_token(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<ProviderTokenResponse, ProviderError> {
        // Use the auth service to get provider token
        let tokens = self.auth_service
            .get_provider_token(&user_id.to_string(), provider.as_str())
            .await
            .map_err(|e| match e {
                domain::error::DomainError::UserNotFound => ProviderError::UserNotFound,
                domain::error::DomainError::ProviderNotSupported(provider) => ProviderError::ProviderNotSupported(provider),
                domain::error::DomainError::NoTokenForProvider(_, _) => ProviderError::NoTokenForProvider,
                domain::error::DomainError::AuthorizationError(msg) => ProviderError::AuthError(msg),
                domain::error::DomainError::RepositoryError(msg) => ProviderError::DbError(Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg))),
                _ => ProviderError::AuthError(e.to_string()),
            })?;

        Ok(ProviderTokenResponse::from(tokens))
    }
}

