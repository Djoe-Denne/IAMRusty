//! IAM service specific configuration
//!
//! This crate provides IAM-specific configuration structures including federated IdP
//! connectors and JWT configuration, while re-exporting core configuration utilities from rustycog-config.

// Re-export core configuration from rustycog-config
pub use rustycog::config::{
    clear_all_caches, generate_default_config_toml, load_config_fresh, load_config_part,
    load_config_with_cache, AuthConfig, CommandConfig, CommandRetryConfig, ConfigError,
    DatabaseConfig, DatabaseCredentials, KafkaConfig, LoggingConfig, QueueConfig, ScalewayConfig,
    ServerConfig, SqsConfig,
};

use rustycog::config::{
    ConfigCache, ConfigLoader, HasDbConfig, HasLoggingConfig, HasQueueConfig, HasScalewayConfig,
    HasServerConfig,
};

pub use rustycog::logger::setup_logging;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use tracing::debug;

mod idp;
pub use idp::{IdpConfig, IdpConnectorConfig, IdpRedirectFlow};
pub mod security;
pub use security::{
    AuthRateLimitConfig, OAuthStateSecret, SecurityConfig, SecurityMode, TrustedAuthProxy,
};

use thiserror::Error;

/// Secret management errors
#[derive(Debug, Error)]
pub enum SecretError {
    #[error("Failed to read secret file: {0}")]
    FileReadError(String),
    #[error("Invalid secret format: {0}")]
    InvalidFormat(String),
    #[error("Secret not found: {0}")]
    NotFound(String),
}

/// Secret storage configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum SecretStorage {
    /// Plain text secret (for backward compatibility)
    #[serde(rename = "plain")]
    PlainText {
        /// The plain text secret value
        value: String,
    },
    /// PEM file-based secrets
    #[serde(rename = "pem_file")]
    PemFile {
        /// Path to the private key file
        private_key_path: String,
        /// Path to the public key file
        public_key_path: String,
        /// Optional key ID for JWKS
        key_id: Option<String>,
    },
    /// `HashiCorp` Vault (future implementation)
    #[serde(rename = "vault")]
    Vault {
        /// Vault server URL
        url: String,
        /// Secret path in vault
        secret_path: String,
        /// Authentication token
        token: String,
    },
    /// Google Cloud Secret Manager (future implementation)
    #[serde(rename = "gcp_secret_manager")]
    GcpSecretManager {
        /// GCP project ID
        project_id: String,
        /// Secret name for private key
        private_key_secret: String,
        /// Secret name for public key
        public_key_secret: String,
    },
}

/// Resolved JWT secrets
#[derive(Debug, Clone)]
pub enum JwtSecret {
    /// HMAC secret
    Hmac(String),
    /// RSA key pair
    Rsa {
        private_key: String,
        public_key: String,
        key_id: String,
    },
}

