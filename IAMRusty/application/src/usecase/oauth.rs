//! OAuth use case module for OAuth provider authentication

use async_trait::async_trait;
use domain::entity::{
    provider::Provider,
    user::User,
};
use domain::service::oauth_service::OAuthService;
use domain::error::DomainError;
use domain::port::{
    repository::{TokenRepository, UserRepository},
    service::RegistrationTokenService,
};
use std::sync::Arc;
use thiserror::Error;
use serde::{Deserialize, Serialize};

/// OAuth use case error
#[derive(Debug, Error)]
pub enum OAuthError {
    /// Domain service error
    #[error("Domain service error: {0}")]
    DomainError(#[from] DomainError),
}

/// OAuth response enum for different scenarios
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OAuthResponse {
    /// User is complete and can login (has username)
    Login(OAuthLoginResponse),
    /// User needs to complete registration (no username)
    Registration(OAuthRegistrationResponse),
}

/// OAuth login response for complete users
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthLoginResponse {
    /// User data
    pub user: User,
    /// JWT access token (our internal token for user authentication)
    pub access_token: String,
}

/// OAuth registration response for incomplete users
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthRegistrationResponse {
    /// Registration token to complete signup
    pub registration_token: String,
    /// Provider information for registration
    pub provider_info: ProviderInfo,
}

/// Provider information for registration
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Email from provider
    pub email: String,
    /// Username suggestion from provider (optional)
    pub username: Option<String>,
    /// Avatar URL from provider (optional)
    pub avatar_url: Option<String>,
}

/// OAuth use case interface
#[async_trait]
pub trait OAuthUseCase: Send + Sync {
    /// Generate OAuth authorization URL for login flow
    fn generate_start_url(&self, provider: Provider) -> Result<String, OAuthError>;

    /// Exchange authorization code for tokens and login user
    /// This handles the OAuth callback and:
    /// 1. Exchanges the authorization code for provider tokens
    /// 2. Gets user profile from provider
    /// 3. Creates or updates user in our system
    /// 4. Stores provider tokens for future API calls
    /// 5. Issues JWT tokens for authentication OR registration token if incomplete
    async fn oauth_login(
        &self,
        provider: Provider,
        code: String,
    ) -> Result<OAuthResponse, OAuthError>;
}

/// OAuth use case implementation - thin orchestration layer
pub struct OAuthUseCaseImpl<UR, TR, RTS>
where
    UR: UserRepository,
    TR: TokenRepository,
    RTS: RegistrationTokenService,
{
    oauth_service: Arc<OAuthService<UR, TR>>,
    registration_token_service: Arc<RTS>,
}

impl<UR, TR, RTS> OAuthUseCaseImpl<UR, TR, RTS>
where
    UR: UserRepository,
    TR: TokenRepository,
    RTS: RegistrationTokenService,
{
    /// Create a new OAuthUseCaseImpl
    pub fn new(oauth_service: Arc<OAuthService<UR, TR>>, registration_token_service: Arc<RTS>) -> Self {
        Self {
            oauth_service,
            registration_token_service,
        }
    }
}

#[async_trait]
impl<UR, TR, RTS> OAuthUseCase for OAuthUseCaseImpl<UR, TR, RTS>
where
    UR: UserRepository + Send + Sync,
    TR: TokenRepository + Send + Sync,
    RTS: RegistrationTokenService + Send + Sync,
    <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    fn generate_start_url(&self, provider: Provider) -> Result<String, OAuthError> {
        // Delegate to domain service
        self.oauth_service
            .generate_authorize_url(provider.as_str())
            .map_err(Into::into)
    }

    async fn oauth_login(
        &self,
        provider: Provider,
        code: String,
    ) -> Result<OAuthResponse, OAuthError> {
        // Delegate to domain service
        let (user, jwt_token, email) = self.oauth_service
            .process_callback(provider.as_str(), &code)
            .await?;

        // Check if user is complete (has username) or needs registration
        if jwt_token.is_empty() {
            // User needs to complete registration - generate proper registration token
            let registration_token = self.registration_token_service
                .generate_registration_token(user.id, email.clone())
                .map_err(|e| OAuthError::DomainError(e))?;
            
            return Ok(OAuthResponse::Registration(OAuthRegistrationResponse {
                registration_token,
                provider_info: ProviderInfo {
                    email,
                    username: user.username.clone(),
                    avatar_url: user.avatar_url.clone(),
                },
            }));
        }

        // User is complete - return login response
        Ok(OAuthResponse::Login(OAuthLoginResponse {
            user,
            access_token: jwt_token,
        }))
    }
} 