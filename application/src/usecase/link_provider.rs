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
    GH: AuthService,
    GL: AuthService,
    UR: UserRepository,
    UER: UserEmailRepository,
    TR: TokenRepository,
    RR: RefreshTokenRepository,
    TS: TokenService,
{
    github_auth: Arc<GH>,
    gitlab_auth: Arc<GL>,
    user_repo: Arc<UR>,
    user_email_repo: Arc<UER>,
    token_repo: Arc<TR>,
    _refresh_token_repo: Arc<RR>,
    _token_service: Arc<TS>,
}

impl<GH, GL, UR, UER, TR, RR, TS> LinkProviderUseCaseImpl<GH, GL, UR, UER, TR, RR, TS>
where
    GH: AuthService,
    GL: AuthService,
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
        Self {
            github_auth,
            gitlab_auth,
            user_repo,
            user_email_repo,
            token_repo,
            _refresh_token_repo: refresh_token_repo,
            _token_service: token_service,
        }
    }
}

#[async_trait]
impl<GH, GL, UR, UER, TR, RR, TS> LinkProviderUseCase for LinkProviderUseCaseImpl<GH, GL, UR, UER, TR, RR, TS>
where
    GH: AuthService + Send + Sync,
    GL: AuthService + Send + Sync,
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
        match provider {
            Provider::GitHub => Ok(self.github_auth.generate_authorize_url()),
            Provider::GitLab => Ok(self.gitlab_auth.generate_authorize_url()),
        }
    }

    async fn link_provider(
        &self,
        user_id: Uuid,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<LinkProviderResponse, LinkProviderError> {
        // Verify that the user exists
        let user = self.user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| LinkProviderError::DbError(Box::new(e)))?
            .ok_or(LinkProviderError::UserNotFound)?;

        // Exchange authorization code for tokens
        let (tokens, profile) = match provider {
            Provider::GitHub => {
                self.github_auth
                    .exchange_code(code, redirect_uri)
                    .await
                    .map_err(|e| LinkProviderError::AuthError(e.to_string()))?
            }
            Provider::GitLab => {
                self.gitlab_auth
                    .exchange_code(code, redirect_uri)
                    .await
                    .map_err(|e| LinkProviderError::AuthError(e.to_string()))?
            }
        };

        // Check if this provider account is already linked to any user
        let existing_user_with_provider = self.user_repo
            .find_by_provider_user_id(provider, &profile.id)
            .await
            .map_err(|e| LinkProviderError::DbError(Box::new(e)))?;

        if let Some(existing_user) = existing_user_with_provider {
            if existing_user.id == user_id {
                // Provider is already linked to the same user
                return Err(LinkProviderError::ProviderAlreadyLinkedToSameUser);
            } else {
                // Provider is linked to a different user
                return Err(LinkProviderError::ProviderAlreadyLinked);
            }
        }

        // Save provider tokens with provider-specific user ID
        self.token_repo
            .save_provider_tokens(user.id, provider, profile.id, tokens)
            .await
            .map_err(|e| LinkProviderError::DbError(Box::new(e)))?;

        // Handle email from the new provider
        let mut new_email_added = false;
        let mut new_email = None;
        
        if let Some(email) = profile.email {
            // Check if this email already exists for any user
            let existing_email = self.user_email_repo
                .find_by_email(&email)
                .await
                .map_err(|e| LinkProviderError::DbError(Box::new(e)))?;

            match existing_email {
                Some(existing) => {
                    if existing.user_id != user_id {
                        // Email belongs to a different user - we'll take the permissive approach
                        // and just log a warning but continue with the linking
                        tracing::warn!(
                            "Email {} from provider {} already belongs to user {}, not adding to user {}",
                            email, provider.as_str(), existing.user_id, user_id
                        );
                    }
                    // Email already exists for this user - nothing to do
                }
                None => {
                    // New email - add it as a secondary email (not verified)
                    let user_email = UserEmail::new_secondary(
                        user_id,
                        email.clone(),
                        false, // OAuth emails are typically not verified initially
                    );

                    self.user_email_repo
                        .create(user_email)
                        .await
                        .map_err(|e| LinkProviderError::DbError(Box::new(e)))?;

                    new_email_added = true;
                    new_email = Some(email);
                }
            }
        }

        // Get all user emails after potential addition
        let emails = self.user_email_repo
            .find_by_user_id(user_id)
            .await
            .map_err(|e| LinkProviderError::DbError(Box::new(e)))?;

        Ok(LinkProviderResponse {
            user,
            emails,
            new_email_added,
            new_email,
        })
    }
} 