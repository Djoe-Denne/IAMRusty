//! OAuth use case module for OAuth provider authentication

use async_trait::async_trait;
use iam_domain::entity::oauth_transaction::{
    BeginOAuthTransaction, BegunOAuthTransaction, ConsumeOAuthTransaction,
    ConsumedOAuthTransaction, OAuthTransaction, OAuthTransactionError,
};
use iam_domain::entity::{provider::Provider, user::User};
use iam_domain::error::DomainError;
use iam_domain::port::{
    repository::{IdentityRepository, TokenRepository, UserEmailRepository, UserRepository},
    service::{AuthTokenService, RegistrationTokenService},
};
use iam_domain::service::oauth_service::OAuthService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// OAuth use case error
#[derive(Debug, Error)]
pub enum OAuthError {
    /// Domain service error
    #[error("Domain service error: {0}")]
    DomainError(#[from] DomainError),
    #[error("{0}")]
    Transaction(#[from] OAuthTransactionError),
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
    /// User's primary email
    pub email: String,
    /// JWT access token (our internal token for user authentication)
    pub access_token: String,
    /// Access token expiration time in seconds
    pub expires_in: u64,
    /// Refresh token for getting new access tokens
    pub refresh_token: String,
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
    async fn begin_oauth_transaction(
        &self,
        input: BeginOAuthTransaction,
    ) -> Result<BegunOAuthTransaction, OAuthError>;
    async fn consume_oauth_transaction(
        &self,
        input: ConsumeOAuthTransaction,
    ) -> Result<ConsumedOAuthTransaction, OAuthError>;
    /// Generate OAuth authorization URL for login flow
    ///
    /// # Errors
    ///
    /// Returns [`OAuthError`] if the authorization URL cannot be generated.
    async fn generate_start_url(
        &self,
        provider: Provider,
        redirect_uri: String,
        state: String,
        begun: &BegunOAuthTransaction,
    ) -> Result<String, OAuthError>;

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
        redirect_uri: String,
        consumed: ConsumedOAuthTransaction,
    ) -> Result<OAuthResponse, OAuthError>;
}

/// OAuth use case implementation - thin orchestration layer
pub struct OAuthUseCaseImpl<UR, TR, UER, RTS, TS>
where
    UR: UserRepository,
    TR: TokenRepository,
    UER: UserEmailRepository,
    RTS: RegistrationTokenService,
    TS: AuthTokenService,
{
    oauth: Arc<OAuthService<UR, TR, UER>>,
    registration_token: Arc<RTS>,
    token: Arc<TS>,
    identity_repo: Arc<dyn IdentityRepository<Error = DomainError>>,
    platform_issuer: String,
    session_writer: Option<Arc<dyn iam_domain::port::repository::AuthenticationSessionWriter>>,
    oauth_transaction_writer:
        Option<Arc<dyn iam_domain::port::repository::OAuthTransactionWriteRepository>>,
}

impl<UR, TR, UER, RTS, TS> OAuthUseCaseImpl<UR, TR, UER, RTS, TS>
where
    UR: UserRepository,
    TR: TokenRepository,
    UER: UserEmailRepository,
    RTS: RegistrationTokenService,
    TS: AuthTokenService,
{
    /// Create a new `OAuthUseCaseImpl`
    pub fn new(
        oauth: Arc<OAuthService<UR, TR, UER>>,
        registration_token: Arc<RTS>,
        token: Arc<TS>,
        identity_repo: Arc<dyn IdentityRepository<Error = DomainError>>,
        platform_issuer: impl Into<String>,
    ) -> Self {
        Self {
            oauth,
            registration_token,
            token,
            identity_repo,
            platform_issuer: platform_issuer.into(),
            session_writer: None,
            oauth_transaction_writer: None,
        }
    }
    #[must_use]
    pub fn with_session_writer(
        mut self,
        writer: Arc<dyn iam_domain::port::repository::AuthenticationSessionWriter>,
    ) -> Self {
        self.session_writer = Some(writer);
        self
    }

    #[must_use]
    pub fn with_oauth_transaction_writer(
        mut self,
        writer: Arc<dyn iam_domain::port::repository::OAuthTransactionWriteRepository>,
    ) -> Self {
        self.oauth_transaction_writer = Some(writer);
        self
    }
}

#[async_trait]
impl<UR, TR, UER, RTS, TS> OAuthUseCase for OAuthUseCaseImpl<UR, TR, UER, RTS, TS>
where
    UR: UserRepository + Send + Sync,
    TR: TokenRepository + Send + Sync,
    UER: UserEmailRepository + Send + Sync,
    RTS: RegistrationTokenService + Send + Sync,
    TS: AuthTokenService + Send + Sync,
    <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    TS::Error: std::error::Error + Send + Sync + 'static,
{
    async fn begin_oauth_transaction(
        &self,
        input: BeginOAuthTransaction,
    ) -> Result<BegunOAuthTransaction, OAuthError> {
        let writer = self
            .oauth_transaction_writer
            .as_ref()
            .ok_or(OAuthTransactionError::Storage)?;
        Ok(OAuthTransaction::begin(writer.as_ref(), input).await?)
    }

    async fn consume_oauth_transaction(
        &self,
        input: ConsumeOAuthTransaction,
    ) -> Result<ConsumedOAuthTransaction, OAuthError> {
        let writer = self
            .oauth_transaction_writer
            .as_ref()
            .ok_or(OAuthTransactionError::Storage)?;
        Ok(ConsumedOAuthTransaction::consume(writer.as_ref(), input).await?)
    }

    async fn generate_start_url(
        &self,
        provider: Provider,
        redirect_uri: String,
        state: String,
        begun: &BegunOAuthTransaction,
    ) -> Result<String, OAuthError> {
        self.oauth
            .generate_authorize_url(&provider, &redirect_uri, &state, begun)
            .await
            .map_err(Into::into)
    }

    async fn oauth_login(
        &self,
        provider: Provider,
        code: String,
        redirect_uri: String,
        consumed: ConsumedOAuthTransaction,
    ) -> Result<OAuthResponse, OAuthError> {
        // Delegate to domain service - note: we ignore the JWT token since we'll generate proper tokens
        let (user, _jwt_token, email) = self
            .oauth
            .process_callback(&provider, &code, &redirect_uri, &consumed)
            .await?;

        // Check if user is complete (has username) or needs registration
        if user.username.is_none() {
            // User needs to complete registration - generate proper registration token
            let registration_token = self
                .registration_token
                .generate_oauth_registration_token(
                    user.id,
                    email.clone(),
                    iam_domain::entity::registration_token::ProviderInfo {
                        email: email.clone(),
                        suggested_username: user.username.clone().unwrap_or_else(String::new), // Default to empty string if no username
                        avatar: user.avatar_url.clone(),
                    },
                )
                .await
                .map_err(OAuthError::DomainError)?;

            return Ok(OAuthResponse::Registration(OAuthRegistrationResponse {
                registration_token,
                provider_info: ProviderInfo {
                    email,
                    username: user.username.clone(),
                    avatar_url: user.avatar_url.clone(),
                },
            }));
        }

        // User is complete - generate proper access and refresh tokens
        let access_token = self
            .token
            .generate_access_token(user.id)
            .await
            .map_err(|e| OAuthError::DomainError(DomainError::TokenServiceError(e.to_string())))?;

        let refresh_token = self
            .token
            .generate_refresh_token(user.id)
            .await
            .map_err(|e| OAuthError::DomainError(DomainError::TokenServiceError(e.to_string())))?;

        // Calculate expires_in from the actual token expiration
        let now = chrono::Utc::now();
        let expires_in =
            u64::try_from((access_token.expires_at - now).num_seconds().max(0)).unwrap_or(0);

        self.identity_repo
            .ensure_platform_identity(user.id, &self.platform_issuer)
            .await
            .map_err(OAuthError::DomainError)?;

        self.session_writer
            .as_ref()
            .ok_or_else(|| DomainError::RepositoryError("session writer not configured".into()))?
            .issue(refresh_token.clone(), user.password_hash.clone())
            .await?;

        Ok(OAuthResponse::Login(OAuthLoginResponse {
            user,
            email,
            access_token: access_token.token,
            expires_in,
            refresh_token: refresh_token.token,
        }))
    }
}
