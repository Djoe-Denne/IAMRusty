use chrono::Duration;
use crate::entity::token::{JwkSet, TokenClaims};
use crate::error::DomainError;
use crate::port::service::TokenEncoder;

/// Service for JWT token operations
pub struct TokenService {
    token_encoder: Box<dyn TokenEncoder>,
    token_duration: Duration,
}

impl TokenService {
    /// Create a new token service
    pub fn new(token_encoder: Box<dyn TokenEncoder>, token_duration: Duration) -> Self {
        Self {
            token_encoder,
            token_duration,
        }
    }

    /// Generate a JWT token for a user
    pub fn generate_token(&self, user_id: &str, username: &str) -> Result<String, DomainError> {
        let claims = TokenClaims::new(user_id, username, self.token_duration);
        
        self.token_encoder.encode(&claims)
            .map_err(|e| DomainError::TokenGenerationFailed(e.to_string()))
    }

    /// Validate a JWT token
    pub fn validate_token(&self, token: &str) -> Result<TokenClaims, DomainError> {
        self.token_encoder.decode(token)
            .map_err(|e| {
                match e {
                    DomainError::TokenExpired => DomainError::TokenExpired,
                    DomainError::InvalidToken => DomainError::InvalidToken,
                    _ => DomainError::TokenValidationFailed(e.to_string()),
                }
            })
    }

    /// Get the JSON Web Key Set
    pub fn jwks(&self) -> JwkSet {
        self.token_encoder.jwks()
    }
} 