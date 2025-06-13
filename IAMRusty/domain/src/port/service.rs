use crate::entity::{
    provider::{ProviderTokens, ProviderUserProfile},
    token::{JwkSet, TokenClaims, JwtToken, RefreshToken},
    registration_token::RegistrationTokenClaims,
};
use uuid::Uuid;
use crate::error::DomainError;
use async_trait::async_trait;

/// Provider OAuth2 client interface
#[async_trait::async_trait]
pub trait ProviderOAuth2Client {
    /// Generate a URL to start the OAuth2 flow
    fn generate_authorize_url(&self) -> String;

    /// Exchange an authorization code for tokens
    async fn exchange_code(&self, code: &str) -> Result<ProviderTokens, DomainError>;

    /// Get user profile from the provider
    async fn get_user_profile(&self, tokens: &ProviderTokens) -> Result<ProviderUserProfile, DomainError>;
}

/// JWT token encoder/decoder
pub trait JwtTokenEncoder: Send + Sync {
    /// Encode a token with the given claims
    fn encode(&self, claims: &TokenClaims) -> Result<String, DomainError>;

    /// Decode a token and validate its signature
    fn decode(&self, token: &str) -> Result<TokenClaims, DomainError>;

    /// Get the JSON Web Key Set (JWKS) for token verification
    fn jwks(&self) -> JwkSet;
}

/// Registration token service for managing RSA-signed registration tokens
pub trait RegistrationTokenService: Send + Sync {
    /// Generate a registration token for email/password flow
    fn generate_registration_token(&self, user_id: Uuid, email: String) -> Result<String, DomainError>;

    /// Generate a registration token for OAuth flow
    fn generate_oauth_registration_token(&self, user_id: Uuid, email: String, provider_info: crate::entity::registration_token::ProviderInfo) -> Result<String, DomainError>;

    /// Validate and decode a registration token
    fn validate_registration_token(&self, token: &str) -> Result<RegistrationTokenClaims, DomainError>;

    /// Check if a registration token is valid and not expired
    fn is_registration_token_valid(&self, token: &str) -> bool;
}

/// Token service for handling JWT tokens
#[async_trait]
pub trait AuthTokenService: Send + Sync {
    /// Error type returned by this service
    type Error: std::error::Error + Send + Sync + 'static;

    /// Generate an access token for a user
    async fn generate_access_token(&self, user_id: Uuid) -> Result<JwtToken, Self::Error>;

    /// Generate a refresh token for a user
    async fn generate_refresh_token(&self, user_id: Uuid) -> Result<RefreshToken, Self::Error>;

    /// Validate an access token and extract the user ID
    async fn validate_access_token(&self, token: &str) -> Result<Uuid, Self::Error>;

    /// Validate a refresh token
    async fn validate_refresh_token(&self, token: &str) -> Result<RefreshToken, Self::Error>;
} 