impl SecretStorage {
    /// Resolve the secret from the configured storage
    ///
    /// # Errors
    ///
    /// Returns [`SecretError`] if a PEM file cannot be read or the secret format is invalid.
    pub fn resolve(&self) -> Result<JwtSecret, SecretError> {
        match self {
            Self::PlainText { value } => {
                tracing::debug!("Resolving plain text JWT secret (length: {})", value.len());
                Ok(JwtSecret::Hmac(value.clone()))
            }
            Self::PemFile {
                private_key_path,
                public_key_path,
                key_id,
            } => {
                tracing::info!(
                    "Resolving PEM file-based JWT secret from private_key_path='{}', public_key_path='{}', key_id={:?}",
                    private_key_path,
                    public_key_path,
                    key_id
                );

                tracing::debug!("Reading private key from: {}", private_key_path);
                let private_key = fs::read_to_string(private_key_path).map_err(|e| {
                    tracing::error!(
                        "Failed to read private key from {}: {}",
                        private_key_path,
                        e
                    );
                    SecretError::FileReadError(format!(
                        "Failed to read private key from {private_key_path}: {e}"
                    ))
                })?;
                tracing::debug!(
                    "Successfully read private key ({} bytes)",
                    private_key.len()
                );

                tracing::debug!("Reading public key from: {}", public_key_path);
                let public_key = fs::read_to_string(public_key_path).map_err(|e| {
                    tracing::error!("Failed to read public key from {}: {}", public_key_path, e);
                    SecretError::FileReadError(format!(
                        "Failed to read public key from {public_key_path}: {e}"
                    ))
                })?;
                tracing::debug!("Successfully read public key ({} bytes)", public_key.len());

                let key_id = key_id.clone().unwrap_or_else(|| "default".to_string());
                tracing::info!("Successfully resolved RSA key pair with key_id: {}", key_id);

                Ok(JwtSecret::Rsa {
                    private_key,
                    public_key,
                    key_id,
                })
            }
            Self::Vault { .. } => {
                tracing::warn!("Vault secret storage requested but not yet implemented");
                Err(SecretError::InvalidFormat(
                    "Vault secret storage not yet implemented".to_string(),
                ))
            }
            Self::GcpSecretManager { .. } => {
                tracing::warn!("GCP Secret Manager requested but not yet implemented");
                Err(SecretError::InvalidFormat(
                    "GCP Secret Manager not yet implemented".to_string(),
                ))
            }
        }
    }
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret storage configuration (PEM for RS256 issuance; plain only for HS256 tests).
    pub secret: SecretStorage,
    /// Access token expiration time in seconds (default: 15 minutes)
    #[serde(default = "default_jwt_expiration")]
    pub expiration_seconds: u64,
    /// Refresh token expiration time in seconds (default: 30 days)
    #[serde(default = "default_refresh_token_expiration")]
    pub refresh_token_expiration_seconds: u64,
    /// JWT issuer claim (HS256 / legacy). Production RS256 uses [`Self::platform_issuer`].
    #[serde(default = "default_jwt_issuer")]
    pub issuer: String,
    /// JWT audience claim
    #[serde(default = "default_jwt_audience")]
    pub audience: String,
    /// Public base URL of the platform (no trailing slash). Issuers are derived from this.
    #[serde(default = "default_public_base_url")]
    pub public_base_url: String,
    /// JWKS URL for the HTTP verifier (typically `{public_base_url}/iam/.well-known/jwks.json`).
    #[serde(default)]
    pub jwks_url: Option<String>,
    /// Allowed verify algorithms for IAM's own `UserIdExtractor`.
    /// Empty → RS256-only when `jwks_url` is set.
    #[serde(default)]
    pub allowed_algorithms: Vec<String>,
    /// HMAC secret for OAuth state (must not be the JWT secret)
    #[serde(default = "default_oauth_state_secret")]
    pub oauth_state_secret: String,
    /// Optional OpenBao / Vault Transit base URL for org-signer mint + probe.
    #[serde(default)]
    pub transit_url: Option<String>,
    /// Optional Transit token (StaticCredential secret). Prefer env / secret inject.
    #[serde(default)]
    pub transit_token: Option<String>,
    /// Optional OIDC WIF / static workload identity (ADR-0307). Absent → static.
    #[serde(default)]
    pub workload: Option<WorkloadIdentityConfig>,
    /// Signing backend override: `pem` | `transit` | `remote` (ADR-0309).
    #[serde(default)]
    pub backend: Option<String>,
    /// Alias of [`Self::backend`] (`provider = "remote"` also selects remote HTTP).
    #[serde(default)]
    pub provider: Option<String>,
    /// Remote Sign / GetPublicKey endpoint (ADR-0309). Absent → PEM / Transit default.
    #[serde(default)]
    pub remote: Option<RemoteSignerConfig>,
}

/// `[jwt.remote]` — HTTP remote signer (digest only; no private key).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RemoteSignerConfig {
    /// Base URL (`POST {url}/sign`, `GET {url}/keys/{key_id}`).
    #[serde(default)]
    pub url: String,
    /// Opaque remote key id (never a private key).
    #[serde(default)]
    pub key_id: String,
    /// Optional static secret for [`crate::JwtConfig::workload`] fallback.
    #[serde(default)]
    pub token: Option<String>,
}

/// Cloud WIF / static workload identity selection (ADR-0307).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct WorkloadIdentityConfig {
    /// `static` | `aws` | `gcp` | `azure`. Absent → static.
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub aws: Option<AwsWorkloadConfig>,
    #[serde(default)]
    pub gcp: Option<GcpWorkloadConfig>,
    #[serde(default)]
    pub azure: Option<AzureWorkloadConfig>,
}

/// AWS STS `AssumeRoleWithWebIdentity` WIF parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AwsWorkloadConfig {
    pub token_url: String,
    pub subject_token_file: String,
    pub role_arn: String,
    pub role_session_name: String,
    pub audience: String,
}

/// GCP STS token-exchange WIF parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GcpWorkloadConfig {
    pub token_url: String,
    pub subject_token_file: String,
    pub audience: String,
}

