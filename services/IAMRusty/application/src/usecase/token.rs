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
        let snapshot = self
            .signing_key_registry
            .jwks_publication_snapshot()
            .await
            .map_err(|e| TokenError::RepositoryError(e.to_string()))?;

        // The writer registry is authoritative, including an empty result. Bootstrapping
        // belongs to setup, never to a publisher that could resurrect revoked keys.
        if snapshot.revision == 0
            || snapshot.access_token_expiration_seconds != self.access_token_expiration_seconds
        {
            return Err(TokenError::RepositoryError(
                "invalid signing publication policy".into(),
            ));
        }
        Ok(snapshot.publication.into_jwks())
    }
}

#[cfg(test)]
mod tests {
    use super::{TokenUseCase, TokenUseCaseImpl};
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
            provider_key_version: None,
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

    #[test]
    fn jwks_metadata_is_registry_truth_and_last_revocation_is_empty() {
        let pem = include_str!("../../../config/keys/test-platform.pub");
        let mut key = signing_key("epoch", pem);
        key.status = SigningKeyStatus::Pending;
        let jwks = JwkSet::from_registry_keys(&[key.clone()]);
        let json = serde_json::to_value(&jwks).unwrap();
        assert_eq!(json["keys"][0]["status"], "pending");
        assert_eq!(json["keys"][0]["trust_scope"], "organization");
        assert_eq!(
            json["keys"][0]["organization_id"],
            key.organization_id.unwrap().to_string()
        );
        assert!(json["keys"][0].get("credential_ref").is_none());
        key.status = SigningKeyStatus::Revoked;
        assert!(JwkSet::from_registry_keys(&[key]).keys.is_empty());
        assert!(JwkSet::from_registry_keys(&[]).keys.is_empty());
    }

