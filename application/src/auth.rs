use domain::entity::provider::{Provider, ProviderTokens, ProviderUserProfile};
use async_trait::async_trait;

/// Authentication service error
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Provider-specific authentication error
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),
    
    /// Invalid response from provider
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Authentication service for OAuth providers
#[async_trait]
pub trait AuthService: Send + Sync {
    /// Error type returned by this service
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Get the provider type
    fn provider(&self) -> Provider;
    
    /// Exchange an authorization code for access tokens and user profile
    async fn exchange_code(
        &self,
        code: String,
        redirect_uri: String,
    ) -> Result<(ProviderTokens, ProviderUserProfile), Self::Error>;
} 