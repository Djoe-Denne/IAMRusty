//! Link Provider use case module

use domain::entity::{
    provider::Provider,
    user::User,
    user_email::UserEmail,
};
use domain::port::{
    repository::{TokenRepository, UserRepository, UserEmailRepository, RefreshTokenRepository},
    service::TokenService,
};
use crate::auth::AuthService;
use crate::usecase::factory::AuthProviderFactory;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Link provider use case error
#[derive(Debug, Error)]
pub enum LinkProviderError {
    /// Authentication error from authentication provider
    #[error("Authentication error: {0}")]
    AuthError(String),

    /// Database error from repository
    #[error("Database error: {0}")]
    DbError(Box<dyn std::error::Error + Send + Sync>),

    /// Token service error
    #[error("Token service error: {0}")]
    TokenError(Box<dyn std::error::Error + Send + Sync>),

    /// User not found
    #[error("User not found")]
    UserNotFound,

    /// Provider already linked to another user
    #[error("Provider account is already linked to another user")]
    ProviderAlreadyLinked,

    /// Provider already linked to the same user
    #[error("Provider is already linked to your account")]
    ProviderAlreadyLinkedToSameUser,
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
pub struct LinkProviderUseCaseImpl<GH, GL, UR, UER, TR, RR, TS> 
where
    GH: AuthService + 'static,
    GL: AuthService + 'static,
    UR: UserRepository,
    UER: UserEmailRepository,
    TR: TokenRepository,
    RR: RefreshTokenRepository,
    TS: TokenService,
{
    auth_factory: Arc<AuthProviderFactory<GH, GL>>,
    user_repo: Arc<UR>,
    user_email_repo: Arc<UER>,
    token_repo: Arc<TR>,
    _refresh_token_repo: Arc<RR>,
    _token_service: Arc<TS>,
}

impl<GH, GL, UR, UER, TR, RR, TS> LinkProviderUseCaseImpl<GH, GL, UR, UER, TR, RR, TS>
where
    GH: AuthService + 'static,
    GL: AuthService + 'static,
    UR: UserRepository,
    UER: UserEmailRepository,
    TR: TokenRepository,
    RR: RefreshTokenRepository,
    TS: TokenService,
{
    /// Create a new LinkProviderUseCaseImpl
    pub fn new(
        github_auth: Arc<GH>,
        gitlab_auth: Arc<GL>,
        user_repo: Arc<UR>,
        user_email_repo: Arc<UER>,
        token_repo: Arc<TR>,
        refresh_token_repo: Arc<RR>,
        token_service: Arc<TS>,
    ) -> Self {
        let auth_factory = Arc::new(AuthProviderFactory::new(github_auth, gitlab_auth));
        
        Self {
            auth_factory,
            user_repo,
            user_email_repo,
            token_repo,
            _refresh_token_repo: refresh_token_repo,
            _token_service: token_service,
        }
    }

    /// Load and verify user exists
    async fn load_user(&self, user_id: Uuid) -> Result<User, LinkProviderError>
    where
        UR: UserRepository,
        <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        self.user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| LinkProviderError::DbError(Box::new(e)))?
            .ok_or(LinkProviderError::UserNotFound)
    }

    /// Exchange authorization code for tokens and user profile
    async fn fetch_provider_profile(
        &self,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<(domain::entity::provider::ProviderTokens, domain::entity::provider::ProviderUserProfile), LinkProviderError>
    where
        GH: AuthService,
        GL: AuthService,
        <GH as AuthService>::Error: std::error::Error + Send + Sync + 'static,
        <GL as AuthService>::Error: std::error::Error + Send + Sync + 'static,
    {
        let auth_service = self.auth_factory.get_auth_service(provider);
        
        let (tokens, profile) = auth_service
            .exchange_code(code, redirect_uri)
            .await
            .map_err(|e| LinkProviderError::AuthError(e.to_string()))?;
        
        Ok((tokens, profile))
    }

    /// Check for provider conflicts with other users
    async fn check_provider_conflicts(
        &self,
        user_id: Uuid,
        provider: Provider,
        provider_user_id: &str,
    ) -> Result<(), LinkProviderError>
    where
        UR: UserRepository,
        <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        let existing_user = self.user_repo
            .find_by_provider_user_id(provider, provider_user_id)
            .await
            .map_err(|e| LinkProviderError::DbError(Box::new(e)))?;

        match existing_user {
            Some(user) if user.id == user_id => {
                Err(LinkProviderError::ProviderAlreadyLinkedToSameUser)
            }
            Some(_) => Err(LinkProviderError::ProviderAlreadyLinked),
            None => Ok(()),
        }
    }

    /// Handle email from provider - add if new, check conflicts if exists
    async fn handle_provider_email(
        &self,
        user_id: Uuid,
        provider: Provider,
        email: Option<String>,
    ) -> Result<(bool, Option<String>), LinkProviderError>
    where
        UER: UserEmailRepository,
        <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        let Some(email) = email else {
            return Ok((false, None));
        };

        let existing_email = self.user_email_repo
            .find_by_email(&email)
            .await
            .map_err(|e| LinkProviderError::DbError(Box::new(e)))?;

        match existing_email {
            Some(existing) if existing.user_id != user_id => {
                tracing::error!(
                    "Email {} from provider {} already belongs to user {}, not adding to user {}",
                    email, provider.as_str(), existing.user_id, user_id
                );
                Err(LinkProviderError::ProviderAlreadyLinked)
            }
            Some(_) => Ok((false, None)), // Email already exists for this user
            None => {
                // Create new secondary email
                let user_email = UserEmail::new_secondary(user_id, email.clone(), false);
                self.user_email_repo.create(user_email).await
                    .map_err(|e| LinkProviderError::DbError(Box::new(e)))?;
                Ok((true, Some(email)))
            }
        }
    }

    /// Save provider tokens
    async fn save_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
        provider_user_id: String,
        tokens: domain::entity::provider::ProviderTokens,
    ) -> Result<(), LinkProviderError>
    where
        TR: TokenRepository,
        <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        self.token_repo
            .save_provider_tokens(user_id, provider, provider_user_id, tokens)
            .await
            .map_err(|e| LinkProviderError::DbError(Box::new(e)))
    }

    /// Get all emails for user
    async fn get_user_emails(&self, user_id: Uuid) -> Result<Vec<UserEmail>, LinkProviderError>
    where
        UER: UserEmailRepository,
        <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        self.user_email_repo
            .find_by_user_id(user_id)
            .await
            .map_err(|e| LinkProviderError::DbError(Box::new(e)))
    }
}

