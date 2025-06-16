use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Claims for JWT tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject (user id)
    pub sub: String,

    /// Username
    pub username: String,

    /// JWT expiration timestamp
    pub exp: i64,

    /// JWT issued at timestamp
    pub iat: i64,

    /// JWT ID
    pub jti: String,
}

impl TokenClaims {
    /// Creates new token claims for a user
    pub fn new(user_id: &str, username: &str, expires_in: Duration) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            username: username.to_string(),
            exp: (now + expires_in).timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
        }
    }
}

/// JWT key pair for token signing and verification
#[derive(Debug, Clone)]
pub struct JwtKeyPair {
    /// Private key (RS256)
    pub private_key: String,

    /// Public key (RS256)
    pub public_key: String,

    /// Key ID
    pub kid: String,
}

/// JSON Web Key Set for token verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwkSet {
    /// List of keys
    pub keys: Vec<Jwk>,
}

/// JSON Web Key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    /// Key type
    pub kty: String,

    /// Key ID
    pub kid: String,

    /// Key usage
    pub use_: String,

    /// Algorithm
    pub alg: String,

    /// Modulus (RS256)
    pub n: String,

    /// Exponent (RS256)
    pub e: String,
}

/// JWT token data
#[derive(Clone)]
pub struct JwtToken {
    /// User ID that the token belongs to
    pub user_id: Uuid,
    /// Token string
    pub token: String,
    /// Token expiration time
    pub expires_at: DateTime<Utc>,
}

/// Refresh token entity
#[derive(Clone)]
pub struct RefreshToken {
    /// Unique identifier for the refresh token
    pub id: Uuid,
    /// User ID that the token belongs to
    pub user_id: Uuid,
    /// Token string (hashed in storage)
    pub token: String,
    /// Is the token still valid or has it been revoked
    pub is_valid: bool,
    /// When the token was created
    pub created_at: DateTime<Utc>,
    /// When the token expires
    pub expires_at: DateTime<Utc>,
}
