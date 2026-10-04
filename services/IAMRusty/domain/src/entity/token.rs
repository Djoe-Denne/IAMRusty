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
#[derive(Clone)]
pub struct JwtKeyPair {
    /// Private key (RS256)
    pub private_key: String,

    /// Public key (RS256)
    pub public_key: String,

    /// Key ID
    pub kid: String,
}

impl std::fmt::Debug for JwtKeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtKeyPair")
            .field("kid", &self.kid)
            .finish_non_exhaustive()
    }
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
    /// Registry lifecycle metadata. Missing values never imply Active.
    #[serde(default)]
    pub status: Option<crate::entity::signing_key::SigningKeyStatus>,
    #[serde(default)]
    pub trust_scope: Option<crate::entity::signing_key::TrustScope>,
    #[serde(default)]
    pub organization_id: Option<Uuid>,
}

impl Jwk {
    /// Build an RS256 JWK from a public PEM (never HMAC / `oct`).
    ///
    /// # Errors
    ///
    /// Returns an error string if the PEM cannot be parsed as an RSA public key.
    pub fn from_rsa_pem(public_key_pem: &str, kid: &str, issuer: &str) -> Result<Self, String> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        use rsa::traits::PublicKeyParts;

        let rsa_pub = crate::entity::signing_key::parse_signing_public_key(public_key_pem)
            .map_err(|_| "invalid RSA public key".to_string())?;
        Ok(Self {
            kty: "RSA".to_string(),
            kid: kid.to_string(),
            use_: "sig".to_string(),
            alg: "RS256".to_string(),
            n: URL_SAFE_NO_PAD.encode(rsa_pub.n().to_bytes_be()),
            e: URL_SAFE_NO_PAD.encode(rsa_pub.e().to_bytes_be()),
            iss: issuer.to_string(),
            status: None,
            trust_scope: None,
            organization_id: None,
        })
    }
}

impl JwkSet {
    /// Complete canonical DTO, never silently drop an admissible row.
    pub fn from_registry_keys_checked(
        keys: &[crate::entity::signing_key::SigningKey],
    ) -> Result<Self, crate::error::DomainError> {
        let mut kids = std::collections::HashSet::new();
        if keys.iter().any(|key| {
            !kids.insert(&key.kid)
                || key.algorithm != "RS256"
                || key.issuer.is_empty()
                || key.status == crate::entity::signing_key::SigningKeyStatus::Revoked
                || (key.trust_scope == crate::entity::signing_key::TrustScope::Platform)
                    != key.organization_id.is_none()
                || Jwk::from_rsa_pem(&key.public_key, &key.kid, &key.issuer).is_err()
        }) {
            return Err(crate::error::DomainError::InvalidSigningKeyMaterial);
        }
        Ok(Self::from_registry_keys(keys))
    }

    /// Exact serde compact UTF8 shared by publication DTO and reservation math.
    pub fn compact_bytes(&self) -> Result<Vec<u8>, crate::error::DomainError> {
        serde_json::to_vec(self).map_err(|_| crate::error::DomainError::InvalidSigningKeyMaterial)
    }
    /// Convert valid registry public material to JWKS with actual lifecycle/trust
    /// metadata. Revoked/invalid bindings, non-RS256 and invalid PEMs are excluded.
    #[must_use]
    pub fn from_registry_keys(keys: &[crate::entity::signing_key::SigningKey]) -> Self {
        let mut jwks_keys = Vec::with_capacity(keys.len());
        for key in keys {
            use crate::entity::signing_key::{SigningKeyStatus, TrustScope};
            if key.status == SigningKeyStatus::Revoked
                || key.issuer.is_empty()
                || (key.trust_scope == TrustScope::Platform) != key.organization_id.is_none()
            {
                continue;
            }
            if !key.algorithm.eq_ignore_ascii_case("RS256") {
                tracing::warn!(
                    kid = %key.kid,
                    algorithm = %key.algorithm,
                    "skipping JWKS key with non-RS256 algorithm"
                );
                continue;
            }
            match Jwk::from_rsa_pem(&key.public_key, &key.kid, &key.issuer) {
                Ok(mut jwk) => {
                    jwk.status = Some(key.status.clone());
                    jwk.trust_scope = Some(key.trust_scope.clone());
                    jwk.organization_id = key.organization_id;
                    jwks_keys.push(jwk);
                }
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