#[async_trait]
impl<GH, GL, UR, UER, TR, RR, TS> LinkProviderUseCase for LinkProviderUseCaseImpl<GH, GL, UR, UER, TR, RR, TS>
where
    GH: AuthService + Send + Sync + 'static,
    GL: AuthService + Send + Sync + 'static,
    UR: UserRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    TR: TokenRepository + Send + Sync,
    RR: RefreshTokenRepository + Send + Sync,
    TS: TokenService + Send + Sync,
    <GH as AuthService>::Error: std::error::Error + Send + Sync + 'static,
    <GL as AuthService>::Error: std::error::Error + Send + Sync + 'static,
    <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    <RR as RefreshTokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    <TS as TokenService>::Error: std::error::Error + Send + Sync + 'static,
{
    fn generate_start_url(&self, provider: Provider) -> Result<String, LinkProviderError> {
        let auth_service = self.auth_factory.get_auth_service(provider);
        Ok(auth_service.generate_authorize_url())
    }

    async fn link_provider(
        &self,
        user_id: Uuid,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<LinkProviderResponse, LinkProviderError> {
        // Step 1: Verify user exists
        let user = self.load_user(user_id).await?;

        // Step 2: Exchange code for tokens and profile
        let (tokens, profile) = self.fetch_provider_profile(
            provider,
            code,
            redirect_uri
        ).await?;

        // Step 3: Check for provider conflicts
        self.check_provider_conflicts(user_id, provider, &profile.id).await?;

        // Step 4: Handle email updates
        let (new_email_added, new_email) = self.handle_provider_email(
            user_id,
            provider,
            profile.email
        ).await?;

        // Step 5: Save provider tokens
        self.save_provider_tokens(user_id, provider, profile.id, tokens).await?;

        // Step 6: Get all user emails and return response
        let emails = self.get_user_emails(user_id).await?;

        Ok(LinkProviderResponse {
            user,
            emails,
            new_email_added,
            new_email,
        })
    }
} 