/// Azure AD client-assertion WIF parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AzureWorkloadConfig {
    pub token_url: String,
    pub subject_token_file: String,
    pub tenant_id: String,
    pub client_id: String,
    pub scope: String,
}

impl JwtConfig {
    /// Resolve the JWT secret from the configured storage
    ///
    /// # Errors
    ///
    /// Returns [`SecretError`] if the configured storage cannot be resolved.
    pub fn resolve_secret(&self) -> Result<JwtSecret, SecretError> {
        self.secret.resolve()
    }

    /// Get the resolved secret as a string (for HMAC compatibility)
    /// This method provides backward compatibility for HMAC-based JWT services
    ///
    /// # Errors
    ///
    /// Returns [`SecretError`] if the secret cannot be resolved or is an RSA key pair.
    pub fn get_secret_string(&self) -> Result<String, SecretError> {
        match self.resolve_secret()? {
            JwtSecret::Hmac(secret) => Ok(secret),
            JwtSecret::Rsa { .. } => Err(SecretError::InvalidFormat(
                "Cannot convert RSA key pair to HMAC secret string".to_string(),
            )),
        }
    }

    /// Check if the configuration uses RSA keys
    #[must_use]
    pub const fn uses_rsa(&self) -> bool {
        matches!(self.secret, SecretStorage::PemFile { .. })
    }

    /// Check if the configuration uses HMAC
    #[must_use]
    pub const fn uses_hmac(&self) -> bool {
        matches!(self.secret, SecretStorage::PlainText { .. })
    }

    /// Create a `JwtAlgorithm` from this configuration
    /// This method bridges the configuration with the JWT encoder implementation
    ///
    /// # Errors
    ///
    /// Returns [`SecretError`] if the JWT secret cannot be resolved.
    pub fn create_jwt_algorithm(&self) -> Result<JwtAlgorithm, SecretError> {
        match self.resolve_secret()? {
            JwtSecret::Hmac(secret) => Ok(JwtAlgorithm::HS256(secret)),
            JwtSecret::Rsa {
                private_key,
                public_key,
                key_id,
            } => Ok(JwtAlgorithm::RS256(JwtKeyPair {
                private_key,
                public_key,
                kid: key_id,
            })),
        }
    }

    /// Auth config for rustycog-http `UserIdExtractor` (RS256 JWKS + optional HS256 window).
    ///
    /// Production / default: `allowed_algorithms=["RS256"]`, `jwks_url` set, no HS256 secret.
    /// IAM seeds inline JWKS at setup so it does not HTTP-call itself before listen.
    ///
    /// # Errors
    ///
    /// Returns [`SecretError`] if RSA PEM material cannot be resolved when required for
    /// issuer setup, or if neither JWKS nor HS256 material is configured.
    pub fn http_verifier_auth(&self) -> Result<AuthConfig, SecretError> {
        let mut auth = AuthConfig::default();
        auth.jwt.audience = Some(self.audience.clone());

        let jwks_url = self.effective_jwks_url();
        if let Some(url) = jwks_url {
            auth.jwt.jwks_url = Some(url);
            auth.jwt.allowed_algorithms = if self.allowed_algorithms.is_empty() {
                vec!["RS256".to_string()]
            } else {
                self.allowed_algorithms.clone()
            };
            // RS256 issuer URL is carried by JWK `iss`; do not force HS256 issuer here.
            if auth.jwt.allowed_algorithms.iter().any(|a| a == "HS256") {
                if let Ok(JwtSecret::Hmac(secret)) = self.resolve_secret() {
                    auth.jwt.hs256_secret = Some(secret);
                    auth.jwt.issuer = Some(self.issuer.clone());
                }
            }
            return Ok(auth);
        }

        // Legacy HS256-only path (tests that have not migrated yet).
        match self.resolve_secret()? {
            JwtSecret::Hmac(secret) => {
                auth.jwt.hs256_secret = Some(secret);
                auth.jwt.issuer = Some(self.issuer.clone());
                auth.jwt.allowed_algorithms = vec!["HS256".to_string()];
                Ok(auth)
            }
            JwtSecret::Rsa { .. } => Err(SecretError::InvalidFormat(
                "RS256 issuer requires jwks_url (or public_base_url) for UserIdExtractor"
                    .to_string(),
            )),
        }
    }

