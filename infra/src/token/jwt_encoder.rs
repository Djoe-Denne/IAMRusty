use domain::entity::{
    token::{JwkSet, Jwk, TokenClaims, JwtKeyPair, JwtToken, RefreshToken},
};
use domain::error::DomainError;
use domain::port::service::{JwtTokenEncoder, AuthTokenService};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use tracing::{debug, error};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use uuid::Uuid;
use rand::Rng;
use thiserror::Error;

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
    /// Create a new JwtTokenService with RSA256 keys
    pub fn with_rsa(key_pair: JwtKeyPair, access_token_expiration: u64) -> Self {
        Self {
            algorithm_config: JwtAlgorithm::RS256(key_pair),
            access_token_expiration,
            refresh_token_expiration: 2_592_000, // Default 30 days
        }
    }

    /// Create a new JwtTokenService with HMAC256 secret
    pub fn with_hmac(secret: String, access_token_expiration: u64) -> Self {
        Self {
            algorithm_config: JwtAlgorithm::HS256(secret),
            access_token_expiration,
            refresh_token_expiration: 2_592_000, // Default 30 days
        }
    }

    /// Create a new JwtTokenService with custom refresh token expiration
    pub fn with_refresh_expiration(algorithm_config: JwtAlgorithm, access_token_expiration: u64, refresh_token_expiration: u64) -> Self {
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
    fn get_algorithm(&self) -> Algorithm {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(_) => Algorithm::RS256,
            JwtAlgorithm::HS256(_) => Algorithm::HS256,
        }
    }

    /// Get the encoding key
    fn get_encoding_key(&self) -> Result<EncodingKey, DomainError> {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => {
                EncodingKey::from_rsa_pem(key_pair.private_key.as_bytes())
                    .map_err(|e| {
                        error!("Failed to create RSA encoding key: {}", e);
                        DomainError::AuthorizationError(format!("Invalid private key: {}", e))
                    })
            }
            JwtAlgorithm::HS256(secret) => {
                Ok(EncodingKey::from_secret(secret.as_bytes()))
            }
        }
    }

    /// Get the decoding key
    fn get_decoding_key(&self) -> Result<DecodingKey, DomainError> {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => {
                DecodingKey::from_rsa_pem(key_pair.public_key.as_bytes())
                    .map_err(|e| {
                        error!("Failed to create RSA decoding key: {}", e);
                        DomainError::InvalidToken
                    })
            }
            JwtAlgorithm::HS256(secret) => {
                Ok(DecodingKey::from_secret(secret.as_bytes()))
            }
        }
    }

    /// Get the key ID (only for RSA)
    fn get_key_id(&self) -> Option<String> {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => Some(key_pair.kid.clone()),
            JwtAlgorithm::HS256(_) => None,
        }
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

        jsonwebtoken::encode(&header, claims, &encoding_key)
            .map_err(|e| {
                error!("Failed to encode JWT: {}", e);
                DomainError::AuthorizationError(format!("Token encoding failed: {}", e))
            })
    }

    fn decode(&self, token: &str) -> Result<TokenClaims, DomainError> {
        debug!("Decoding JWT token");
        
        let decoding_key = self.get_decoding_key()?;

        let mut validation = Validation::new(self.get_algorithm());
        validation.set_required_spec_claims(&["sub", "exp", "iat", "jti"]);

        let token_data = jsonwebtoken::decode::<TokenClaims>(token, &decoding_key, &validation)
            .map_err(|e| {
                match e.kind() {
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
                }
            })?;

        Ok(token_data.claims)
    }

    fn jwks(&self) -> JwkSet {
        debug!("Building JWKS");
        
        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => {
                // Extract modulus and exponent from the public key
                // In a real implementation, we would parse the RSA key properly
                // This is just a placeholder for the structure
                let n = URL_SAFE_NO_PAD.encode("placeholder_modulus");
                let e = URL_SAFE_NO_PAD.encode("AQAB"); // Standard RSA exponent

                let jwk = Jwk {
                    kty: "RSA".to_string(),
                    kid: key_pair.kid.clone(),
                    use_: "sig".to_string(),
                    alg: "RS256".to_string(),
                    n,
                    e,
                };

                JwkSet {
                    keys: vec![jwk],
                }
            }
            JwtAlgorithm::HS256(_) => {
                // HMAC keys are not included in JWKS for security reasons
                JwkSet {
                    keys: vec![],
                }
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
            username: "".to_string(), // This could be enhanced to include username
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
        };
        
        let token = self.encode(&claims)
            .map_err(|e| {
                match e {
                    DomainError::AuthorizationError(msg) => TokenError::GenericError(msg),
                    DomainError::InvalidToken => TokenError::InvalidToken,
                    DomainError::TokenExpired => TokenError::TokenExpired,
                    _ => TokenError::GenericError(e.to_string()),
                }
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
        let claims = self.decode(token)
            .map_err(|e| {
                match e {
                    DomainError::TokenExpired => TokenError::TokenExpired,
                    DomainError::InvalidToken => TokenError::InvalidToken,
                    _ => TokenError::GenericError(e.to_string()),
                }
            })?;
        
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| TokenError::InvalidToken)?;
        
        Ok(user_id)
    }

    async fn validate_refresh_token(&self, _token: &str) -> Result<RefreshToken, Self::Error> {
        // This is just a stub implementation
        // The actual validation will query the database to check if the token exists and is valid
        // That is handled by the RefreshToken repository and use case
        Err(TokenError::GenericError("Not implemented directly in the token service".to_string()))
    }
} 