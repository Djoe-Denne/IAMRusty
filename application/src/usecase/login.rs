//! Login use case module

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
    /// Primary email address from UserEmail entity
    pub email: String,
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
    /// Generate OAuth authorization URL for login flow
    fn generate_start_url(&self, provider: Provider) -> Result<String, LoginError>;

    /// Exchange authorization code for tokens and login user
    async fn login(
        &self,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<LoginResponse, LoginError>;
}

/// Login use case implementation
pub struct LoginUseCaseImpl<GH, GL, UR, UER, TR, RR, TS> 
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
    refresh_token_repo: Arc<RR>,
    token_service: Arc<TS>,
}

impl<GH, GL, UR, UER, TR, RR, TS> LoginUseCaseImpl<GH, GL, UR, UER, TR, RR, TS>
where
    GH: AuthService + 'static,
    GL: AuthService + 'static,
    UR: UserRepository,
    UER: UserEmailRepository,
    TR: TokenRepository,
    RR: RefreshTokenRepository,
    TS: TokenService,
{
    /// Create a new LoginUseCaseImpl
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
            refresh_token_repo,
            token_service,
        }
    }

    /// Exchange authorization code for tokens and user profile
    async fn exchange_code_for_profile(
        &self,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<(domain::entity::provider::ProviderTokens, domain::entity::provider::ProviderUserProfile), LoginError>
    where
        GH: AuthService,
        GL: AuthService,
        <GH as AuthService>::Error: std::error::Error + Send + Sync + 'static,
        <GL as AuthService>::Error: std::error::Error + Send + Sync + 'static,
    {
        let auth_service = self.auth_factory.get_auth_service(provider);
        
        auth_service
            .exchange_code(code, redirect_uri)
            .await
            .map_err(|e| LoginError::AuthError(e.to_string()))
    }

    /// Find or create user based on OAuth profile
    async fn find_or_create_user(
        &self,
        profile: &domain::entity::provider::ProviderUserProfile,
        email: &str,
    ) -> Result<User, LoginError>
    where
        UR: UserRepository,
        UER: UserEmailRepository,
        <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
        <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        // Check if a user exists with this email
        let existing_email = self.user_email_repo
            .find_by_email(email)
            .await
            .map_err(|e| LoginError::DbError(Box::new(e)))?;

        match existing_email {
            Some(user_email) => self.update_existing_user(user_email.user_id, profile).await,
            None => self.create_new_user(profile, email).await,
        }
    }

    /// Update existing user with latest profile data
    async fn update_existing_user(
        &self,
        user_id: Uuid,
        profile: &domain::entity::provider::ProviderUserProfile,
    ) -> Result<User, LoginError>
    where
        UR: UserRepository,
        <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        let user = self.user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| LoginError::DbError(Box::new(e)))?
            .ok_or_else(|| LoginError::DbError("User not found for email".into()))?;

        // Check if update is needed
        if user.username == profile.username && user.avatar_url == profile.avatar_url {
            return Ok(user);
        }

        // Update user profile
        let mut updated_user = user;
        updated_user.username = profile.username.clone();
        updated_user.avatar_url = profile.avatar_url.clone();
        updated_user.updated_at = Utc::now();

        self.user_repo
            .update(updated_user)
            .await
            .map_err(|e| LoginError::DbError(Box::new(e)))
    }

    /// Create new user with primary email
    async fn create_new_user(
        &self,
        profile: &domain::entity::provider::ProviderUserProfile,
        email: &str,
    ) -> Result<User, LoginError>
    where
        UR: UserRepository,
        UER: UserEmailRepository,
        <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
        <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        // Create new user
        let new_user = User::new(
            profile.username.clone(),
            profile.avatar_url.clone(),
        );

        let created_user = self.user_repo
            .create(new_user)
            .await
            .map_err(|e| LoginError::DbError(Box::new(e)))?;

        // Create primary email for the new user
        let user_email = UserEmail::new_primary(
            created_user.id,
            email.to_string(),
            false, // OAuth emails are typically not verified initially
        );

        self.user_email_repo
            .create(user_email)
            .await
            .map_err(|e| LoginError::DbError(Box::new(e)))?;

        Ok(created_user)
    }

    /// Save provider tokens for user
    async fn save_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
        provider_user_id: String,
        tokens: domain::entity::provider::ProviderTokens,
    ) -> Result<(), LoginError>
    where
        TR: TokenRepository,
        <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        self.token_repo
            .save_provider_tokens(user_id, provider, provider_user_id, tokens)
            .await
            .map_err(|e| LoginError::DbError(Box::new(e)))
    }

    /// Generate and store authentication tokens
    async fn generate_auth_tokens(
        &self,
        user_id: Uuid,
    ) -> Result<(domain::entity::token::JwtToken, domain::entity::token::RefreshToken), LoginError>
    where
        TS: TokenService,
        RR: RefreshTokenRepository,
        <TS as TokenService>::Error: std::error::Error + Send + Sync + 'static,
        <RR as RefreshTokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        // Generate JWT access token
        let jwt_token = self.token_service
            .generate_access_token(user_id)
            .await
            .map_err(|e| LoginError::TokenError(Box::new(e)))?;

        // Generate refresh token
        let refresh_token = self.token_service
            .generate_refresh_token(user_id)
            .await
            .map_err(|e| LoginError::TokenError(Box::new(e)))?;

        // Store refresh token in database
        self.refresh_token_repo
            .create(refresh_token.clone())
            .await
            .map_err(|e| LoginError::DbError(Box::new(e)))?;

        Ok((jwt_token, refresh_token))
    }
}

#[async_trait]
impl<GH, GL, UR, UER, TR, RR, TS> LoginUseCase for LoginUseCaseImpl<GH, GL, UR, UER, TR, RR, TS>
where
    GH: AuthService + Send + Sync + 'static,
    GL: AuthService + Send + Sync + 'static,
    UR: UserRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    TR: TokenRepository + Send + Sync,
    RR: RefreshTokenRepository + Send + Sync,
    TS: TokenService + Send + Sync,
    GH::Error: std::error::Error + Send + Sync + 'static,
    GL::Error: std::error::Error + Send + Sync + 'static,
    <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    <RR as RefreshTokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    TS::Error: std::error::Error + Send + Sync + 'static,
{
    fn generate_start_url(&self, provider: Provider) -> Result<String, LoginError> {
        let auth_service = self.auth_factory.get_auth_service(provider);
        Ok(auth_service.generate_authorize_url())
    }

    async fn login(
        &self,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<LoginResponse, LoginError> {
        // Step 1: Exchange code for tokens and profile
        let (tokens, profile) = self.exchange_code_for_profile(provider, code, redirect_uri).await?;

        // Step 2: Validate email requirement
        let email = profile.email
            .as_ref()
            .ok_or_else(|| LoginError::AuthError("Email is required from OAuth provider".to_string()))?
            .clone();

        // Step 3: Find or create user
        let user = self.find_or_create_user(&profile, &email).await?;

        // Step 4: Save provider tokens
        self.save_provider_tokens(user.id, provider, profile.id, tokens).await?;

        // Step 5: Generate authentication tokens
        let (jwt_token, refresh_token) = self.generate_auth_tokens(user.id).await?;

        // Step 6: Calculate expiration and return response
        let expires_in = (jwt_token.expires_at - Utc::now())
            .num_seconds()
            .max(0) as u64;

        Ok(LoginResponse {
            user,
            email,
            access_token: jwt_token.token,
            expires_in,
            refresh_token: refresh_token.token,
        })
    }
} 