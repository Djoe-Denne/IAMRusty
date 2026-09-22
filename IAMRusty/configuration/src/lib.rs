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
    /// JWT secret storage configuration
    pub secret: SecretStorage,
    /// Access token expiration time in seconds (default: 15 minutes)
    #[serde(default = "default_jwt_expiration")]
    pub expiration_seconds: u64,
    /// Refresh token expiration time in seconds (default: 30 days)
    #[serde(default = "default_refresh_token_expiration")]
    pub refresh_token_expiration_seconds: u64,
    /// JWT issuer claim
    #[serde(default = "default_jwt_issuer")]
    pub issuer: String,
    /// JWT audience claim
    #[serde(default = "default_jwt_audience")]
    pub audience: String,
    /// HMAC secret for OAuth state (must not be the JWT secret)
    #[serde(default = "default_oauth_state_secret")]
    pub oauth_state_secret: String,
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

    /// Auth config for rustycog-http `UserIdExtractor` (HS256 only).
    ///
    /// `[jwt.secret]` is the issuer source of truth. HMAC material is copied
    /// into `AuthConfig` so IAM, Manifesto, Telegraph and Hive share one
    /// secret. RSA is rejected: rustycog-http 0.1.1 cannot verify JWKS/RS256,
    /// so an RS256 issuer would 401 every `.authenticated()` route.
    ///
    /// # Errors
    ///
    /// Returns [`SecretError`] if the issuer is RS256/RSA (HS256-only verifier)
    /// or if the HMAC secret cannot be resolved.
    pub fn http_verifier_auth(&self) -> Result<AuthConfig, SecretError> {
        if self.uses_rsa() {
            return Err(SecretError::InvalidFormat(
                "IAM issuer is RS256 but rustycog-http UserIdExtractor only verifies HS256. \
                 Configure [jwt.secret] type=\"plain\" with the same value as \
                 Manifesto/Telegraph/Hive [auth.jwt].hs256_secret. JWKS/RS256 verification \
                 is not available in rustycog-framework 0.1.1."
                    .to_string(),
            ));
        }

        match self.resolve_secret()? {
            JwtSecret::Hmac(secret) => {
                let mut auth = AuthConfig::default();
                auth.jwt.hs256_secret = Some(secret);
                auth.jwt.issuer = Some(self.issuer.clone());
                auth.jwt.audience = Some(self.audience.clone());
                Ok(auth)
            }
            JwtSecret::Rsa { .. } => Err(SecretError::InvalidFormat(
                "IAM issuer is RS256 but rustycog-http UserIdExtractor only verifies HS256."
                    .to_string(),
            )),
        }
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
            oauth_state_secret: default_oauth_state_secret(),
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
    "iam-oauth-state-hmac-change-me".to_string()
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
                oauth_state_secret: default_oauth_state_secret(),
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
    fn http_verifier_auth_rejects_rsa_issuer() {
        let jwt = JwtConfig {
            secret: SecretStorage::PemFile {
                private_key_path: "unused-private.pem".to_string(),
                public_key_path: "unused-public.pem".to_string(),
                key_id: Some("kid".to_string()),
            },
            ..JwtConfig::default()
        };

        let err = jwt
            .http_verifier_auth()
            .expect_err("RSA issuer is incompatible with rustycog-http 0.1.1");
        let message = err.to_string();
        assert!(message.contains("RS256"), "{message}");
        assert!(message.contains("HS256"), "{message}");
    }
}