    /// Platform issuer URL: `{public_base_url}/iam`.
    #[must_use]
    pub fn platform_issuer(&self) -> String {
        format!("{}/iam", self.public_base_url.trim_end_matches('/'))
    }

    /// Organization issuer URL: `{public_base_url}/iam/orgs/{slug}`.
    #[must_use]
    pub fn organization_issuer(&self, org_slug: &str) -> String {
        format!("{}/orgs/{org_slug}", self.platform_issuer())
    }

    /// Signing backend string (`backend` then `provider`).
    #[must_use]
    pub fn signing_backend(&self) -> Option<&str> {
        self.backend
            .as_deref()
            .or(self.provider.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    /// Whether config selects the remote HTTP signer (ADR-0309).
    #[must_use]
    pub fn remote_is_requested(&self) -> bool {
        self.signing_backend()
            .is_some_and(|s| s.eq_ignore_ascii_case("remote"))
    }

    /// Remote signer block when `backend`/`provider` = `remote`.
    ///
    /// Returns `Ok(None)` unless remote is explicitly requested — a bare `jwt.remote.url`
    /// without `backend`/`provider` = `remote` is ignored (not activated).
    ///
    /// # Errors
    ///
    /// Returns [`SecretError::InvalidFormat`] when remote is requested and `jwt.remote.url` is empty.
    pub fn remote_http_endpoint(&self) -> Result<Option<&RemoteSignerConfig>, SecretError> {
        if !self.remote_is_requested() {
            return Ok(None);
        }
        let url = self.remote.as_ref().map(|r| r.url.trim()).unwrap_or("");
        if url.is_empty() {
            return Err(SecretError::InvalidFormat(
                "remote signer url absent — fail-closed".to_string(),
            ));
        }
        Ok(self.remote.as_ref())
    }

    /// Effective JWKS URL: explicit `jwks_url` or derived from `public_base_url`.
    #[must_use]
    pub fn effective_jwks_url(&self) -> Option<String> {
        if let Some(url) = &self.jwks_url {
            if !url.trim().is_empty() {
                return Some(url.clone());
            }
        }
        let base = self.public_base_url.trim();
        if base.is_empty() {
            None
        } else {
            Some(format!(
                "{}/iam/.well-known/jwks.json",
                base.trim_end_matches('/')
            ))
        }
    }

    /// Resolve optional Transit URL + token from `[jwt].transit_*` or `secret = vault`.
    #[must_use]
    pub fn transit_endpoint(&self) -> Option<(String, String)> {
        if let (Some(url), Some(token)) = (&self.transit_url, &self.transit_token) {
            let url = url.trim();
            let token = token.trim();
            if !url.is_empty() && !token.is_empty() {
                return Some((url.to_string(), token.to_string()));
            }
        }
        if let SecretStorage::Vault { url, token, .. } = &self.secret {
            let url = url.trim();
            let token = token.trim();
            if !url.is_empty() && !token.is_empty() {
                return Some((url.to_string(), token.to_string()));
            }
        }
        None
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: SecretStorage::PlainText {
                value: String::new(),
            },
            expiration_seconds: default_jwt_expiration(),
            refresh_token_expiration_seconds: default_refresh_token_expiration(),
            issuer: default_jwt_issuer(),
            audience: default_jwt_audience(),
            public_base_url: default_public_base_url(),
            jwks_url: None,
            allowed_algorithms: Vec::new(),
            oauth_state_secret: default_oauth_state_secret(),
            transit_url: None,
            transit_token: None,
            workload: None,
            backend: None,
            provider: None,
            remote: None,
        }
    }
}

/// JWT algorithm configuration
/// This is re-exported from the infra crate to avoid circular dependencies
/// It should match the `JwtAlgorithm` enum in `infra::token::jwt_encoder`
#[derive(Debug, Clone)]
pub enum JwtAlgorithm {
    /// RSA256 with key pair
    RS256(JwtKeyPair),
    /// HMAC256 with secret
    HS256(String),
}

/// JWT key pair for token signing and verification
/// This is re-exported from the domain to avoid circular dependencies
#[derive(Debug, Clone)]
pub struct JwtKeyPair {
    /// Private key (RS256)
    pub private_key: String,
    /// Public key (RS256)
    pub public_key: String,
    /// Key ID
    pub kid: String,
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Explicit boot-time security and authentication abuse policy.
    #[serde(default)]
    pub security: SecurityConfig,
    /// Server configuration
    pub server: ServerConfig,
    /// Shared authentication verifier configuration
    #[serde(default)]
    pub auth: AuthConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Federated IdP connector registry (`[[idp.connectors]]`)
    #[serde(default)]
    pub idp: IdpConfig,
    /// JWT configuration
    pub jwt: JwtConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Scaleway configuration
    #[serde(default)]
    pub scaleway: ScalewayConfig,
    /// Command configuration
    #[serde(default)]
    pub command: CommandConfig,
    /// Queue configuration (Kafka, SQS, or Disabled)
    pub queue: QueueConfig,
    /// Legacy Kafka configuration (for backward compatibility)
    #[serde(default)]
    pub kafka: KafkaConfig,
    /// Shared secret for internal `IdP` token routes
    #[serde(default)]
    pub internal_service_token: String,
}

// Default value functions
const fn default_jwt_expiration() -> u64 {
    900 // 15 minutes
}

const fn default_refresh_token_expiration() -> u64 {
    2_592_000 // 30 days (30 * 24 * 60 * 60)
}

fn default_jwt_issuer() -> String {
    "iamrusty".to_string()
}

fn default_jwt_audience() -> String {
    "aiforall".to_string()
}

fn default_oauth_state_secret() -> String {
    String::new()
}

fn default_public_base_url() -> String {
    "http://127.0.0.1:8080".to_string()
}

/// Global configuration cache
static CONFIG_CACHE: OnceLock<Arc<Mutex<Option<AppConfig>>>> = OnceLock::new();

/// Configuration cache implementation for `AppConfig`
pub struct AppConfigCache;

impl ConfigCache<AppConfig> for AppConfigCache {
    fn get_cached() -> Option<AppConfig> {
        let cache = CONFIG_CACHE.get_or_init(|| Arc::new(Mutex::new(None)));
        cache.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn set_cached(config: AppConfig) {
        let cache = CONFIG_CACHE.get_or_init(|| Arc::new(Mutex::new(None)));
        *cache.lock().unwrap_or_else(PoisonError::into_inner) = Some(config);
    }

    fn clear_cached() {
        let cache = CONFIG_CACHE.get_or_init(|| Arc::new(Mutex::new(None)));
        *cache.lock().unwrap_or_else(PoisonError::into_inner) = None;
        debug!("AppConfig cache cleared");
    }
}

/// Configuration loader implementation for `AppConfig`
impl ConfigLoader<Self> for AppConfig {
    fn create_default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
            security: SecurityConfig::default(),
            database: DatabaseConfig::default(),
            idp: IdpConfig::default(),
            jwt: JwtConfig {
                secret: SecretStorage::PlainText {
                    value: "your-256-bit-secret-key-change-this-in-production".to_string(),
                },
                expiration_seconds: default_jwt_expiration(),
                refresh_token_expiration_seconds: default_refresh_token_expiration(),
                issuer: default_jwt_issuer(),
                audience: default_jwt_audience(),
                public_base_url: default_public_base_url(),
                jwks_url: None,
                allowed_algorithms: vec!["RS256".to_string()],
                oauth_state_secret: default_oauth_state_secret(),
                transit_url: None,
                transit_token: None,
                workload: None,
                backend: None,
                provider: None,
                remote: None,
            },
            logging: LoggingConfig::default(),
            scaleway: ScalewayConfig::default(),
            command: CommandConfig::default(),
            queue: QueueConfig::default(),
            kafka: KafkaConfig::default(),
            internal_service_token: String::new(),
        }
    }

