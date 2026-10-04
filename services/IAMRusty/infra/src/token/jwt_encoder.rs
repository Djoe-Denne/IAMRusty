//! JWT encoder with SigningProvider RS256 path (ADR-0304).

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
use iam_domain::entity::registration_token::RegistrationTokenClaims;
use iam_domain::entity::signing_key::ACCESS_TOKEN_TYP;
use iam_domain::entity::token::{
    Jwk, JwkSet, JwtKeyPair, JwtToken, RefreshToken, TokenClaims, DEFAULT_JWT_AUDIENCE,
    DEFAULT_JWT_ISSUER,
};
use iam_domain::error::DomainError;
use iam_domain::port::service::{AuthTokenService, JwtTokenEncoder};
use iam_domain::port::SigningProvider;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use rsa::{pkcs1v15::Pkcs1v15Sign, traits::PublicKeyParts};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error};
use uuid::Uuid;

/// JWT token service error
#[derive(Debug, Error)]
pub enum TokenError {
    /// JWT encoding/decoding error
    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    /// Invalid token
    #[error("Invalid token")]
    InvalidToken,

    /// Token expired
    #[error("Token expired")]
    TokenExpired,

    /// Generic error
    #[error("Token error: {0}")]
    GenericError(String),
}

/// JWT algorithm configuration (local material; RS256 prefer SigningProvider).
#[derive(Clone)]
pub enum JwtAlgorithm {
    /// RSA256 with key pair (legacy / bootstrap; prefer [`SigningProvider`])
    RS256(JwtKeyPair),
    /// HMAC256 with secret (HS256 migration / registration tokens)
    HS256(String),
}

impl std::fmt::Debug for JwtAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RS256(key) => f.debug_tuple("RS256").field(&key.kid).finish(),
            Self::HS256(_) => f.write_str("HS256([redacted])"),
        }
    }
}

/// Unified JWT token service that handles both encoding/decoding and token management
#[derive(Clone)]
pub struct JwtTokenService {
    algorithm_config: JwtAlgorithm,
    access_token_expiration: u64,
    refresh_token_expiration: u64,
    issuer: String,
    audience: String,
    /// Optional RS256 SigningProvider used by both access and registration.
    signing_provider: Option<Arc<dyn SigningProvider>>,
    /// Opaque kid for SigningProvider-backed encode.
    signing_kid: Option<String>,
    /// Issuer bound to the signing key (must match JWK `iss`).
    signing_issuer: Option<String>,
    /// Cached JWKS built from registry / PEM public key.
    jwks_cache: Arc<JwkSet>,
    signing_registry:
        Option<Arc<dyn iam_domain::port::repository::SigningKeyRegistry<Error = DomainError>>>,
}

impl JwtTokenService {
    /// Create a new `JwtTokenService` with RSA256 keys
    #[must_use]
    pub fn with_rsa(key_pair: JwtKeyPair, access_token_expiration: u64) -> Self {
        Self {
            algorithm_config: JwtAlgorithm::RS256(key_pair),
            access_token_expiration,
            refresh_token_expiration: 2_592_000,
            issuer: String::new(),
            audience: String::new(),
            signing_provider: None,
            signing_kid: None,
            signing_issuer: None,
            jwks_cache: Arc::new(JwkSet { keys: vec![] }),
            signing_registry: None,
        }
    }

    /// Create a new `JwtTokenService` with HMAC256 secret (tests / dual-window only).
    #[must_use]
    pub fn with_hmac(secret: String, access_token_expiration: u64) -> Self {
        Self {
            algorithm_config: JwtAlgorithm::HS256(secret),
            access_token_expiration,
            refresh_token_expiration: 2_592_000,
            issuer: String::new(),
            audience: String::new(),
            signing_provider: None,
            signing_kid: None,
            signing_issuer: None,
            jwks_cache: Arc::new(JwkSet { keys: vec![] }),
            signing_registry: None,
        }
    }

    /// Create from algorithm + refresh expiration.
    #[must_use]
    pub fn with_refresh_expiration(
        algorithm_config: JwtAlgorithm,
        access_token_expiration: u64,
        refresh_token_expiration: u64,
    ) -> Self {
        Self {
            algorithm_config,
            access_token_expiration,
            refresh_token_expiration,
            issuer: String::new(),
            audience: String::new(),
            signing_provider: None,
            signing_kid: None,
            signing_issuer: None,
            jwks_cache: Arc::new(JwkSet { keys: vec![] }),
            signing_registry: None,
        }
    }

    /// Bind platform `iss` / `aud` claims used when minting access tokens.
    #[must_use]
    pub fn with_issuer_audience(
        mut self,
        issuer: impl Into<String>,
        audience: impl Into<String>,
    ) -> Self {
        self.issuer = issuer.into();
        self.audience = audience.into();
        self
    }

    /// Attach the shared SigningProvider for RS256 access and registration minting.
    #[must_use]
    pub fn with_signing_provider(
        mut self,
        provider: Arc<dyn SigningProvider>,
        kid: impl Into<String>,
        issuer: impl Into<String>,
        jwks: JwkSet,
    ) -> Self {
        self.signing_provider = Some(provider);
        self.signing_kid = Some(kid.into());
        self.signing_issuer = Some(issuer.into());
        self.jwks_cache = Arc::new(jwks);
        self
    }

    /// Opaque kid used when minting access tokens via [`SigningProvider`] (platform key).
    #[must_use]
    pub fn signing_kid(&self) -> Option<&str> {
        self.signing_kid.as_deref()
    }

    /// Access-token TTL in seconds (also drives retiring JWKS retention).
    #[must_use]
    pub const fn access_token_expiration_seconds(&self) -> u64 {
        self.access_token_expiration
    }

    /// Replace JWKS cache (e.g. after registry bootstrap).
    #[must_use]
    pub fn with_jwks(mut self, jwks: JwkSet) -> Self {
        self.jwks_cache = Arc::new(jwks);
        self
    }

    /// Bind the authoritative writer registry. Every RS256 access signature checks
    /// the registered epoch again; bootstrap caches are not authorization.
    #[must_use]
    pub fn with_signing_registry(
        mut self,
        registry: Arc<dyn iam_domain::port::repository::SigningKeyRegistry<Error = DomainError>>,
    ) -> Self {
        self.signing_registry = Some(registry);
        self
    }

