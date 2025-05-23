//! Login use case module

use domain::entity::{
    provider::{Provider, ProviderTokens},
    user::User,
};
use domain::port::{
    repository::{TokenRepository, UserRepository, RefreshTokenRepository},
    service::TokenService,
};
use crate::auth::AuthService;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;
use chrono::Utc;

/// Login use case error
#[derive(Debug, Error)]
pub enum LoginError {
    /// Authentication error from authentication provider
    #[error("Authentication error: {0}")]
    AuthError(String),

    /// Database error from repository
    #[error("Database error: {0}")]
    DbError(Box<dyn std::error::Error + Send + Sync>),

    /// Token service error
    #[error("Token service error: {0}")]
    TokenError(Box<dyn std::error::Error + Send + Sync>),
}

/// Login response
#[derive(Debug)]
pub struct LoginResponse {
    /// User data
    pub user: User,
    /// JWT access token
    pub access_token: String,
    /// Access token expiration in seconds
    pub expires_in: u64,
    /// Refresh token for getting new access tokens
    pub refresh_token: String,
}

/// Login use case interface
#[async_trait]
pub trait LoginUseCase: Send + Sync {
    /// Exchange authorization code for tokens and login user
    async fn login(
        &self,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<LoginResponse, LoginError>;
}

/// Login use case implementation
pub struct LoginUseCaseImpl<GH, GL, UR, TR, RR, TS> 
where
    GH: AuthService,
    GL: AuthService,
    UR: UserRepository,
    TR: TokenRepository,
    RR: RefreshTokenRepository,
    TS: TokenService,
{
    github_auth: Arc<GH>,
    gitlab_auth: Arc<GL>,
    user_repo: Arc<UR>,
    token_repo: Arc<TR>,
    refresh_token_repo: Arc<RR>,
    token_service: Arc<TS>,
}

impl<GH, GL, UR, TR, RR, TS> LoginUseCaseImpl<GH, GL, UR, TR, RR, TS>
where
    GH: AuthService,
    GL: AuthService,
    UR: UserRepository,
    TR: TokenRepository,
    RR: RefreshTokenRepository,
    TS: TokenService,
{
    /// Create a new LoginUseCaseImpl
    pub fn new(
        github_auth: Arc<GH>,
        gitlab_auth: Arc<GL>,
        user_repo: Arc<UR>,
        token_repo: Arc<TR>,
        refresh_token_repo: Arc<RR>,
        token_service: Arc<TS>,
    ) -> Self {
        Self {
            github_auth,
            gitlab_auth,
            user_repo,
            token_repo,
            refresh_token_repo,
            token_service,
        }
    }
}

#[async_trait]
impl<GH, GL, UR, TR, RR, TS> LoginUseCase for LoginUseCaseImpl<GH, GL, UR, TR, RR, TS>
where
    GH: AuthService + Send + Sync,
    GL: AuthService + Send + Sync,
    UR: UserRepository + Send + Sync,
    TR: TokenRepository + Send + Sync,
    RR: RefreshTokenRepository + Send + Sync,
    TS: TokenService + Send + Sync,
    GH::Error: std::error::Error + Send + Sync + 'static,
    GL::Error: std::error::Error + Send + Sync + 'static,
    <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    <RR as RefreshTokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    TS::Error: std::error::Error + Send + Sync + 'static,
{
    async fn login(
        &self,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<LoginResponse, LoginError> {
        // Exchange authorization code for tokens
        let (tokens, profile) = match provider {
            Provider::GitHub => {
                self.github_auth
                    .exchange_code(code, redirect_uri)
                    .await
                    .map_err(|e| LoginError::AuthError(e.to_string()))?
            }
            Provider::GitLab => {
                self.gitlab_auth
                    .exchange_code(code, redirect_uri)
                    .await
                    .map_err(|e| LoginError::AuthError(e.to_string()))?
            }
        };

        // Store provider tokens
        let provider_user_id = format!("{}_{}", provider.as_str(), profile.id);

        // Check if user exists
        let user = match self
            .user_repo
            .find_by_provider_user_id(provider, &profile.id)
            .await
            .map_err(|e| LoginError::DbError(Box::new(e)))?
        {
            Some(user) => {
                // Update user details
                let mut updated_user = user.clone();
                updated_user.username = profile.username;
                updated_user.email = profile.email;
                updated_user.avatar_url = profile.avatar_url;
                updated_user.updated_at = Utc::now();

                self.user_repo
                    .update(updated_user)
                    .await
                    .map_err(|e| LoginError::DbError(Box::new(e)))?
            }
            None => {
                // Create new user
                let new_user = User {
                    id: Uuid::new_v4(),
                    provider_user_id,
                    username: profile.username,
                    email: profile.email,
                    avatar_url: profile.avatar_url,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };

                self.user_repo
                    .create(new_user)
                    .await
                    .map_err(|e| LoginError::DbError(Box::new(e)))?
            }
        };

        // Save provider tokens
        self.token_repo
            .save_provider_tokens(user.id, provider, tokens)
            .await
            .map_err(|e| LoginError::DbError(Box::new(e)))?;

        // Generate JWT access token
        let jwt_token = self
            .token_service
            .generate_access_token(user.id)
            .await
            .map_err(|e| LoginError::TokenError(Box::new(e)))?;
            
        // Generate refresh token
        let refresh_token = self
            .token_service
            .generate_refresh_token(user.id)
            .await
            .map_err(|e| LoginError::TokenError(Box::new(e)))?;
            
        // Store refresh token in database
        self.refresh_token_repo
            .create(refresh_token.clone())
            .await
            .map_err(|e| LoginError::DbError(Box::new(e)))?;

        // Calculate expiration time in seconds
        let now = Utc::now();
        let expires_in = (jwt_token.expires_at - now)
            .num_seconds()
            .max(0) as u64;

        Ok(LoginResponse {
            user,
            access_token: jwt_token.token,
            expires_in,
            refresh_token: refresh_token.token,
        })
    }
} 