    fn config_prefix() -> &'static str {
        "IAM"
    }
}

impl HasDbConfig for AppConfig {
    fn db_config(&self) -> &DatabaseConfig {
        &self.database
    }

    fn set_db_config(&mut self, config: DatabaseConfig) {
        self.database = config;
    }
}

impl HasQueueConfig for AppConfig {
    fn queue_config(&self) -> &QueueConfig {
        &self.queue
    }

    fn set_queue_config(&mut self, config: QueueConfig) {
        self.queue = config;
    }
}

impl HasServerConfig for AppConfig {
    fn server_config(&self) -> &ServerConfig {
        &self.server
    }

    fn set_server_config(&mut self, config: ServerConfig) {
        self.server = config;
    }
}

impl HasLoggingConfig for AppConfig {
    fn logging_config(&self) -> &LoggingConfig {
        &self.logging
    }

    fn set_logging_config(&mut self, config: LoggingConfig) {
        self.logging = config;
    }
}

impl HasScalewayConfig for AppConfig {
    fn scaleway_config(&self) -> &ScalewayConfig {
        &self.scaleway
    }

    fn set_scaleway_config(&mut self, config: ScalewayConfig) {
        self.scaleway = config;
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::create_default()
    }
}

/// Load configuration from environment and config files
/// This function caches the configuration to ensure consistent behavior,
/// especially for random port generation in database configuration.
///
/// # Errors
///
/// Returns [`ConfigError`] if configuration files or environment cannot be loaded.
pub fn load_config() -> Result<AppConfig, ConfigError> {
    load_config_with_cache::<AppConfig, AppConfigCache>()
}