    async fn require_active_signing_epoch(
        &self,
        issuer: &str,
        organization: Option<&str>,
        kid: &str,
        provider: &dyn SigningProvider,
    ) -> Result<iam_domain::entity::signing_key::SigningKey, DomainError> {
        use iam_domain::entity::signing_key::{SigningKeyStatus, TrustScope};
        let registry = self.signing_registry.as_ref().ok_or_else(|| {
            DomainError::AuthorizationError("signing registry not configured".into())
        })?;
        let key = registry
            .find_by_kid(kid)
            .await
            .map_err(|_| DomainError::AuthorizationError("signing epoch unavailable".into()))?
            .ok_or_else(|| DomainError::AuthorizationError("signing epoch unavailable".into()))?;
        if key.status != SigningKeyStatus::Active
            || key.algorithm != "RS256"
            || key.issuer != issuer
            || key.issuer != self.issuer()
            || key.kid != kid
            || key.trust_scope != TrustScope::Platform
            || key.organization_id.is_some()
            || organization.is_some()
        {
            return Err(DomainError::AuthorizationError(
                "signing epoch not eligible".into(),
            ));
        }
        let public = provider
            .public_key()
            .await
            .map_err(|_| DomainError::AuthorizationError("signer unavailable".into()))?;
        let expected =
            Self::extract_rsa_components(&key.public_key).map_err(|_| DomainError::InvalidToken)?;
        let actual =
            Self::extract_rsa_components(&public).map_err(|_| DomainError::InvalidToken)?;
        if expected != actual {
            return Err(DomainError::AuthorizationError(
                "signing material mismatch".into(),
            ));
        }
        Ok(key)
    }

    async fn confirm_emission(
        &self,
        expected: &iam_domain::entity::signing_key::SigningKey,
    ) -> Result<(), DomainError> {
        let error = || DomainError::AuthorizationError("signing epoch unavailable".into());
        let registry = self.signing_registry.as_ref().ok_or_else(error)?;
        if !registry
            .confirm_active_for_emission(expected)
            .await
            .map_err(|_| error())?
        {
            return Err(error());
        }
        Ok(())
    }

    fn issuer(&self) -> &str {
        if let Some(iss) = &self.signing_issuer {
            if !iss.is_empty() {
                return iss;
            }
        }
        if self.issuer.is_empty() {
            DEFAULT_JWT_ISSUER
        } else {
            &self.issuer
        }
    }

    fn audience(&self) -> &str {
        if self.audience.is_empty() {
            DEFAULT_JWT_AUDIENCE
        } else {
            &self.audience
        }
    }

