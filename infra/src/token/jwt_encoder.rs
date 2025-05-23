use domain::entity::token::{JwkSet, Jwk, TokenClaims, JwtKeyPair};
use domain::error::DomainError;
use domain::port::service::TokenEncoder;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

/// JSON Web Token encoder/decoder
pub struct JwtTokenEncoder {
    key_pair: JwtKeyPair,
    algorithm: Algorithm,
}

impl JwtTokenEncoder {
    /// Create a new JwtTokenEncoder with an existing key pair
    pub fn new(key_pair: JwtKeyPair) -> Self {
        Self {
            key_pair,
            algorithm: Algorithm::RS256,
        }
    }

    /// Create a JwtTokenEncoder with a new key pair
    pub fn generate() -> Result<Self, DomainError> {
        // In a real implementation, we would generate RS256 keys
        // For simplicity, this is just a placeholder
        Err(DomainError::AuthorizationError("Key generation not implemented".to_string()))
    }
}

impl TokenEncoder for JwtTokenEncoder {
    fn encode(&self, claims: &TokenClaims) -> Result<String, DomainError> {
        debug!("Encoding JWT token for user: {}", claims.sub);
        
        let header = Header {
            kid: Some(self.key_pair.kid.clone()),
            alg: self.algorithm,
            ..Default::default()
        };

        let encoding_key = EncodingKey::from_rsa_pem(self.key_pair.private_key.as_bytes())
            .map_err(|e| {
                error!("Failed to create encoding key: {}", e);
                DomainError::AuthorizationError(format!("Invalid private key: {}", e))
            })?;

        jsonwebtoken::encode(&header, claims, &encoding_key)
            .map_err(|e| {
                error!("Failed to encode JWT: {}", e);
                DomainError::AuthorizationError(format!("Token encoding failed: {}", e))
            })
    }

    fn decode(&self, token: &str) -> Result<TokenClaims, DomainError> {
        debug!("Decoding JWT token");
        
        let decoding_key = DecodingKey::from_rsa_pem(self.key_pair.public_key.as_bytes())
            .map_err(|e| {
                error!("Failed to create decoding key: {}", e);
                DomainError::InvalidToken
            })?;

        let mut validation = Validation::new(self.algorithm);
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
        
        // Extract modulus and exponent from the public key
        // In a real implementation, we would parse the RSA key properly
        // This is just a placeholder for the structure
        let n = URL_SAFE_NO_PAD.encode("placeholder_modulus");
        let e = URL_SAFE_NO_PAD.encode("AQAB"); // Standard RSA exponent

        let jwk = Jwk {
            kty: "RSA".to_string(),
            kid: self.key_pair.kid.clone(),
            use_: "sig".to_string(),
            alg: "RS256".to_string(),
            n,
            e,
        };

        JwkSet {
            keys: vec![jwk],
        }
    }
} 