/// Clear the configuration cache
pub fn clear_config_cache() {
    AppConfigCache::clear_cached();
}

/// Generate a default configuration file in TOML format
///
/// # Errors
///
/// Returns [`ConfigError`] if the default configuration cannot be serialized to TOML.
pub fn generate_default_config() -> Result<String, ConfigError> {
    generate_default_config_toml::<AppConfig>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 0);
        assert_eq!(config.jwt.expiration_seconds, 900);
    }

    #[test]
    fn test_config_cache() {
        // Clear any existing cache
        AppConfigCache::clear_cached();

        // Should return None initially
        assert!(AppConfigCache::get_cached().is_none());

        // Set a config
        let config = AppConfig::default();
        AppConfigCache::set_cached(config.clone());

        // Should return the cached config
        let cached = AppConfigCache::get_cached().unwrap();
        assert_eq!(cached.server.host, config.server.host);

        // Clear cache
        AppConfigCache::clear_cached();
        assert!(AppConfigCache::get_cached().is_none());
    }

    #[test]
    fn test_generate_default_config() {
        let toml_config = generate_default_config().expect("Should generate default config");
        assert!(toml_config.contains("[server]"));
        assert!(toml_config.contains("[database]"));
        assert!(toml_config.contains("[idp]"));
        assert!(toml_config.contains("[jwt]"));
    }

    #[test]
    fn http_verifier_auth_copies_hmac_issuer_secret() {
        let jwt = JwtConfig {
            secret: SecretStorage::PlainText {
                value: "rustycog-dev-hs256-secret".to_string(),
            },
            public_base_url: String::new(),
            jwks_url: None,
            ..JwtConfig::default()
        };

        let auth = jwt
            .http_verifier_auth()
            .expect("HMAC issuer must map to AuthConfig");
        assert_eq!(
            auth.jwt.hs256_secret.as_deref(),
            Some("rustycog-dev-hs256-secret")
        );
    }

    #[test]
    fn http_verifier_auth_accepts_rsa_issuer() {
        let jwt = JwtConfig {
            secret: SecretStorage::PemFile {
                private_key_path: "unused-private.pem".to_string(),
                public_key_path: "unused-public.pem".to_string(),
                key_id: Some("kid".to_string()),
            },
            public_base_url: "http://127.0.0.1:8080".to_string(),
            jwks_url: Some("http://127.0.0.1:8080/iam/.well-known/jwks.json".to_string()),
            allowed_algorithms: vec!["RS256".to_string()],
            ..JwtConfig::default()
        };

        let auth = jwt
            .http_verifier_auth()
            .expect("RSA issuer must map to JWKS AuthConfig");
        assert_eq!(
            auth.jwt.jwks_url.as_deref(),
            Some("http://127.0.0.1:8080/iam/.well-known/jwks.json")
        );
        assert_eq!(auth.jwt.allowed_algorithms, vec!["RS256".to_string()]);
        assert!(auth.jwt.hs256_secret.is_none());
    }

    #[test]
    fn http_verifier_auth_rs256_only_excludes_hs256() {
        let jwt = JwtConfig {
            secret: SecretStorage::PemFile {
                private_key_path: "unused-private.pem".to_string(),
                public_key_path: "unused-public.pem".to_string(),
                key_id: Some("kid".to_string()),
            },
            public_base_url: "http://127.0.0.1:8080".to_string(),
            jwks_url: Some("http://127.0.0.1:8080/iam/.well-known/jwks.json".to_string()),
            allowed_algorithms: vec!["RS256".to_string()],
            ..JwtConfig::default()
        };
        let auth = jwt.http_verifier_auth().expect("RS256 config");
        assert_eq!(auth.jwt.allowed_algorithms, vec!["RS256".to_string()]);
        assert!(!auth.jwt.allowed_algorithms.iter().any(|a| a == "HS256"));
        assert!(auth.jwt.hs256_secret.is_none());
    }
}
