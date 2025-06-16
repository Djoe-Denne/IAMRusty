//! Link Provider use case module

use crate::auth::OAuthService;
use crate::usecase::factory::OAuthProviderFactory;
use async_trait::async_trait;
use domain::entity::{provider::Provider, user::User, user_email::UserEmail};
use domain::error::DomainError;
use domain::service::ProviderLinkService;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Link provider use case error
#[derive(Debug, Error)]
pub enum LinkProviderError {
    /// Authentication error from authentication provider
    #[error("Authentication error: {0}")]
    AuthError(String),

    /// Domain service error
    #[error("Domain service error: {0}")]
    DomainError(#[from] DomainError),

    /// Provider not configured
    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),
}

/// Link provider response
#[derive(Debug)]
pub struct LinkProviderResponse {
    /// Updated user data
    pub user: User,
    /// All user emails (including the new one if added)
    pub emails: Vec<UserEmail>,
    /// Whether a new email was added
    pub new_email_added: bool,
    /// The new email that was added (if any)
    pub new_email: Option<String>,
}

/// Link provider use case interface
#[async_trait]
pub trait LinkProviderUseCase: Send + Sync {
    /// Generate OAuth authorization URL for link provider flow
    fn generate_start_url(&self, provider: Provider) -> Result<String, LinkProviderError>;

    /// Link a new OAuth provider to an existing authenticated user
    async fn link_provider(
        &self,
        user_id: Uuid,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<LinkProviderResponse, LinkProviderError>;
}

/// Link provider use case implementation
pub struct LinkProviderUseCaseImpl<GH, GL, UR, UER, TR>
where
    GH: OAuthService + 'static,
    GL: OAuthService + 'static,
    UR: domain::port::repository::UserRepository,
    UER: domain::port::repository::UserEmailRepository,
    TR: domain::port::repository::TokenRepository,
{
    auth_factory: Arc<OAuthProviderFactory<GH, GL>>,
    provider_link_service: Arc<ProviderLinkService<UR, UER, TR>>,
}

impl<GH, GL, UR, UER, TR> LinkProviderUseCaseImpl<GH, GL, UR, UER, TR>
where
    GH: OAuthService + 'static,
    GL: OAuthService + 'static,
    UR: domain::port::repository::UserRepository,
    UER: domain::port::repository::UserEmailRepository,
    TR: domain::port::repository::TokenRepository,
{
    /// Create a new LinkProviderUseCaseImpl
    pub fn new(
        github_auth: Arc<GH>,
        gitlab_auth: Arc<GL>,
        provider_link_service: Arc<ProviderLinkService<UR, UER, TR>>,
    ) -> Self {
        let auth_factory = Arc::new(OAuthProviderFactory::new(github_auth, gitlab_auth));

        Self {
            auth_factory,
            provider_link_service,
        }
    }

    /// Exchange authorization code for tokens and user profile
    async fn fetch_provider_profile(
        &self,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<
        (
            domain::entity::provider::ProviderTokens,
            domain::entity::provider::ProviderUserProfile,
        ),
        LinkProviderError,
    >
    where
        GH: OAuthService,
        GL: OAuthService,
        <GH as OAuthService>::Error: std::error::Error + Send + Sync + 'static,
        <GL as OAuthService>::Error: std::error::Error + Send + Sync + 'static,
    {
        let auth_service = self.auth_factory.get_oauth_service(provider);

        let (tokens, profile) = auth_service
            .exchange_code(code, redirect_uri)
            .await
            .map_err(|e| LinkProviderError::AuthError(e.to_string()))?;

        Ok((tokens, profile))
    }
}

#[async_trait]
impl<GH, GL, UR, UER, TR> LinkProviderUseCase for LinkProviderUseCaseImpl<GH, GL, UR, UER, TR>
where
    GH: OAuthService + Send + Sync + 'static,
    GL: OAuthService + Send + Sync + 'static,
    UR: domain::port::repository::UserRepository + Send + Sync,
    UER: domain::port::repository::UserEmailRepository + Send + Sync,
    TR: domain::port::repository::TokenRepository + Send + Sync,
    <GH as OAuthService>::Error: std::error::Error + Send + Sync + 'static,
    <GL as OAuthService>::Error: std::error::Error + Send + Sync + 'static,
    <UR as domain::port::repository::UserRepository>::Error:
        std::error::Error + Send + Sync + 'static,
    <UER as domain::port::repository::UserEmailRepository>::Error:
        std::error::Error + Send + Sync + 'static,
    <TR as domain::port::repository::TokenRepository>::Error:
        std::error::Error + Send + Sync + 'static,
{
    fn generate_start_url(&self, provider: Provider) -> Result<String, LinkProviderError> {
        let auth_service = self.auth_factory.get_oauth_service(provider);
        Ok(auth_service.generate_authorize_url())
    }

    async fn link_provider(
        &self,
        user_id: Uuid,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<LinkProviderResponse, LinkProviderError> {
        // Step 1: Exchange code for tokens and profile
        let (tokens, profile) = self
            .fetch_provider_profile(provider, code, redirect_uri)
            .await?;

        // Step 2: Use domain service to handle the business logic
        let result = self
            .provider_link_service
            .link_provider_to_user(user_id, provider, profile.id.clone(), tokens, profile)
            .await?;

        // Step 3: Convert domain result to use case response
        Ok(LinkProviderResponse {
            user: result.user,
            emails: result.emails,
            new_email_added: result.new_email_added,
            new_email: result.new_email,
        })
    }
}
