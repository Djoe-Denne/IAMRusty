use chrono::Duration;
use domain::entity::token::{JwkSet, TokenClaims};
use domain::error::DomainError;
use domain::port::service::TokenEncoder;

use crate::error::ApplicationError;

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
    pub fn generate_token(&self, user_id: &str, username: &str) -> Result<String, ApplicationError> {
        let claims = TokenClaims::new(user_id, username, self.token_duration);
        
        self.token_encoder.encode(&claims)
            .map_err(|e| ApplicationError::Token(format!("Failed to generate token: {}", e)))
    }

    /// Validate a JWT token
    pub fn validate_token(&self, token: &str) -> Result<TokenClaims, ApplicationError> {
        self.token_encoder.decode(token)
            .map_err(|e| {
                match e {
                    DomainError::TokenExpired => ApplicationError::Domain(DomainError::TokenExpired),
                    DomainError::InvalidToken => ApplicationError::Domain(DomainError::InvalidToken),
                    _ => ApplicationError::Token(format!("Failed to validate token: {}", e)),
                }
            })
    }

    /// Get the JSON Web Key Set
    pub fn jwks(&self) -> JwkSet {
        self.token_encoder.jwks()
    }
} 