    /// Generate a secure random token string for refresh tokens
    fn generate_random_token() -> String {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..64).map(|_| rng.gen::<u8>()).collect();
        URL_SAFE_NO_PAD.encode(&random_bytes)
    }

    const fn get_algorithm(&self) -> Algorithm {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(_) => Algorithm::RS256,
            JwtAlgorithm::HS256(_) => Algorithm::HS256,
        }
    }

    fn get_hmac_encoding_key(&self) -> Result<EncodingKey, DomainError> {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(_) => Err(DomainError::InvalidToken),
            JwtAlgorithm::HS256(secret) => Ok(EncodingKey::from_secret(secret.as_bytes())),
        }
    }

    fn get_decoding_key(&self) -> Result<DecodingKey, DomainError> {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => {
                DecodingKey::from_rsa_pem(key_pair.public_key.as_bytes()).map_err(|e| {
                    error!("Failed to create RSA decoding key: {}", e);
                    DomainError::InvalidToken
                })
            }
            JwtAlgorithm::HS256(secret) => Ok(DecodingKey::from_secret(secret.as_bytes())),
        }
    }

    fn get_key_id(&self) -> Option<String> {
        if let Some(kid) = &self.signing_kid {
            return Some(kid.clone());
        }
        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => Some(key_pair.kid.clone()),
            JwtAlgorithm::HS256(_) => None,
        }
    }

    /// Extract RSA modulus (n) and exponent (e) from a PEM public key.
    pub fn extract_rsa_components(public_key_pem: &str) -> Result<(String, String), String> {
        let rsa_pub = iam_domain::entity::signing_key::parse_signing_public_key(public_key_pem)
            .map_err(|_| "invalid RSA public key".to_string())?;
        let n_b64 = URL_SAFE_NO_PAD.encode(rsa_pub.n().to_bytes_be());
        let e_b64 = URL_SAFE_NO_PAD.encode(rsa_pub.e().to_bytes_be());
        Ok((n_b64, e_b64))
    }

    /// Build a JWK from a public PEM + kid + issuer (never HMAC).
    ///
    /// # Errors
    ///
    /// Returns an error string if the PEM cannot be parsed.
    pub fn jwk_from_pem(public_key_pem: &str, kid: &str, issuer: &str) -> Result<Jwk, String> {
        Jwk::from_rsa_pem(public_key_pem, kid, issuer)
    }

    fn uses_rs256(&self) -> bool {
        self.signing_provider.is_some() || matches!(self.algorithm_config, JwtAlgorithm::RS256(_))
    }

    pub(super) fn require_registration_configuration(&self) -> Result<(), DomainError> {
        if self.uses_rs256() {
            if self.signing_registry.is_none()
                || self.get_key_id().is_none_or(|kid| kid.is_empty())
                || self.signing_provider.is_some() != self.signing_kid.is_some()
            {
                return Err(DomainError::AuthorizationError("unbound RSA codec".into()));
            }
            if self.signing_provider.is_none() {
                if let JwtAlgorithm::RS256(pair) = &self.algorithm_config {
                    crate::signing::PemSigningProvider::new(
                        &pair.private_key,
                        pair.public_key.clone(),
                    )?;
                }
            }
        }
        Ok(())
    }

    fn verify_rs256_candidate(
        candidate: &str,
        expected: &iam_domain::entity::signing_key::SigningKey,
    ) -> Result<(), DomainError> {
        let error = || DomainError::AuthorizationError("signing verification failed".into());
        let (input, signature) = candidate.rsplit_once('.').ok_or_else(error)?;
        let signature = URL_SAFE_NO_PAD.decode(signature).map_err(|_| error())?;
        let public =
            iam_domain::entity::signing_key::parse_signing_public_key(&expected.public_key)
                .map_err(|_| error())?;
        public
            .verify(
                Pkcs1v15Sign::new::<Sha256>(),
                &Sha256::digest(input.as_bytes()),
                &signature,
            )
            .map_err(|_| error())
    }

    // One crypto authority and final writer fence for both namespaces. Claims
    // remain typed; registration is never converted to access TokenClaims.
    async fn encode_rs256<T: serde::Serialize + Sync>(
        &self,
        claims: &T,
        issuer: &str,
        organization: Option<&str>,
        typ: &str,
    ) -> Result<String, DomainError> {
        self.require_registration_configuration()?;
        let local;
        let provider: &dyn SigningProvider = if let Some(provider) = &self.signing_provider {
            provider.as_ref()
        } else if let JwtAlgorithm::RS256(pair) = &self.algorithm_config {
            local = crate::signing::PemSigningProvider::new(
                &pair.private_key,
                pair.public_key.clone(),
            )?;
            &local
        } else {
            return Err(DomainError::InvalidToken);
        };
        let kid = self.get_key_id().ok_or(DomainError::InvalidToken)?;
        let expected = self
            .require_active_signing_epoch(issuer, organization, &kid, provider)
            .await?;
        let header = Header {
            alg: Algorithm::RS256,
            typ: Some(typ.to_string()),
            kid: Some(expected.kid.clone()),
            ..Default::default()
        };
        let serialize_error =
            |_| DomainError::AuthorizationError("token serialization failed".into());
        let header_json = serde_json::to_vec(&header).map_err(serialize_error)?;
        let claims_json = serde_json::to_vec(claims).map_err(serialize_error)?;
        let signing_input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(header_json),
            URL_SAFE_NO_PAD.encode(claims_json)
        );
        let digest = Sha256::digest(signing_input.as_bytes());
        let signature = provider
            .sign_digest(&digest)
            .await
            .map_err(|_| DomainError::AuthorizationError("signer unavailable".into()))?;
        let candidate = format!("{}.{}", signing_input, URL_SAFE_NO_PAD.encode(signature));
        Self::verify_rs256_candidate(&candidate, &expected)?;
        self.confirm_emission(&expected).await?;
        Ok(candidate)
    }

    /// Mint a completion-only token through the shared signer and writer fence.
    /// # Errors
    /// Rejects invalid registration identity, signer material, or inactive epoch.
    pub async fn encode_registration(
        &self,
        claims: &RegistrationTokenClaims,
    ) -> Result<String, DomainError> {
        if !claims.is_registration_token() || claims.get_user_id().is_err() {
            return Err(DomainError::InvalidToken);
        }
        let mut claims = claims.clone();
        claims.iss = Some(self.issuer().into());
        claims.aud = Some("registration".into());
        if self.uses_rs256() {
            return self.encode_rs256(&claims, self.issuer(), None, "JWT").await;
        }
        // Explicit existing HMAC codec only (isolated fixture / authorized dual
        // window). A failed RSA branch can never reach this branch.
        jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &self.get_hmac_encoding_key()?,
        )
        .map_err(|_| DomainError::AuthorizationError("registration encoding failed".into()))
    }

    /// Verify completion tokens with the registered public key, never a vendor or
    /// fixed PEM bootstrap. Legacy missing iss/aud is allowed only in this method.
    /// # Errors
    /// Rejects bad algorithm/namespace/binding/signature, revoked/pending or expired tokens.
    pub async fn decode_registration(
        &self,
        token: &str,
    ) -> Result<RegistrationTokenClaims, DomainError> {
        use iam_domain::entity::signing_key::{SigningKeyStatus, TrustScope};
        let header = jsonwebtoken::decode_header(token).map_err(|_| DomainError::InvalidToken)?;
        if header.typ.as_deref() != Some("JWT") {
            return Err(DomainError::InvalidToken);
        }
        let (algorithm, public, issuer) = if self.uses_rs256() {
            if header.alg != Algorithm::RS256 {
                return Err(DomainError::InvalidToken);
            }
            let kid = header
                .kid
                .as_deref()
                .filter(|kid| !kid.is_empty())
                .ok_or(DomainError::InvalidToken)?;
            let registry = self
                .signing_registry
                .as_ref()
                .ok_or(DomainError::InvalidToken)?;
            let key = registry
                .find_by_kid(kid)
                .await
                .map_err(|_| DomainError::InvalidToken)?
                .ok_or(DomainError::InvalidToken)?;
            if key.kid != kid
                || key.algorithm != "RS256"
                || key.issuer != self.issuer()
                || key.trust_scope != TrustScope::Platform
                || key.organization_id.is_some()
                || !matches!(
                    key.status,
                    SigningKeyStatus::Active | SigningKeyStatus::Retiring
                )
            {
                return Err(DomainError::InvalidToken);
            }
            let public = DecodingKey::from_rsa_pem(key.public_key.as_bytes())
                .map_err(|_| DomainError::InvalidToken)?;
            (Algorithm::RS256, public, key.issuer)
        } else {
            if header.alg != Algorithm::HS256 {
                return Err(DomainError::InvalidToken);
            }
            (
                Algorithm::HS256,
                self.get_decoding_key()?,
                self.issuer().to_string(),
            )
        };
        let mut validation = Validation::new(algorithm);
        validation.set_required_spec_claims(&["sub", "exp", "iat", "jti"]);
        validation.leeway = 0;
        // Optional legacy fields are checked strictly below, only after signature.
        validation.validate_aud = false;
        let claims = jsonwebtoken::decode::<RegistrationTokenClaims>(token, &public, &validation)
            .map_err(|error| match error.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => DomainError::TokenExpired,
                _ => DomainError::InvalidToken,
            })?
            .claims;
        if !claims.is_registration_token()
            || claims.get_user_id().is_err()
            || claims.iss.as_deref().is_some_and(|value| value != issuer)
            || claims
                .aud
                .as_deref()
                .is_some_and(|value| value != "registration")
        {
            return Err(DomainError::InvalidToken);
        }
        if claims.is_expired() {
            return Err(DomainError::TokenExpired);
        }
        Ok(claims)
    }
}

