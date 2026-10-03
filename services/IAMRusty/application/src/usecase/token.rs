use async_trait::async_trait;
use iam_domain::entity::token::JwkSet;
use iam_domain::error::DomainError;
use iam_domain::port::repository::{IdentityRepository, SigningKeyRegistry};
use iam_domain::service::{
    RefreshTokenResponse as DomainRefreshTokenResponse, RefreshTokenService,
};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Token usecase error
#[derive(Debug, Error)]
pub enum TokenError {
    /// Domain service error
    #[error("Domain service error: {0}")]
    DomainError(#[from] DomainError),

    /// Repository error
    #[error("Repository error: {0}")]
    RepositoryError(String),

    /// Token service error
    #[error("Token service error: {0}")]
    TokenServiceError(String),

    /// Token not found
    #[error("Token not found")]
    TokenNotFound,

    /// Token invalid
    #[error("Token invalid")]
    TokenInvalid,

    /// Token expired
    #[error("Token expired")]
    TokenExpired,
}

/// Response for token refresh
#[derive(Debug)]
pub struct RefreshTokenResponse {
    /// New access token
    pub access_token: String,
    /// Access token expiration time in seconds
    pub expires_in: u64,
    /// New refresh token (replaces the old one)
    pub refresh_token: String,
    /// Refresh token expiration time in seconds
    pub refresh_expires_in: u64,
    /// User id bound to the refreshed session
    pub user_id: Uuid,
}

impl From<DomainRefreshTokenResponse> for RefreshTokenResponse {
    fn from(domain_response: DomainRefreshTokenResponse) -> Self {
        Self {
            access_token: domain_response.access_token,
            expires_in: domain_response.expires_in,
            refresh_token: domain_response.refresh_token,
            refresh_expires_in: domain_response.refresh_expires_in,
            user_id: domain_response.user_id,
        }
    }
}

/// Token use case interface
#[async_trait]
pub trait TokenUseCase: Send + Sync {
    /// Refresh an access token using a refresh token
    async fn refresh_token(
        &self,
        refresh_token: String,
    ) -> Result<RefreshTokenResponse, TokenError>;

    /// Revoke a refresh token
    async fn revoke_token(&self, refresh_token: String) -> Result<(), TokenError>;

    /// Revoke all refresh tokens for a user
    async fn revoke_all_tokens(&self, user_id: Uuid) -> Result<u64, TokenError>;

