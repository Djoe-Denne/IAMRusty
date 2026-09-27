use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Default JWT issuer for the `AIForAll` platform
pub const DEFAULT_JWT_ISSUER: &str = "iamrusty";
/// Default JWT audience for the `AIForAll` platform
pub const DEFAULT_JWT_AUDIENCE: &str = "aiforall";

/// Claims for JWT tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject (user id)
    pub sub: String,

    /// Username
    pub username: String,

    /// Issuer
    pub iss: String,

    /// Audience
    pub aud: String,

    /// Optional organization trust-context claim (not a permission).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,

    /// JWT expiration timestamp
    pub exp: i64,

    /// JWT issued at timestamp
    pub iat: i64,

    /// JWT ID
    pub jti: String,
}

impl TokenClaims {
    /// Creates new token claims for a user
    #[must_use]
    pub fn new(user_id: &str, username: &str, expires_in: Duration) -> Self {
        Self::new_with_issuer_audience(
            user_id,
            username,
            expires_in,
            DEFAULT_JWT_ISSUER,
            DEFAULT_JWT_AUDIENCE,
        )
    }

    /// Creates new token claims with explicit issuer and audience
    #[must_use]
    pub fn new_with_issuer_audience(
        user_id: &str,
        username: &str,
        expires_in: Duration,
        issuer: &str,
        audience: &str,
    ) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            username: username.to_string(),
            iss: issuer.to_string(),
            aud: audience.to_string(),
            org: None,
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

    /// Key usage (`use` in JOSE).
    #[serde(rename = "use")]
    pub use_: String,

    /// Algorithm
    pub alg: String,

    /// Modulus (RS256)
    pub n: String,

    /// Exponent (RS256)
    pub e: String,

    /// Issuer bound to this key (custom claim; must equal JWT `iss`).
    pub iss: String,
}

impl Jwk {
    /// Build an RS256 JWK from a public PEM (never HMAC / `oct`).
    ///
    /// # Errors
    ///
    /// Returns an error string if the PEM cannot be parsed as an RSA public key.
    pub fn from_rsa_pem(public_key_pem: &str, kid: &str, issuer: &str) -> Result<Self, String> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        use rsa::{pkcs8::DecodePublicKey, traits::PublicKeyParts, RsaPublicKey};

        let rsa_pub = RsaPublicKey::from_public_key_pem(public_key_pem)
            .map_err(|e| format!("Failed to parse RSA public key PEM: {e}"))?;
        Ok(Self {
            kty: "RSA".to_string(),
            kid: kid.to_string(),
            use_: "sig".to_string(),
            alg: "RS256".to_string(),
            n: URL_SAFE_NO_PAD.encode(rsa_pub.n().to_bytes_be()),
            e: URL_SAFE_NO_PAD.encode(rsa_pub.e().to_bytes_be()),
            iss: issuer.to_string(),
        })
    }
}

impl JwkSet {
    /// Convert registry keys to JWKS. Non-RS256 and invalid RSA PEMs are skipped (never fail-closed).
    #[must_use]
    pub fn from_registry_keys(keys: &[crate::entity::signing_key::SigningKey]) -> Self {
        let mut jwks_keys = Vec::with_capacity(keys.len());
        for key in keys {
            if !key.algorithm.eq_ignore_ascii_case("RS256") {
                tracing::warn!(
                    kid = %key.kid,
                    algorithm = %key.algorithm,
                    "skipping JWKS key with non-RS256 algorithm"
                );
                continue;
            }
            match Jwk::from_rsa_pem(&key.public_key, &key.kid, &key.issuer) {
                Ok(jwk) => jwks_keys.push(jwk),
                Err(e) => {
                    tracing::warn!(
                        kid = %key.kid,
                        issuer = %key.issuer,
                        error = %e,
                        "skipping JWKS key with invalid RSA public PEM"
                    );
                }
            }
        }
        Self { keys: jwks_keys }
    }
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

impl RefreshToken {
    /// Hash a raw refresh token for at-rest storage (SHA-256 hex).
    #[must_use]
    pub fn hash_token(raw_token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(raw_token.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
