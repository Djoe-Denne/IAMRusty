//! JWT encoder with SigningProvider RS256 path (ADR-0304).

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
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
use rsa::{pkcs8::DecodePublicKey, traits::PublicKeyParts, RsaPublicKey};
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
#[derive(Debug, Clone)]
pub enum JwtAlgorithm {
    /// RSA256 with key pair (legacy / bootstrap; prefer [`SigningProvider`])
    RS256(JwtKeyPair),
    /// HMAC256 with secret (HS256 migration / registration tokens)
    HS256(String),
}

/// Unified JWT token service that handles both encoding/decoding and token management
#[derive(Clone)]
pub struct JwtTokenService {
    algorithm_config: JwtAlgorithm,
    access_token_expiration: u64,
    refresh_token_expiration: u64,
    issuer: String,
    audience: String,
    /// Optional RS256 SigningProvider (PEM / Transit). When set, access tokens use it.
    signing_provider: Option<Arc<dyn SigningProvider>>,
    /// Opaque kid for SigningProvider-backed encode.
    signing_kid: Option<String>,
    /// Issuer bound to the signing key (must match JWK `iss`).
    signing_issuer: Option<String>,
    /// Cached JWKS built from registry / PEM public key.
    jwks_cache: Arc<JwkSet>,
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

    /// Attach a SigningProvider for RS256 access-token minting (login/refresh only).
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

    fn get_encoding_key(&self) -> Result<EncodingKey, DomainError> {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => {
                EncodingKey::from_rsa_pem(key_pair.private_key.as_bytes()).map_err(|e| {
                    error!("Failed to create RSA encoding key: {}", e);
                    DomainError::AuthorizationError(format!("Invalid private key: {e}"))
                })
            }
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
        let rsa_pub = RsaPublicKey::from_public_key_pem(public_key_pem)
            .map_err(|e| format!("Failed to parse RSA public key PEM: {e}"))?;
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

    async fn encode_with_provider(
        &self,
        claims: &TokenClaims,
        provider: &dyn SigningProvider,
        kid: &str,
    ) -> Result<String, DomainError> {
        let mut header = Header {
            alg: Algorithm::RS256,
            typ: Some(ACCESS_TOKEN_TYP.to_string()),
            kid: Some(kid.to_string()),
            ..Default::default()
        };
        // jsonwebtoken sets typ to "JWT" by default via Header::new — we override.
        header.typ = Some(ACCESS_TOKEN_TYP.to_string());

        let header_json = serde_json::to_vec(&header).map_err(|e| {
            DomainError::AuthorizationError(format!("header serialize failed: {e}"))
        })?;
        let claims_json = serde_json::to_vec(claims).map_err(|e| {
            DomainError::AuthorizationError(format!("claims serialize failed: {e}"))
        })?;
        let signing_input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(header_json),
            URL_SAFE_NO_PAD.encode(claims_json)
        );
        let digest = Sha256::digest(signing_input.as_bytes());
        let signature = provider.sign_digest(&digest).await?;
        Ok(format!(
            "{}.{}",
            signing_input,
            URL_SAFE_NO_PAD.encode(signature)
        ))
    }
}

#[async_trait]
impl JwtTokenEncoder for JwtTokenService {
    async fn encode(&self, claims: &TokenClaims) -> Result<String, DomainError> {
        debug!("Encoding JWT token for user: {}", claims.sub);

        if let (Some(provider), Some(kid)) = (&self.signing_provider, &self.signing_kid) {
            return self
                .encode_with_provider(claims, provider.as_ref(), kid)
                .await;
        }

        // Fallback: local jsonwebtoken encode (HS256 tests / legacy RS256 without provider).
        let mut header = Header {
            alg: self.get_algorithm(),
            ..Default::default()
        };
        if matches!(self.algorithm_config, JwtAlgorithm::RS256(_)) {
            header.typ = Some(ACCESS_TOKEN_TYP.to_string());
        }
        if let Some(kid) = self.get_key_id() {
            header.kid = Some(kid);
        }
        let encoding_key = self.get_encoding_key()?;
        jsonwebtoken::encode(&header, claims, &encoding_key).map_err(|e| {
            error!("Failed to encode JWT: {}", e);
            DomainError::AuthorizationError(format!("Token encoding failed: {e}"))
        })
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
}