    /// Get the JSON Web Key Set (JWKS) for token verification
    async fn get_jwks(&self) -> Result<JwkSet, TokenError>;
}

/// Token use case implementation - thin orchestration layer
pub struct TokenUseCaseImpl<RTS>
where
    RTS: RefreshTokenService,
{
    refresh_token_service: Arc<RTS>,
    signing_key_registry: Arc<dyn SigningKeyRegistry<Error = DomainError>>,
    identity_repo: Arc<dyn IdentityRepository<Error = DomainError>>,
    platform_issuer: String,
    access_token_expiration_seconds: u64,
}

impl<RTS> TokenUseCaseImpl<RTS>
where
    RTS: RefreshTokenService,
{
    /// Create a new `TokenUseCaseImpl`
    pub fn new(
        refresh_token_service: Arc<RTS>,
        signing_key_registry: Arc<dyn SigningKeyRegistry<Error = DomainError>>,
        identity_repo: Arc<dyn IdentityRepository<Error = DomainError>>,
        platform_issuer: impl Into<String>,
    ) -> Self {
        Self::with_expiration(
            refresh_token_service,
            signing_key_registry,
            identity_repo,
            platform_issuer,
            900,
        )
    }

    /// Create with explicit access-token TTL (retiring JWKS window).
    pub fn with_expiration(
        refresh_token_service: Arc<RTS>,
        signing_key_registry: Arc<dyn SigningKeyRegistry<Error = DomainError>>,
        identity_repo: Arc<dyn IdentityRepository<Error = DomainError>>,
        platform_issuer: impl Into<String>,
        access_token_expiration_seconds: u64,
    ) -> Self {
        Self {
            refresh_token_service,
            signing_key_registry,
            identity_repo,
            platform_issuer: platform_issuer.into(),
            access_token_expiration_seconds,
        }
    }
}

#[async_trait]
impl<RTS> TokenUseCase for TokenUseCaseImpl<RTS>
where
    RTS: RefreshTokenService + Send + Sync,
{
    async fn refresh_token(
        &self,
        refresh_token: String,
    ) -> Result<RefreshTokenResponse, TokenError> {
        let domain_response = self
            .refresh_token_service
            .refresh_token(refresh_token)
            .await?;

        self.identity_repo
            .ensure_platform_identity(domain_response.user_id, &self.platform_issuer)
            .await
            .map_err(TokenError::DomainError)?;

        Ok(RefreshTokenResponse::from(domain_response))
    }

    async fn revoke_token(&self, refresh_token: String) -> Result<(), TokenError> {
        self.refresh_token_service
            .revoke_token(refresh_token)
            .await
            .map_err(Into::into)
    }

    async fn revoke_all_tokens(&self, user_id: Uuid) -> Result<u64, TokenError> {
        self.refresh_token_service
            .revoke_all_tokens(user_id)
            .await
            .map_err(Into::into)
    }

    async fn get_jwks(&self) -> Result<JwkSet, TokenError> {
        let keys = self
            .signing_key_registry
            .list_jwks_keys()
            .await
            .map_err(|e| TokenError::RepositoryError(e.to_string()))?;

        if keys.is_empty() {
            // Bootstrap cache fallback when the registry has not been seeded yet.
            return Ok(self.refresh_token_service.get_jwks());
        }

        let keys = iam_domain::entity::signing_key::filter_jwks_publication_keys(
            keys,
            self.access_token_expiration_seconds,
        );
        Ok(JwkSet::from_registry_keys(&keys))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use iam_domain::entity::signing_key::{
        SigningKey, SigningKeyStatus, SigningProviderType, TrustScope,
    };
    use iam_domain::entity::token::JwkSet;
    use uuid::Uuid;

    fn signing_key(kid: &str, public_key: &str) -> SigningKey {
        let now = Utc::now();
        SigningKey {
            id: Uuid::new_v4(),
            kid: kid.to_string(),
            algorithm: "RS256".to_string(),
            trust_scope: TrustScope::Organization,
            issuer: "http://127.0.0.1/iam/orgs/acme".to_string(),
            provider_type: SigningProviderType::PemFile,
            provider_key_ref: "config/keys/org.pem".to_string(),
            credential_ref: None,
            public_key: public_key.to_string(),
            status: SigningKeyStatus::Active,
            organization_id: Some(Uuid::new_v4()),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn jwks_from_registry_keys_skips_garbage_public_key() {
        let valid_pem = include_str!("../../../config/keys/test-platform.pub");
        let keys = [
            signing_key("garbage-kid", "not-a-pem"),
            signing_key("good-kid", valid_pem),
        ];
        let jwks = JwkSet::from_registry_keys(&keys);
        assert_eq!(jwks.keys.len(), 1);
        assert_eq!(jwks.keys[0].kid, "good-kid");
    }

    #[test]
    fn jwks_from_registry_keys_skips_non_rs256_algorithm() {
        let valid_pem = include_str!("../../../config/keys/test-platform.pub");
        let mut hmac = signing_key("hmac-kid", valid_pem);
        hmac.algorithm = "HS256".to_string();
        let mut good = signing_key("good-kid", valid_pem);
        good.algorithm = "RS256".to_string();
        let jwks = JwkSet::from_registry_keys(&[hmac, good]);
        assert_eq!(jwks.keys.len(), 1);
        assert_eq!(jwks.keys[0].kid, "good-kid");
    }
}
