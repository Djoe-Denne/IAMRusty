use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};

use iam_domain::entity::token::{Jwk, JwkSet, JwtKeyPair, JwtToken, RefreshToken, TokenClaims};
use iam_domain::error::DomainError;
use iam_domain::port::service::{AuthTokenService, JwtTokenEncoder};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use rsa::{pkcs8::DecodePublicKey, traits::PublicKeyParts, RsaPublicKey};
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

/// JWT algorithm configuration
#[derive(Debug, Clone)]
pub enum JwtAlgorithm {
    /// RSA256 with key pair
    RS256(JwtKeyPair),
    /// HMAC256 with secret
    HS256(String),
}

/// Unified JWT token service that handles both encoding/decoding and token management
#[derive(Clone)]
pub struct JwtTokenService {
    algorithm_config: JwtAlgorithm,
    access_token_expiration: u64,
    refresh_token_expiration: u64,
}

impl JwtTokenService {
    /// Create a new `JwtTokenService` with RSA256 keys
    #[must_use]
    pub const fn with_rsa(key_pair: JwtKeyPair, access_token_expiration: u64) -> Self {
        Self {
            algorithm_config: JwtAlgorithm::RS256(key_pair),
            access_token_expiration,
            refresh_token_expiration: 2_592_000, // Default 30 days
        }
    }

    /// Create a new `JwtTokenService` with HMAC256 secret.
    ///
    /// Only available when the `test-relaxed-jwt` Cargo feature is on
    /// (enabled by `iam-service`'s dev-dependency on `iam-infra`). The
    /// production build does not expose this constructor at all, so any
    /// release-mode caller would fail to compile rather than slip an
    /// HS256 service into the runtime.
    #[cfg(feature = "test-relaxed-jwt")]
    #[must_use]
    pub const fn with_hmac(secret: String, access_token_expiration: u64) -> Self {
        Self {
            algorithm_config: JwtAlgorithm::HS256(secret),
            access_token_expiration,
            refresh_token_expiration: 2_592_000, // Default 30 days
        }
    }

    /// Create a new `JwtTokenService` with custom refresh token expiration.
    ///
    /// In production builds (no `test-relaxed-jwt` feature) the
    /// `algorithm_config` must be RS256 — passing HS256 trips the
    /// `assert!` and panics at app boot, the same fail-fast posture
    /// `RegistrationTokenServiceImpl::new` enforces. The `test-relaxed-jwt`
    /// build (test harness only) compiles the assertion out, so the
    /// in-tree HS256 `test.toml` boots without committing RSA PEM keys.
    #[must_use]
    pub const fn with_refresh_expiration(
        algorithm_config: JwtAlgorithm,
        access_token_expiration: u64,
        refresh_token_expiration: u64,
    ) -> Self {
        #[cfg(not(feature = "test-relaxed-jwt"))]
        assert!(
            matches!(algorithm_config, JwtAlgorithm::RS256(_)),
            "JwtTokenService must use RSA256 algorithm for security"
        );

        Self {
            algorithm_config,
            access_token_expiration,
            refresh_token_expiration,
        }
    }

