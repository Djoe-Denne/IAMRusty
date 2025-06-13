//! OAuth use case module for OAuth provider authentication

use async_trait::async_trait;
use domain::entity::{
    provider::Provider,
    user::User,
    user_email::UserEmail,
    events::{DomainEvent, UserLoggedInEvent},
};
use domain::port::{
    repository::{TokenRepository, UserRepository, UserEmailRepository, RefreshTokenRepository},
    service::{AuthTokenService, RegistrationTokenService},
    event_publisher::EventPublisher,
};
use crate::auth::OAuthService;
use crate::usecase::factory::OAuthProviderFactory;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// OAuth use case error
#[derive(Debug, Error)]
pub enum OAuthError {
    /// Authentication error from OAuth provider
    #[error("Authentication error: {0}")]
    AuthError(String),

    /// Database error from repository
    #[error("Database error: {0}")]
    DbError(Box<dyn std::error::Error + Send + Sync>),

    /// Token service error
    #[error("Token service error: {0}")]
    TokenError(Box<dyn std::error::Error + Send + Sync>),

    /// Provider not configured
    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),

    /// Event publishing error
    #[error("Event publishing error: {0}")]
    EventPublishingError(String),
}

/// OAuth login response
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthLoginResponse {
    /// User data
    pub user: User,
    /// Primary email address from UserEmail entity
    pub email: String,
    /// JWT access token (our internal token for user authentication)
    pub access_token: String,
    /// Access token expiration in seconds
    pub expires_in: u64,
    /// Refresh token for getting new access tokens
    pub refresh_token: String,
}

/// OAuth registration response for new users requiring username
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthRegistrationResponse {
    /// User data (incomplete)
    pub user: User,
    /// Primary email address from UserEmail entity
    pub email: String,
    /// Registration token for completing registration with username
    pub registration_token: String,
    /// Provider information for UI display and suggestions
    pub provider_info: ProviderInfo,
}

/// Provider information from OAuth flow
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider email
    pub email: String,
    /// Provider avatar URL
    pub avatar: Option<String>,
    /// Suggested username based on provider data
    pub suggested_username: String,
}

/// OAuth result enum that can be either login or registration
#[derive(Debug, Serialize, Deserialize)]
pub enum OAuthResult {
    /// User successfully logged in
    Login(OAuthLoginResponse),
    /// User needs to complete registration
    Registration(OAuthRegistrationResponse),
}

/// OAuth use case interface
#[async_trait]
pub trait OAuthUseCase: Send + Sync {
    /// Generate OAuth authorization URL for login flow
    fn generate_start_url(&self, provider: Provider) -> Result<String, OAuthError>;