#[async_trait]
impl JwtTokenEncoder for JwtTokenService {
    async fn encode(&self, claims: &TokenClaims) -> Result<String, DomainError> {
        debug!("Encoding JWT token for user: {}", claims.sub);

        if self.uses_rs256() {
            return self
                .encode_rs256(claims, &claims.iss, claims.org.as_deref(), ACCESS_TOKEN_TYP)
                .await;
        }
        let header = Header::new(Algorithm::HS256);
        let encoding_key = self.get_hmac_encoding_key()?;
        let candidate = jsonwebtoken::encode(&header, claims, &encoding_key).map_err(|e| {
            error!("Failed to encode JWT: {}", e);
            DomainError::AuthorizationError(format!("Token encoding failed: {e}"))
        })?;
        Ok(candidate)
    }

    fn decode(&self, token: &str) -> Result<TokenClaims, DomainError> {
        debug!("Decoding JWT token");
        let decoding_key = self.get_decoding_key()?;
        let mut validation = Validation::new(self.get_algorithm());
        validation.set_required_spec_claims(&["sub", "exp", "iat", "jti", "iss", "aud"]);
        validation.set_issuer(&[self.issuer()]);
        validation.set_audience(&[self.audience()]);

        let token_data = jsonwebtoken::decode::<TokenClaims>(token, &decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => DomainError::TokenExpired,
                jsonwebtoken::errors::ErrorKind::InvalidSignature => DomainError::InvalidToken,
                _ => DomainError::InvalidToken,
            })?;
        Ok(token_data.claims)
    }

    fn jwks(&self) -> JwkSet {
        if !self.jwks_cache.keys.is_empty() {
            return (*self.jwks_cache).clone();
        }
        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => {
                match Self::jwk_from_pem(&key_pair.public_key, &key_pair.kid, self.issuer()) {
                    Ok(jwk) => JwkSet { keys: vec![jwk] },
                    Err(e) => {
                        error!("Failed to extract RSA components for JWKS: {}", e);
                        JwkSet { keys: vec![] }
                    }
                }
            }
            JwtAlgorithm::HS256(_) => JwkSet { keys: vec![] },
        }
    }
}

#[async_trait]
impl AuthTokenService for JwtTokenService {
    type Error = TokenError;

    async fn generate_access_token(&self, user_id: Uuid) -> Result<JwtToken, Self::Error> {
        let now = Utc::now();
        let expires_at = now
            + Duration::seconds(i64::try_from(self.access_token_expiration).unwrap_or(i64::MAX));

        let claims = TokenClaims {
            sub: user_id.to_string(),
            username: String::new(),
            iss: self.issuer().to_string(),
            aud: self.audience().to_string(),
            org: None,
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let token = self.encode(&claims).await.map_err(|e| match e {
            DomainError::AuthorizationError(msg) => TokenError::GenericError(msg),
            DomainError::InvalidToken => TokenError::InvalidToken,
            DomainError::TokenExpired => TokenError::TokenExpired,
            _ => TokenError::GenericError(e.to_string()),
        })?;

        Ok(JwtToken {
            user_id,
            token,
            expires_at,
        })
    }

    async fn generate_refresh_token(&self, user_id: Uuid) -> Result<RefreshToken, Self::Error> {
        let now = Utc::now();
        let expires_at = now
            + Duration::seconds(i64::try_from(self.refresh_token_expiration).unwrap_or(i64::MAX));
        let token = Self::generate_random_token();

        Ok(RefreshToken {
            id: Uuid::new_v4(),
            user_id,
            token,
            is_valid: true,
            created_at: now,
            expires_at,
        })
    }

    async fn validate_access_token(&self, token: &str) -> Result<Uuid, Self::Error> {
        let claims = self.decode(token).map_err(|e| match e {
            DomainError::TokenExpired => TokenError::TokenExpired,
            DomainError::InvalidToken => TokenError::InvalidToken,
            _ => TokenError::GenericError(e.to_string()),
        })?;
        Uuid::parse_str(&claims.sub).map_err(|_| TokenError::InvalidToken)
    }

    async fn validate_refresh_token(&self, _token: &str) -> Result<RefreshToken, Self::Error> {
        Err(TokenError::GenericError(
            "Not implemented directly in the token service".to_string(),
        ))
    }
}

#[cfg(test)]
mod platform_hs256 {
    use super::*;
    use iam_domain::port::service::JwtTokenEncoder;

    #[tokio::test]
    async fn hs256_roundtrip_and_empty_jwks() {
        let service = JwtTokenService::with_hmac("rustycog-test-hs256-secret".into(), 900);
        let claims = TokenClaims {
            sub: Uuid::new_v4().to_string(),
            username: "tester".into(),
            iss: DEFAULT_JWT_ISSUER.into(),
            aud: DEFAULT_JWT_AUDIENCE.into(),
            org: None,
            exp: Utc::now().timestamp() + 60,
            iat: Utc::now().timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let token = service.encode(&claims).await.expect("HS256 encode");
        let decoded = service.decode(&token).expect("HS256 decode");
        assert_eq!(decoded.sub, claims.sub);
        assert!(
            service.jwks().keys.is_empty(),
            "HMAC secrets must not be published in JWKS"
        );
    }

    #[test]
    fn signing_configuration_debug_never_contains_private_material() {
        let hmac = JwtAlgorithm::HS256("unit-hmac-secret".into());
        assert!(!format!("{hmac:?}").contains("unit-hmac-secret"));
        let pair = JwtKeyPair {
            private_key: "unit-private-pem".into(),
            public_key: "unit-public".into(),
            kid: "public-kid".into(),
        };
        assert!(!format!("{pair:?}").contains("unit-private-pem"));
        assert!(!format!("{:?}", JwtAlgorithm::RS256(pair)).contains("unit-private-pem"));
    }

    #[tokio::test]
    async fn rs256_missing_registry_or_ineligible_epoch_never_calls_signer() {
        use iam_domain::port::SigningCapabilities;
        use sea_orm::{DbBackend, MockDatabase};
        struct NoSign;
        #[async_trait]
        impl SigningProvider for NoSign {
            async fn sign_digest(&self, _: &[u8]) -> Result<Vec<u8>, DomainError> {
                panic!("ineligible epoch must not reach signer")
            }
            async fn public_key(&self) -> Result<String, DomainError> {
                Ok(include_str!("../../../config/keys/test-platform.pub").into())
            }
            fn capabilities(&self) -> SigningCapabilities {
                SigningCapabilities::default()
            }
        }
        for status in [None, Some("pending"), Some("retiring"), Some("revoked")] {
            let public = include_str!("../../../config/keys/test-platform.pub");
            let mut service = JwtTokenService::with_rsa(
                JwtKeyPair {
                    private_key: String::new(),
                    public_key: public.into(),
                    kid: "epoch".into(),
                },
                900,
            )
            .with_signing_provider(
                Arc::new(NoSign),
                "epoch",
                DEFAULT_JWT_ISSUER,
                JwkSet { keys: vec![] },
            );
            if let Some(status) = status {
                let now = Utc::now().naive_utc();
                let row = crate::repository::entity::signing_keys::Model {
                    id: Uuid::new_v4(),
                    kid: "epoch".into(),
                    algorithm: "RS256".into(),
                    trust_scope: "platform".into(),
                    issuer: DEFAULT_JWT_ISSUER.into(),
                    provider_type: "pem_file".into(),
                    provider_key_ref: "platform.pem".into(),
                    credential_ref: None,
                    public_key: public.into(),
                    status: status.into(),
                    organization_id: None,
                    created_at: now,
                    updated_at: now,
                };
                let db = MockDatabase::new(DbBackend::Postgres)
                    .append_query_results([vec![row]])
                    .into_connection();
                service = service.with_signing_registry(Arc::new(
                    crate::repository::signing_key_registry::SeaOrmSigningKeyRegistry::new(
                        Arc::new(db),
                        Arc::new(
                            iam_domain::entity::signing_key::SigningKeyLifecyclePolicy::new()
                                .unwrap(),
                        ),
                        900,
                    )
                    .unwrap(),
                ));
            }
            assert!(service.generate_access_token(Uuid::new_v4()).await.is_err());
        }
    }
}

#[cfg(test)]
mod emission_fence_tests {
    use super::*;
    use crate::token::RegistrationTokenServiceImpl;
    use iam_domain::entity::registration_token::{ProviderInfo, RegistrationFlow};
    use iam_domain::port::service::RegistrationTokenService;
    use iam_domain::{
        entity::signing_key::{SigningKey, SigningKeyStatus, SigningProviderType, TrustScope},
        port::{repository::SigningKeyRegistry, SigningCapabilities},
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };
    use tokio::sync::Notify;