    /// Generate a secure random token string for refresh tokens
    fn generate_random_token(&self) -> String {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..64).map(|_| rng.gen::<u8>()).collect();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&random_bytes)
    }

    /// Get the algorithm used by this service
    const fn get_algorithm(&self) -> Algorithm {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(_) => Algorithm::RS256,
            JwtAlgorithm::HS256(_) => Algorithm::HS256,
        }
    }

    /// Get the encoding key
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

    /// Get the decoding key
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

    /// Get the key ID (only for RSA)
    fn get_key_id(&self) -> Option<String> {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => Some(key_pair.kid.clone()),
            JwtAlgorithm::HS256(_) => None,
        }
    }

    /// Extract RSA modulus (n) and exponent (e) from a PEM public key PKCS#8 format
    fn extract_rsa_components(&self, public_key_pem: &str) -> Result<(String, String), String> {
        // 1. Parser le PEM (PKCS#8 ou PKCS#1) en RsaPublicKey
        let rsa_pub = RsaPublicKey::from_public_key_pem(public_key_pem)
            .map_err(|e| format!("Failed to parse RSA public key PEM: {e}"))?;

        // 2. Récupérer le modulus (n) et l'exposant (e) sous forme de BigUint
        let n_big = rsa_pub.n();
        let e_big = rsa_pub.e();

        // 3. Convertir en octets big-endian
        let n_bytes = n_big.to_bytes_be();
        let e_bytes = e_big.to_bytes_be();

        // 4. Encoder en Base64URL sans padding
        let n_b64 = URL_SAFE_NO_PAD.encode(&n_bytes);
        let e_b64 = URL_SAFE_NO_PAD.encode(&e_bytes);

        Ok((n_b64, e_b64))
    }
}

// Implementation of JwtTokenEncoder trait for low-level JWT operations
impl JwtTokenEncoder for JwtTokenService {
    fn encode(&self, claims: &TokenClaims) -> Result<String, DomainError> {
        debug!("Encoding JWT token for user: {}", claims.sub);

        let mut header = Header {
            alg: self.get_algorithm(),
            ..Default::default()
        };

        // Set key ID for RSA keys
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
        validation.set_required_spec_claims(&["sub", "exp", "iat", "jti"]);

        let token_data = jsonwebtoken::decode::<TokenClaims>(token, &decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    debug!("JWT token expired");
                    DomainError::TokenExpired
                }
                jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                    debug!("JWT token has invalid signature");
                    DomainError::InvalidToken
                }
                _ => {
                    error!("JWT validation error: {}", e);
                    DomainError::InvalidToken
                }
            })?;

        Ok(token_data.claims)
    }

    fn jwks(&self) -> JwkSet {
        debug!("Building JWKS");

        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => {
                // Parse the RSA public key to extract modulus and exponent
                match self.extract_rsa_components(&key_pair.public_key) {
                    Ok((n, e)) => {
                        let jwk = Jwk {
                            kty: "RSA".to_string(),
                            kid: key_pair.kid.clone(),
                            use_: "sig".to_string(),
                            alg: "RS256".to_string(),
                            n: n.clone(),
                            e,
                        };

                        debug!("Successfully created JWKS with RSA key (kid: {}, modulus length: {} chars)", 
                            key_pair.kid, n.len());

                        JwkSet { keys: vec![jwk] }
                    }
                    Err(e) => {
                        error!("Failed to extract RSA components for JWKS: {}", e);
                        // Return empty JWKS if we can't parse the key
                        JwkSet { keys: vec![] }
                    }
                }
            }
            JwtAlgorithm::HS256(_) => {
                // HMAC keys are not included in JWKS for security reasons
                JwkSet { keys: vec![] }
            }
        }
    }
}

// Implementation of TokenService trait for high-level token management
#[async_trait]
impl AuthTokenService for JwtTokenService {
    type Error = TokenError;

    async fn generate_access_token(&self, user_id: Uuid) -> Result<JwtToken, Self::Error> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(self.access_token_expiration as i64);

        let claims = TokenClaims {
            sub: user_id.to_string(),
            username: String::new(), // This could be enhanced to include username
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let token = self.encode(&claims).map_err(|e| match e {
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
        let expires_at = now + Duration::seconds(self.refresh_token_expiration as i64);
        let token = self.generate_random_token();

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

        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| TokenError::InvalidToken)?;

        Ok(user_id)
    }

    async fn validate_refresh_token(&self, _token: &str) -> Result<RefreshToken, Self::Error> {
        // This is just a stub implementation
        // The actual validation will query the database to check if the token exists and is valid
        // That is handled by the RefreshToken repository and use case
        Err(TokenError::GenericError(
            "Not implemented directly in the token service".to_string(),
        ))
    }
}