    /// Exchange authorization code for tokens and login user or start registration
    /// This handles the OAuth callback and:
    /// 1. Exchanges the authorization code for provider tokens
    /// 2. Gets user profile from provider
    /// 3. Creates or updates user in our system
    /// 4. Stores provider tokens for future API calls
    /// 5. Either issues JWT tokens (existing user) or registration token (new user)
    async fn oauth_login(
        &self,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<OAuthResult, OAuthError>;
}

/// OAuth use case implementation
pub struct OAuthUseCaseImpl<GH, GL, UR, UER, TR, RR, TS, RTS, EP> 
where
    GH: OAuthService + 'static,
    GL: OAuthService + 'static,
    UR: UserRepository,
    UER: UserEmailRepository,
    TR: TokenRepository,
    RR: RefreshTokenRepository,
    TS: AuthTokenService,
    RTS: RegistrationTokenService,
    EP: EventPublisher,
{
    auth_factory: Arc<OAuthProviderFactory<GH, GL>>,
    user_repo: Arc<UR>,
    user_email_repo: Arc<UER>,
    token_repo: Arc<TR>,
    refresh_token_repo: Arc<RR>,
    token_service: Arc<TS>,
    registration_token_service: Arc<RTS>,
    event_publisher: Arc<EP>,
}

impl<GH, GL, UR, UER, TR, RR, TS, RTS, EP> OAuthUseCaseImpl<GH, GL, UR, UER, TR, RR, TS, RTS, EP>
where
    GH: OAuthService + 'static,
    GL: OAuthService + 'static,
    UR: UserRepository,
    UER: UserEmailRepository,
    TR: TokenRepository,
    RR: RefreshTokenRepository,
    TS: AuthTokenService,
    RTS: RegistrationTokenService,
    EP: EventPublisher,
{
    /// Create a new OAuthUseCaseImpl
    pub fn new(
        github_auth: Arc<GH>,
        gitlab_auth: Arc<GL>,
        user_repo: Arc<UR>,
        user_email_repo: Arc<UER>,
        token_repo: Arc<TR>,
        refresh_token_repo: Arc<RR>,
        token_service: Arc<TS>,
        registration_token_service: Arc<RTS>,
        event_publisher: Arc<EP>,
    ) -> Self {
        let auth_factory = Arc::new(OAuthProviderFactory::new(github_auth, gitlab_auth));
        
        Self {
            auth_factory,
            user_repo,
            user_email_repo,
            token_repo,
            refresh_token_repo,
            token_service,
            registration_token_service,
            event_publisher,
        }
    }

    /// Exchange authorization code for provider tokens and user profile
    /// This gets the provider tokens (for calling the provider's API) and user profile
    async fn exchange_code_for_profile(
        &self,
        provider: Provider,
        code: String,
        redirect_uri: String,
    ) -> Result<(domain::entity::provider::ProviderTokens, domain::entity::provider::ProviderUserProfile), OAuthError>
    where
        GH: OAuthService,
        GL: OAuthService,
        <GH as OAuthService>::Error: std::error::Error + Send + Sync + 'static,
        <GL as OAuthService>::Error: std::error::Error + Send + Sync + 'static,
    {
        let auth_service = self.auth_factory.get_oauth_service(provider);
        
        auth_service
            .exchange_code(code, redirect_uri)
            .await
            .map_err(|e| OAuthError::AuthError(e.to_string()))
    }

    /// Find or create user based on OAuth profile
    async fn find_or_create_user(
        &self,
        profile: &domain::entity::provider::ProviderUserProfile,
        email: &str,
    ) -> Result<User, OAuthError>
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
            .map_err(|e| OAuthError::DbError(Box::new(e)))?;

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
    ) -> Result<User, OAuthError>
    where
        UR: UserRepository,
        <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        let user = self.user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| OAuthError::DbError(Box::new(e)))?
            .ok_or_else(|| OAuthError::DbError("User not found for email".into()))?;

        // Check if update is needed
        if user.username == Some(profile.username.clone()) && user.avatar_url == profile.avatar_url {
            return Ok(user);
        }

        // Update user profile
        let mut updated_user = user;
        updated_user.username = Some(profile.username.clone());
        updated_user.avatar_url = profile.avatar_url.clone();
        updated_user.updated_at = Utc::now();

        self.user_repo
            .update(updated_user)
            .await
            .map_err(|e| OAuthError::DbError(Box::new(e)))
    }

    /// Create new user with primary email (without username for OAuth)
    async fn create_new_user(
        &self,
        profile: &domain::entity::provider::ProviderUserProfile,
        email: &str,
    ) -> Result<User, OAuthError>
    where
        UR: UserRepository,
        UER: UserEmailRepository,
        <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
        <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        // Create new user without username (OAuth users need to complete registration)
        let new_user = User::new_incomplete(
            profile.avatar_url.clone(),
        );

        let created_user = self.user_repo
            .create(new_user)
            .await
            .map_err(|e| OAuthError::DbError(Box::new(e)))?;

        // Create primary email for the new user
        let user_email = UserEmail::new_primary(
            created_user.id,
            email.to_string(),
            true, // OAuth emails are considered verified since they come from the provider
        );

        self.user_email_repo
            .create(user_email)
            .await
            .map_err(|e| OAuthError::DbError(Box::new(e)))?;

        Ok(created_user)
    }

    /// Save provider tokens for user
    /// These tokens allow our system to make API calls to the provider on behalf of the user
    async fn save_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
        provider_user_id: String,
        tokens: domain::entity::provider::ProviderTokens,
    ) -> Result<(), OAuthError>
    where
        TR: TokenRepository,
        <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        self.token_repo
            .save_provider_tokens(user_id, provider, provider_user_id, tokens)
            .await
            .map_err(|e| OAuthError::DbError(Box::new(e)))
    }

    /// Generate and store authentication tokens
    /// These are our internal JWT tokens used to authenticate the user in our system
    async fn generate_auth_tokens(
        &self,
        user_id: Uuid,
    ) -> Result<(domain::entity::token::JwtToken, domain::entity::token::RefreshToken), OAuthError>
    where
        TS: AuthTokenService,
        RR: RefreshTokenRepository,
        <TS as AuthTokenService>::Error: std::error::Error + Send + Sync + 'static,
        <RR as RefreshTokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        // Generate JWT access token (our internal token)
        let jwt_token = self.token_service
            .generate_access_token(user_id)
            .await
            .map_err(|e| OAuthError::TokenError(Box::new(e)))?;

        // Generate refresh token
        let refresh_token = self.token_service
            .generate_refresh_token(user_id)
            .await
            .map_err(|e| OAuthError::TokenError(Box::new(e)))?;

        // Store refresh token in database
        self.refresh_token_repo
            .create(refresh_token.clone())
            .await
            .map_err(|e| OAuthError::DbError(Box::new(e)))?;
        
        Ok((jwt_token, refresh_token))
    }

    /// Handle login for complete users (have username)
    async fn handle_complete_user_login(
        &self,
        user: User,
        email: String,
    ) -> Result<OAuthResult, OAuthError>
    where
        TS: AuthTokenService,
        RR: RefreshTokenRepository,
        EP: EventPublisher,
        <TS as AuthTokenService>::Error: std::error::Error + Send + Sync + 'static,
        <RR as RefreshTokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        // Generate authentication tokens (our internal JWT tokens)
        let (jwt_token, refresh_token) = self.generate_auth_tokens(user.id).await?;

        // Publish user logged in event
        let event = DomainEvent::UserLoggedIn(UserLoggedInEvent::new(
            user.id,
            email.clone(),
            format!("oauth_login"),
        ));

        if let Err(e) = self.event_publisher.publish(event).await {
            tracing::warn!("Failed to publish UserLoggedIn event: {}", e);
            // Don't fail the login for event publishing errors
        }

        // Calculate expiration and return response
        let expires_in = (jwt_token.expires_at - Utc::now())
            .num_seconds()
            .max(0) as u64;

        Ok(OAuthResult::Login(OAuthLoginResponse {
            user,
            email,
            access_token: jwt_token.token,
            expires_in,
            refresh_token: refresh_token.token,
        }))
    }

    /// Handle registration for incomplete users (no username)
    async fn handle_incomplete_user_registration(
        &self,
        user: User,
        email: String,
        profile: &domain::entity::provider::ProviderUserProfile,
    ) -> Result<OAuthResult, OAuthError>
    where
        RTS: RegistrationTokenService,
    {
        // Create provider info for frontend
        let provider_info = ProviderInfo {
            email: profile.email.clone().unwrap(),
            avatar: profile.avatar_url.clone(),
            suggested_username: profile.username.clone(),
        };

        // Generate OAuth registration token with provider info
        let domain_provider_info = domain::entity::registration_token::ProviderInfo {
            email: provider_info.email.clone(),
            avatar: provider_info.avatar.clone(), 
            suggested_username: provider_info.suggested_username.clone(),
        };
        
        let registration_token = self.registration_token_service
            .generate_oauth_registration_token(user.id, email.clone(), domain_provider_info)
            .map_err(|e| OAuthError::TokenError(Box::new(e)))?;

        Ok(OAuthResult::Registration(OAuthRegistrationResponse {
            user,
            email,
            registration_token,
            provider_info,
        }))
    }
}