    fn registered_codec(key: SigningKey) -> (Arc<JwtTokenService>, Arc<FenceRegistry>) {
        let registry = Arc::new(FenceRegistry {
            final_snapshot: Mutex::new(FinalSnapshot::Row(Some(key.clone()))),
            initial: key,
            confirmations: AtomicUsize::new(0),
            signatures: Arc::new(AtomicUsize::new(0)),
            remote: false,
        });
        (
            Arc::new(local_service().with_signing_registry(registry.clone())),
            registry,
        )
    }

    async fn registration_call(
        service: &RegistrationTokenServiceImpl,
        oauth: bool,
        user_id: Uuid,
    ) -> Result<String, DomainError> {
        if oauth {
            service
                .generate_oauth_registration_token(
                    user_id,
                    "unit@example.test".into(),
                    ProviderInfo {
                        email: "provider@example.test".into(),
                        avatar: Some("https://example.test/avatar".into()),
                        suggested_username: "suggested".into(),
                    },
                )
                .await
        } else {
            service
                .generate_registration_token(user_id, "unit@example.test".into())
                .await
        }
    }

    fn distinct_pair() -> &'static (String, String) {
        use rand::SeedableRng;
        use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
        static PAIR: std::sync::OnceLock<(String, String)> = std::sync::OnceLock::new();
        PAIR.get_or_init(|| {
            // Non-secret deterministic test key, not a production key/fixture.
            let key = rsa::RsaPrivateKey::new(&mut rand::rngs::StdRng::seed_from_u64(1313), 2048)
                .unwrap();
            (
                key.to_pkcs8_pem(LineEnding::LF).unwrap().to_string(),
                key.to_public_key()
                    .to_public_key_pem(LineEnding::LF)
                    .unwrap(),
            )
        })
    }

