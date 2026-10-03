//! Provider use case module

use async_trait::async_trait;
use iam_domain::entity::provider::{Provider, ProviderTokens};
use iam_domain::service::oauth_service::OAuthService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

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

    /// Revoke provider token for authenticated user
    async fn revoke_provider_token(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<(), ProviderError>;
}

/// Provider use case implementation
pub struct ProviderUseCaseImpl<U, T, UE>
where
    U: iam_domain::port::repository::UserRepository,
    T: iam_domain::port::repository::TokenRepository,
    UE: iam_domain::port::repository::UserEmailRepository,
{
    auth_service: Arc<OAuthService<U, T, UE>>,
}

impl<U, T, UE> ProviderUseCaseImpl<U, T, UE>
where
    U: iam_domain::port::repository::UserRepository,
    T: iam_domain::port::repository::TokenRepository,
    UE: iam_domain::port::repository::UserEmailRepository,
{
    /// Create a new `ProviderUseCaseImpl`
    pub const fn new(auth_service: Arc<OAuthService<U, T, UE>>) -> Self {
        Self { auth_service }
    }
}

#[async_trait]
impl<U, T, UE> ProviderUseCase for ProviderUseCaseImpl<U, T, UE>
where
    U: iam_domain::port::repository::UserRepository + Send + Sync,
    T: iam_domain::port::repository::TokenRepository + Send + Sync,
    UE: iam_domain::port::repository::UserEmailRepository + Send + Sync,
    <U as iam_domain::port::repository::UserRepository>::Error:
        std::error::Error + Send + Sync + 'static,
    <T as iam_domain::port::repository::TokenRepository>::Error:
        std::error::Error + Send + Sync + 'static,
    <UE as iam_domain::port::repository::UserEmailRepository>::Error:
        std::error::Error + Send + Sync + 'static,
{
    async fn get_provider_token(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<ProviderTokenResponse, ProviderError> {
        self.auth_service
            .get_provider_token(user_id, provider)
            .await
            .map(Into::into)
            .map_err(|e| match e {
                iam_domain::error::DomainError::UserNotFound => ProviderError::UserNotFound,
                iam_domain::error::DomainError::ProviderNotSupported(p) => {
                    ProviderError::ProviderNotSupported(p)
                }
                iam_domain::error::DomainError::NoTokenForProvider => {
                    ProviderError::NoTokenForProvider
                }
                iam_domain::error::DomainError::RepositoryError(msg) => {
                    ProviderError::DbError(Box::new(std::io::Error::other(msg)))
                }
                _ => ProviderError::AuthError(e.to_string()),
            })
    }

    async fn revoke_provider_token(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<(), ProviderError> {
        self.auth_service
            .revoke_provider_token(user_id, provider)
            .await
            .map_err(|e| match e {
                iam_domain::error::DomainError::UserNotFound => ProviderError::UserNotFound,
                iam_domain::error::DomainError::ProviderNotSupported(p) => {
                    ProviderError::ProviderNotSupported(p)
                }
                iam_domain::error::DomainError::NoTokenForProvider => {
                    ProviderError::NoTokenForProvider
                }
                iam_domain::error::DomainError::RepositoryError(msg) => {
                    ProviderError::DbError(Box::new(std::io::Error::other(msg)))
                }
                _ => ProviderError::AuthError(e.to_string()),
            })
    }
}