#[async_trait]
impl<GH, GL, UR, UER, TR, RR, TS, RTS, EP> OAuthUseCase for OAuthUseCaseImpl<GH, GL, UR, UER, TR, RR, TS, RTS, EP>
where
    GH: OAuthService + Send + Sync + 'static,
    GL: OAuthService + Send + Sync + 'static,
    UR: UserRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    TR: TokenRepository + Send + Sync,
    RR: RefreshTokenRepository + Send + Sync,
    TS: AuthTokenService + Send + Sync,
    RTS: RegistrationTokenService + Send + Sync,
    EP: EventPublisher + Send + Sync,
    GH::Error: std::error::Error + Send + Sync + 'static,
    GL::Error: std::error::Error + Send + Sync + 'static,
    <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    <RR as RefreshTokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    TS::Error: std::error::Error + Send + Sync + 'static,
{
    fn generate_start_url(&self, provider: Provider) -> Result<String, OAuthError> {
        let auth_service = self.auth_factory.get_oauth_service(provider);
        Ok(auth_service.generate_authorize_url())
    }

    async fn oauth_login(
        &self,
        provider: Provider,  
        code: String,
        redirect_uri: String,
    ) -> Result<OAuthResult, OAuthError> {
        // Step 1: Exchange code for provider tokens and profile
        let (provider_tokens, profile) = self.exchange_code_for_profile(provider, code, redirect_uri).await?;

        // Step 2: Validate email requirement
        let email = profile.email
            .as_ref()
            .ok_or_else(|| OAuthError::AuthError("Email is required from OAuth provider".to_string()))?
            .clone();

        // Step 3: Find or create user
        let user = self.find_or_create_user(&profile, &email).await?;

        // Step 4: Save provider tokens (for future API calls to the provider)
        self.save_provider_tokens(user.id, provider, profile.id.clone(), provider_tokens).await?;

        // Step 5: Check if user is complete (has username) or needs registration
        if user.username.is_some() {
            // Existing complete user - proceed with login
            self.handle_complete_user_login(user, email).await
        } else {
            // New incomplete user - require registration
            self.handle_incomplete_user_registration(user, email, &profile).await
        }
    }
} 