    // Deliberately signed legacy/invalid fixtures, never a production bypass.
    fn raw_registration(claims: &RegistrationTokenClaims, kid: &str) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.into());
        jsonwebtoken::encode(
            &header,
            claims,
            &EncodingKey::from_rsa_pem(include_bytes!("../../../config/keys/test-platform.pem"))
                .unwrap(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn registration_both_flows_share_active_codec_and_preserve_claims_ttl_audience() {
        let (codec, registry) = registered_codec(active_epoch());
        let service = RegistrationTokenServiceImpl::new(codec.clone()).unwrap();
        for oauth in [false, true] {
            let id = Uuid::new_v4();
            let token = registration_call(&service, oauth, id).await.unwrap();
            let claims = service.validate_registration_token(&token).await.unwrap();
            assert_eq!(claims.sub, "registration");
            assert_eq!(claims.get_user_id().unwrap(), id);
            assert_eq!(claims.email, "unit@example.test");
            assert_eq!(claims.exp - claims.iat, 24 * 60 * 60);
            assert!(Uuid::parse_str(&claims.jti).is_ok());
            assert_eq!(claims.iss.as_deref(), Some(DEFAULT_JWT_ISSUER));
            assert_eq!(claims.aud.as_deref(), Some("registration"));
            assert_eq!(
                claims.flow,
                if oauth {
                    RegistrationFlow::OAuth
                } else {
                    RegistrationFlow::EmailPassword
                }
            );
            if oauth {
                let provider = claims.provider_info.as_ref().unwrap();
                assert_eq!(provider.email, "provider@example.test");
                assert_eq!(provider.suggested_username, "suggested");
                assert_eq!(
                    provider.avatar.as_deref(),
                    Some("https://example.test/avatar")
                );
            } else {
                assert!(claims.provider_info.is_none());
            }
            let header = jsonwebtoken::decode_header(&token).unwrap();
            assert_eq!(header.alg, Algorithm::RS256);
            assert_eq!(header.kid.as_deref(), Some("epoch"));
            assert_eq!(header.typ.as_deref(), Some("JWT"));
            assert!(service.is_registration_token_valid(&token).await);
            // Low-level namespace evidence only; actual IAM/Envoy guards belong to E.
            assert!(codec.decode(&token).is_err());
        }
        assert_eq!(registry.confirmations.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn registration_both_flows_reject_retirement_during_public_key_or_sign_barrier() {
        for oauth in [false, true] {
            for block_public_key in [false, true] {
                for case in [0, 2, 3, 7] {
                    let initial = active_epoch();
                    let signatures = Arc::new(AtomicUsize::new(0));
                    let registry = Arc::new(FenceRegistry {
                        initial: initial.clone(),
                        final_snapshot: Mutex::new(FinalSnapshot::Row(Some(initial.clone()))),
                        confirmations: AtomicUsize::new(0),
                        signatures: signatures.clone(),
                        remote: true,
                    });
                    let signer = Arc::new(BarrierSigner {
                        pem: crate::signing::PemSigningProvider::new(
                            include_str!("../../../config/keys/test-platform.pem"),
                            initial.public_key.clone(),
                        )
                        .unwrap(),
                        block_public_key,
                        entered: Notify::new(),
                        release: Notify::new(),
                        signatures,
                    });
                    let codec = Arc::new(
                        local_service()
                            .with_signing_provider(
                                signer.clone(),
                                "epoch",
                                DEFAULT_JWT_ISSUER,
                                JwkSet { keys: vec![] },
                            )
                            .with_signing_registry(registry.clone()),
                    );
                    let service = RegistrationTokenServiceImpl::new(codec).unwrap();
                    let transition = async {
                        signer.entered.notified().await;
                        assert_eq!(registry.confirmations.load(Ordering::SeqCst), 0);
                        *registry.final_snapshot.lock().unwrap() = snapshot(case, &initial);
                        signer.release.notify_one();
                    };
                    let (result, ()) = tokio::join!(
                        registration_call(&service, oauth, Uuid::new_v4()),
                        transition
                    );
                    assert_eq!(result.is_ok(), case == 0);
                    assert_eq!(registry.confirmations.load(Ordering::SeqCst), 1);
                    if let Err(error) = result {
                        assert!(!format!("{error:?} {error}").contains("unit-secret"));
                    }
                }
            }
        }
    }

    struct NoVendor;
    #[async_trait]
    impl SigningProvider for NoVendor {
        async fn public_key(&self) -> Result<String, DomainError> {
            panic!("verification/pre-ineligible mint must never call vendor")
        }
        async fn sign_digest(&self, _: &[u8]) -> Result<Vec<u8>, DomainError> {
            panic!("verification/pre-ineligible mint must never sign")
        }
        fn capabilities(&self) -> SigningCapabilities {
            SigningCapabilities::default()
        }
    }

    #[tokio::test]
    async fn registration_verification_uses_registered_retiring_public_not_bootstrap_or_vendor() {
        let (codec, _) = registered_codec(active_epoch());
        let issuer = RegistrationTokenServiceImpl::new(codec).unwrap();
        let token = registration_call(&issuer, true, Uuid::new_v4())
            .await
            .unwrap();
        let mut retiring = active_epoch();
        retiring.status = SigningKeyStatus::Retiring;
        let (_, registry) = registered_codec(retiring);
        let (private, public) = distinct_pair();
        let stale_bootstrap = JwtTokenService::with_rsa(
            JwtKeyPair {
                private_key: private.clone(),
                public_key: public.clone(),
                kid: "stale-bootstrap".into(),
            },
            900,
        );
        let decoder = Arc::new(
            stale_bootstrap
                .with_signing_provider(
                    Arc::new(NoVendor),
                    "epoch",
                    DEFAULT_JWT_ISSUER,
                    JwkSet { keys: vec![] },
                )
                .with_signing_registry(registry.clone()),
        );
        let service = RegistrationTokenServiceImpl::new(decoder).unwrap();
        assert!(service
            .validate_registration_token(&token)
            .await
            .unwrap()
            .is_oauth_flow());
        assert_eq!(registry.confirmations.load(Ordering::SeqCst), 0);
        assert!(registration_call(&service, false, Uuid::new_v4())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn registration_validation_legacy_is_completion_only_and_wrong_bindings_are_rejected() {
        let (codec, _) = registered_codec(active_epoch());
        let service = RegistrationTokenServiceImpl::new(codec.clone()).unwrap();
        let legacy = RegistrationTokenClaims::new(Uuid::new_v4(), "unit@example.test".into());
        let legacy_token = raw_registration(&legacy, "epoch");
        assert!(service
            .validate_registration_token(&legacy_token)
            .await
            .is_ok());
        assert!(codec.decode(&legacy_token).is_err());
        let access = codec
            .encode(&TokenClaims::new(
                &Uuid::new_v4().to_string(),
                "user",
                chrono::Duration::seconds(60),
            ))
            .await
            .unwrap();
        assert!(service.validate_registration_token(&access).await.is_err());
        for case in 0..6 {
            let mut claims = legacy.clone();
            let mut kid = "epoch";
            match case {
                0 => claims.iss = Some("wrong-issuer".into()),
                1 => claims.aud = Some(DEFAULT_JWT_AUDIENCE.into()),
                2 => claims.sub = Uuid::new_v4().to_string(),
                3 => claims.user_id = "not-a-user-id".into(),
                4 => claims.exp = claims.iat - 1,
                _ => kid = "unknown-kid",
            }
            assert!(service
                .validate_registration_token(&raw_registration(&claims, kid))
                .await
                .is_err());
        }
        for case in 0..5 {
            let mut key = active_epoch();
            match case {
                0 => key.status = SigningKeyStatus::Pending,
                1 => key.status = SigningKeyStatus::Revoked,
                2 => key.organization_id = Some(Uuid::new_v4()),
                3 => key.issuer = "wrong-issuer".into(),
                _ => key.public_key = distinct_pair().1.clone(),
            }
            let (decoder, _) = registered_codec(key);
            assert!(decoder.decode_registration(&legacy_token).await.is_err());
        }
    }

    #[tokio::test]
    async fn registration_unbound_missing_inactive_or_changed_material_fails_closed() {
        assert!(RegistrationTokenServiceImpl::new(Arc::new(local_service())).is_err());
        for oauth in [false, true] {
            for case in 0..5 {
                let mut key = active_epoch();
                match case {
                    0 => key.status = SigningKeyStatus::Pending,
                    1 => key.status = SigningKeyStatus::Retiring,
                    2 => key.status = SigningKeyStatus::Revoked,
                    3 => key.provider_key_ref.push_str("changed-binding"),
                    _ => key.public_key = distinct_pair().1.clone(),
                }
                let (codec, registry) = registered_codec(key);
                if case == 3 {
                    *registry.final_snapshot.lock().unwrap() = snapshot(6, &registry.initial);
                }
                let service = RegistrationTokenServiceImpl::new(codec).unwrap();
                assert!(registration_call(&service, oauth, Uuid::new_v4())
                    .await
                    .is_err());
                assert_eq!(
                    registry.confirmations.load(Ordering::SeqCst),
                    if case == 3 { 1 } else { 0 }
                );
            }
        }
        use sea_orm::{DbBackend, MockDatabase};
        for error in [false, true] {
            let mock = MockDatabase::new(DbBackend::Postgres);
            let mock = if error {
                mock.append_query_errors([sea_orm::DbErr::Custom("secret-db-payload".into())])
            } else {
                mock.append_query_results([
                    Vec::<crate::repository::entity::signing_keys::Model>::new(),
                ])
            };
            let codec = local_service().with_signing_registry(Arc::new(
                crate::repository::signing_key_registry::SeaOrmSigningKeyRegistry::new(
                    Arc::new(mock.into_connection()),
                    Arc::new(
                        iam_domain::entity::signing_key::SigningKeyLifecyclePolicy::new().unwrap(),
                    ),
                    900,
                )
                .unwrap(),
            ));
            let service = RegistrationTokenServiceImpl::new(Arc::new(codec)).unwrap();
            let error = registration_call(&service, false, Uuid::new_v4())
                .await
                .unwrap_err();
            assert!(!format!("{error:?} {error}").contains("secret"));
        }
    }

    struct WrongSignature {
        pem: crate::signing::PemSigningProvider,
        signatures: Arc<AtomicUsize>,
    }
    #[async_trait]
    impl SigningProvider for WrongSignature {
        async fn public_key(&self) -> Result<String, DomainError> {
            Ok(include_str!("../../../config/keys/test-platform.pub").into())
        }
        async fn sign_digest(&self, digest: &[u8]) -> Result<Vec<u8>, DomainError> {
            self.signatures.fetch_add(1, Ordering::SeqCst);
            self.pem.sign_digest(digest).await
        }
        fn capabilities(&self) -> SigningCapabilities {
            self.pem.capabilities()
        }
    }

    #[tokio::test]
    async fn shared_candidate_verification_rejects_wrong_remote_key_for_access_and_both_registration_flows(
    ) {
        for namespace in 0..3 {
            let (_, registry) = registered_codec(active_epoch());
            let signatures = Arc::new(AtomicUsize::new(0));
            let (private, public) = distinct_pair();
            let signer = WrongSignature {
                pem: crate::signing::PemSigningProvider::new(private, public.clone()).unwrap(),
                signatures: signatures.clone(),
            };
            let codec = Arc::new(
                local_service()
                    .with_signing_provider(
                        Arc::new(signer),
                        "epoch",
                        DEFAULT_JWT_ISSUER,
                        JwkSet { keys: vec![] },
                    )
                    .with_signing_registry(registry.clone()),
            );
            let result = if namespace == 0 {
                codec
                    .encode(&TokenClaims::new(
                        "unit-user",
                        "user",
                        chrono::Duration::seconds(60),
                    ))
                    .await
            } else {
                registration_call(
                    &RegistrationTokenServiceImpl::new(codec).unwrap(),
                    namespace == 2,
                    Uuid::new_v4(),
                )
                .await
            };
            assert!(result.is_err());
            assert_eq!(signatures.load(Ordering::SeqCst), 1);
            assert_eq!(
                registry.confirmations.load(Ordering::SeqCst),
                0,
                "candidate must fail before final SELECT"
            );
        }
    }

    #[tokio::test]
    async fn registration_explicit_hmac_fixture_codec_roundtrips_without_rsa_downgrade() {
        let hmac = Arc::new(JwtTokenService::with_hmac(
            "isolated-fixture-secret".into(),
            900,
        ));
        let service = RegistrationTokenServiceImpl::new(hmac.clone()).unwrap();
        let (rsa, _) = registered_codec(active_epoch());
        for oauth in [false, true] {
            let token = registration_call(&service, oauth, Uuid::new_v4())
                .await
                .unwrap();
            let claims = service.validate_registration_token(&token).await.unwrap();
            assert_eq!(claims.aud.as_deref(), Some("registration"));
            assert!(hmac.decode(&token).is_err());
            assert!(rsa.decode_registration(&token).await.is_err());
        }
        let token = registration_call(
            &RegistrationTokenServiceImpl::new(rsa).unwrap(),
            false,
            Uuid::new_v4(),
        )
        .await
        .unwrap();
        assert!(service.validate_registration_token(&token).await.is_err());
    }

    enum FinalSnapshot {
        Row(Option<SigningKey>),
        False,
        Error,
    }
    struct FenceRegistry {
        initial: SigningKey,
        final_snapshot: Mutex<FinalSnapshot>,
        confirmations: AtomicUsize,
        signatures: Arc<AtomicUsize>,
        remote: bool,
    }
    #[async_trait]
    impl SigningKeyRegistry for FenceRegistry {
        type Error = DomainError;
        async fn jwks_publication_snapshot(
            &self,
        ) -> Result<iam_domain::entity::signing_key::SigningKeyPublicationSnapshot, DomainError>
        {
            panic!("emission must not use publication snapshot")
        }
        async fn confirm_active_for_emission(
            &self,
            expected: &SigningKey,
        ) -> Result<bool, DomainError> {
            // Catches an id-only synthetic expectation or a replaced/blinded binding.
            assert_eq!(expected, &self.initial);
            if self.remote {
                assert_eq!(self.signatures.load(Ordering::SeqCst), 1);
            }
            self.confirmations.fetch_add(1, Ordering::SeqCst);
            match &*self.final_snapshot.lock().unwrap() {
                FinalSnapshot::Row(Some(row)) => {
                    Ok(row.status == SigningKeyStatus::Active && row == expected)
                }
                FinalSnapshot::Row(None) | FinalSnapshot::False => Ok(false),
                FinalSnapshot::Error => Err(DomainError::RepositoryError(
                    "unit-secret sql payload".into(),
                )),
            }
        }
        async fn find_by_kid(&self, _: &str) -> Result<Option<SigningKey>, DomainError> {
            Ok(Some(self.initial.clone()))
        }
        async fn insert(&self, _: &SigningKey) -> Result<(), DomainError> {
            unreachable!()
        }
        async fn update(&self, _: &SigningKey) -> Result<(), DomainError> {
            unreachable!()
        }
        async fn replace_active_organization_key(
            &self,
            _: &SigningKey,
            _: Option<&str>,
        ) -> Result<SigningKey, DomainError> {
            unreachable!()
        }
        async fn revoke_organization_keys(&self, _: Uuid) -> Result<Vec<SigningKey>, DomainError> {
            unreachable!()
        }
        async fn find_active_platform_key(&self) -> Result<Option<SigningKey>, DomainError> {
            unreachable!()
        }
        async fn list_jwks_keys(&self) -> Result<Vec<SigningKey>, DomainError> {
            unreachable!()
        }
        async fn find_by_organization(&self, _: Uuid) -> Result<Vec<SigningKey>, DomainError> {
            unreachable!()
        }
        async fn find_by_issuer(&self, _: &str) -> Result<Vec<SigningKey>, DomainError> {
            unreachable!()
        }
    }

    struct BarrierSigner {
        pem: crate::signing::PemSigningProvider,
        block_public_key: bool,
        entered: Notify,
        release: Notify,
        signatures: Arc<AtomicUsize>,
    }
    #[async_trait]
    impl SigningProvider for BarrierSigner {
        async fn public_key(&self) -> Result<String, DomainError> {
            if self.block_public_key {
                self.entered.notify_one();
                self.release.notified().await;
            }
            self.pem.public_key().await
        }
        async fn sign_digest(&self, digest: &[u8]) -> Result<Vec<u8>, DomainError> {
            if !self.block_public_key {
                self.entered.notify_one();
                self.release.notified().await;
            }
            let signature = self.pem.sign_digest(digest).await?;
            self.signatures.fetch_add(1, Ordering::SeqCst);
            Ok(signature)
        }
        fn capabilities(&self) -> SigningCapabilities {
            self.pem.capabilities()
        }
    }

    fn active_epoch() -> SigningKey {
        let now = Utc::now();
        SigningKey {
            id: Uuid::new_v4(),
            kid: "epoch".into(),
            algorithm: "RS256".into(),
            trust_scope: TrustScope::Platform,
            issuer: DEFAULT_JWT_ISSUER.into(),
            provider_type: SigningProviderType::RemoteHttp,
            provider_key_ref: "unit-signer".into(),
            credential_ref: Some("unit-credential-ref".into()),
            public_key: include_str!("../../../config/keys/test-platform.pub").into(),
            status: SigningKeyStatus::Active,
            organization_id: None,
            created_at: now,
            updated_at: now,
        }
    }
    fn local_service() -> JwtTokenService {
        JwtTokenService::with_rsa(
            JwtKeyPair {
                private_key: include_str!("../../../config/keys/test-platform.pem").into(),
                public_key: include_str!("../../../config/keys/test-platform.pub").into(),
                kid: "epoch".into(),
            },
            900,
        )
    }
    fn snapshot(case: u8, initial: &SigningKey) -> FinalSnapshot {
        let mut row = initial.clone();
        match case {
            0 => FinalSnapshot::Row(Some(row)),
            1..=3 => {
                row.status = match case {
                    1 => SigningKeyStatus::Pending,
                    2 => SigningKeyStatus::Retiring,
                    _ => SigningKeyStatus::Revoked,
                };
                FinalSnapshot::Row(Some(row))
            }
            4 => FinalSnapshot::Row(None),
            5 => {
                row.public_key.push_str("changed");
                FinalSnapshot::Row(Some(row))
            }
            6 => FinalSnapshot::False,
            _ => FinalSnapshot::Error,
        }
    }

    #[tokio::test]
    async fn rs256_final_writer_fence_after_public_key_or_signature_barrier() {
        for block_public_key in [false, true] {
            for case in 0..=7 {
                let initial = active_epoch();
                let signatures = Arc::new(AtomicUsize::new(0));
                let registry = Arc::new(FenceRegistry {
                    initial: initial.clone(),
                    final_snapshot: Mutex::new(FinalSnapshot::Row(Some(initial.clone()))),
                    confirmations: AtomicUsize::new(0),
                    signatures: signatures.clone(),
                    remote: true,
                });
                let signer = Arc::new(BarrierSigner {
                    pem: crate::signing::PemSigningProvider::new(
                        include_str!("../../../config/keys/test-platform.pem"),
                        initial.public_key.clone(),
                    )
                    .unwrap(),
                    block_public_key,
                    entered: Notify::new(),
                    release: Notify::new(),
                    signatures,
                });
                let service = local_service()
                    .with_signing_provider(
                        signer.clone(),
                        "epoch",
                        DEFAULT_JWT_ISSUER,
                        JwkSet { keys: vec![] },
                    )
                    .with_signing_registry(registry.clone());
                let transition = async {
                    signer.entered.notified().await;
                    assert_eq!(registry.confirmations.load(Ordering::SeqCst), 0);
                    *registry.final_snapshot.lock().unwrap() = snapshot(case, &initial);
                    signer.release.notify_one();
                };
                let claims = TokenClaims::new("unit-user", "user", chrono::Duration::seconds(60));
                let (result, ()) = tokio::join!(service.encode(&claims), transition);
                assert_eq!(registry.confirmations.load(Ordering::SeqCst), 1);
                if case == 0 {
                    let token = result.unwrap();
                    assert_eq!(service.decode(&token).unwrap().sub, claims.sub);
                    // A transition after L does not retroactively fail this emission.
                    *registry.final_snapshot.lock().unwrap() = snapshot(3, &initial);
                } else {
                    let error = result.unwrap_err();
                    assert!(!format!("{error:?} {error}").contains("unit-secret"));
                }
            }
        }
    }

    #[tokio::test]
    async fn rs256_local_rsa_candidate_uses_same_final_gate() {
        for case in 0..=7 {
            let initial = active_epoch();
            let registry = Arc::new(FenceRegistry {
                final_snapshot: Mutex::new(snapshot(case, &initial)),
                initial,
                confirmations: AtomicUsize::new(0),
                signatures: Arc::new(AtomicUsize::new(0)),
                remote: false,
            });
            let service = local_service().with_signing_registry(registry.clone());
            let result = service
                .encode(&TokenClaims::new(
                    "unit-user",
                    "user",
                    chrono::Duration::seconds(60),
                ))
                .await;
            assert_eq!(result.is_ok(), case == 0);
            assert_eq!(registry.confirmations.load(Ordering::SeqCst), 1);
        }
    }
}
