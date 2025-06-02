//! JWT token encoder implementation

mod jwt_encoder;
mod token_service_adapter;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use domain::entity::token::{JwtToken, RefreshToken};
use domain::port::service::TokenService;
use thiserror::Error;
use tracing::{debug, error};
use rand::Rng;
use base64::{Engine as _, engine::general_purpose};

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,        // Subject (user ID)
    exp: usize,         // Expiration time (as UTC timestamp)
    iat: usize,         // Issued at (as UTC timestamp)
    jti: String,        // JWT ID (unique identifier for the token)
}

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

/// JWT token service implementation
#[derive(Clone)]
pub struct JwtTokenService {
    /// Secret key for signing tokens
    secret: String,
    /// Access token expiration in seconds
    access_token_expiration: u64,
    /// Refresh token expiration in seconds
    refresh_token_expiration: u64,
}

impl JwtTokenService {
    /// Create a new JwtTokenService
    pub fn new(secret: String, access_token_expiration: u64) -> Self {
        Self {
            secret,
            access_token_expiration,
            refresh_token_expiration: 2_592_000, // Default 30 days
        }
    }
    
    /// Create a new JwtTokenService with custom refresh token expiration
    pub fn with_refresh_expiration(secret: String, access_token_expiration: u64, refresh_token_expiration: u64) -> Self {
        Self {
            secret,
            access_token_expiration,
            refresh_token_expiration,
        }
    }
    
    /// Generate a secure random token string
    fn generate_random_token(&self) -> String {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..64).map(|_| rng.gen::<u8>()).collect();
        general_purpose::URL_SAFE_NO_PAD.encode(&random_bytes)
    }
}

#[async_trait]
impl TokenService for JwtTokenService {
    type Error = TokenError;

    async fn generate_access_token(&self, user_id: Uuid) -> Result<JwtToken, Self::Error> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(self.access_token_expiration as i64);
        
        let claims = Claims {
            sub: user_id.to_string(),
            exp: expires_at.timestamp() as usize,
            iat: now.timestamp() as usize,
            jti: Uuid::new_v4().to_string(),
        };
        
        let header = Header::new(Algorithm::HS256);
        let encoding_key = EncodingKey::from_secret(self.secret.as_bytes());
        
        let token = encode(&header, &claims, &encoding_key)
            .map_err(|e| {
                error!("Failed to encode JWT token: {}", e);
                TokenError::JwtError(e)
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
        let decoding_key = DecodingKey::from_secret(self.secret.as_bytes());
        let validation = Validation::new(Algorithm::HS256);
        
        let token_data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| {
                debug!("Failed to decode JWT token: {}", e);
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => TokenError::TokenExpired,
                    _ => TokenError::JwtError(e),
                }
            })?;
        
        let user_id = Uuid::parse_str(&token_data.claims.sub)
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

pub use jwt_encoder::*;
pub use token_service_adapter::*; 