    struct Registry {
        fail: bool,
        keys: Vec<SigningKey>,
    }
    #[async_trait::async_trait]
    impl iam_domain::port::repository::SigningKeyRegistry for Registry {
        type Error = iam_domain::error::DomainError;
        async fn jwks_publication_snapshot(
            &self,
        ) -> Result<iam_domain::entity::signing_key::SigningKeyPublicationSnapshot, Self::Error>
        {
            if self.fail {
                return Err(iam_domain::error::DomainError::RepositoryError(
                    "unavailable".into(),
                ));
            }
            iam_domain::entity::signing_key::SigningKeyPublicationSnapshot::from_fixture_keys(
                self.keys.clone(),
                Utc::now(),
                900,
            )
        }
        async fn confirm_active_for_emission(&self, _: &SigningKey) -> Result<bool, Self::Error> {
            panic!("JWKS publication must not invoke an emission fence")
        }
        async fn insert(&self, _: &SigningKey) -> Result<(), Self::Error> {
            unreachable!()
        }
        async fn update(&self, _: &SigningKey) -> Result<(), Self::Error> {
            unreachable!()
        }
        async fn replace_active_organization_key(
            &self,
            _: &SigningKey,
            _: Option<&str>,
        ) -> Result<SigningKey, Self::Error> {
            unreachable!()
        }
        async fn revoke_organization_keys(&self, _: Uuid) -> Result<Vec<SigningKey>, Self::Error> {
            unreachable!()
        }
        async fn find_by_kid(&self, _: &str) -> Result<Option<SigningKey>, Self::Error> {
            unreachable!()
        }
        async fn find_active_platform_key(&self) -> Result<Option<SigningKey>, Self::Error> {
            unreachable!()
        }
        async fn find_by_organization(&self, _: Uuid) -> Result<Vec<SigningKey>, Self::Error> {
            unreachable!()
        }
        async fn find_by_issuer(&self, _: &str) -> Result<Vec<SigningKey>, Self::Error> {
            unreachable!()
        }
        async fn list_jwks_keys(&self) -> Result<Vec<SigningKey>, Self::Error> {
            if self.fail {
                Err(iam_domain::error::DomainError::RepositoryError(
                    "unavailable".into(),
                ))
            } else {
                Ok(self.keys.clone())
            }
        }
    }
    struct Bootstrap;
    #[async_trait::async_trait]
    impl iam_domain::service::RefreshTokenService for Bootstrap {
        async fn refresh_token(
            &self,
            _: String,
        ) -> Result<iam_domain::service::RefreshTokenResponse, iam_domain::error::DomainError>
        {
            unreachable!()
        }
        async fn revoke_token(&self, _: String) -> Result<(), iam_domain::error::DomainError> {
            unreachable!()
        }
        async fn revoke_all_tokens(&self, _: Uuid) -> Result<u64, iam_domain::error::DomainError> {
            unreachable!()
        }
        fn get_jwks(&self) -> JwkSet {
            panic!("publisher must never resurrect bootstrap")
        }
    }
    struct Identities;
    #[async_trait::async_trait]
    impl iam_domain::port::repository::IdentityRepository for Identities {
        type Error = iam_domain::error::DomainError;
        async fn find_by_issuer_subject(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Option<iam_domain::entity::identity::Identity>, Self::Error> {
            unreachable!()
        }
        async fn find_by_user_id(
            &self,
            _: Uuid,
        ) -> Result<Vec<iam_domain::entity::identity::Identity>, Self::Error> {
            unreachable!()
        }
        async fn create(
            &self,
            _: &iam_domain::entity::identity::Identity,
        ) -> Result<iam_domain::entity::identity::Identity, Self::Error> {
            unreachable!()
        }
        async fn ensure_platform_identity(
            &self,
            _: Uuid,
            _: &str,
        ) -> Result<iam_domain::entity::identity::Identity, Self::Error> {
            unreachable!()
        }
        async fn ensure_organization_managed_identity(
            &self,
            _: Uuid,
            _: &str,
        ) -> Result<iam_domain::entity::identity::Identity, Self::Error> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn empty_writer_snapshot_is_success_but_writer_failure_is_not_empty_or_bootstrap() {
        use std::sync::Arc;
        for fail in [false, true] {
            let publisher = TokenUseCaseImpl::new(
                Arc::new(Bootstrap),
                Arc::new(Registry { fail, keys: vec![] }),
                Arc::new(Identities),
                "https://iam.example",
            );
            let result = publisher.get_jwks().await;
            if fail {
                assert!(result.is_err());
            } else {
                assert!(result.unwrap().keys.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn last_key_revocation_is_empty_but_corrupt_or_duplicate_registry_is_an_error() {
        use std::sync::Arc;
        let pem = include_str!("../../../config/keys/test-platform.pub");
        let mut revoked = signing_key("epoch", pem);
        revoked.status = SigningKeyStatus::Revoked;
        let publisher = TokenUseCaseImpl::new(
            Arc::new(Bootstrap),
            Arc::new(Registry {
                fail: false,
                keys: vec![revoked],
            }),
            Arc::new(Identities),
            "https://iam.example",
        );
        assert!(publisher.get_jwks().await.unwrap().keys.is_empty());
        let mut good = signing_key("epoch", pem);
        good.kid = iam_domain::entity::signing_key::opaque_kid();
        let positive = TokenUseCaseImpl::new(
            Arc::new(Bootstrap),
            Arc::new(Registry {
                fail: false,
                keys: vec![good.clone()],
            }),
            Arc::new(Identities),
            "https://iam.example",
        );
        assert_eq!(positive.get_jwks().await.unwrap().keys[0].kid, good.kid);
        let mut corrupt = good.clone();
        corrupt.public_key = "garbage".into();
        let mut unbound = good.clone();
        unbound.organization_id = None;
        for keys in [vec![good.clone(), good], vec![corrupt], vec![unbound]] {
            let publisher = TokenUseCaseImpl::new(
                Arc::new(Bootstrap),
                Arc::new(Registry { fail: false, keys }),
                Arc::new(Identities),
                "https://iam.example",
            );
            assert!(publisher.get_jwks().await.is_err());
        }
    }

    #[tokio::test]
    async fn mismatched_materialized_policy_is_an_error_not_a_second_pem_publication() {
        use std::sync::Arc;
        let publisher = TokenUseCaseImpl::with_expiration(
            Arc::new(Bootstrap),
            Arc::new(Registry {
                fail: false,
                keys: vec![],
            }),
            Arc::new(Identities),
            "https://iam.example",
            173,
        );
        assert!(publisher.get_jwks().await.is_err());
    }
}
