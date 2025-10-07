//! IAM service specific configuration
//!
//! This crate provides IAM-specific configuration structures including OAuth and JWT
//! configuration, while re-exporting core configuration utilities from rustycog-config.

// Re-export core configuration from rustycog-config
pub use rustycog_config::{
    clear_all_caches, generate_default_config_toml, load_config_fresh, load_config_part,
    load_config_with_cache, CommandConfig, CommandRetryConfig, ConfigError,
    DatabaseConfig, DatabaseCredentials, KafkaConfig, LoggingConfig, QueueConfig, ServerConfig,
    SqsConfig, ScalewayConfig,
};

use rustycog_config::{
    ConfigCache, ConfigLoader, HasDbConfig, HasLoggingConfig, HasQueueConfig, HasServerConfig, HasScalewayConfig,
};

pub use rustycog_logger::{setup_logging};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::{Arc, Mutex, OnceLock};
use tracing::debug;

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// HashiCorp Vault (future implementation)
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
    pub fn resolve(&self) -> Result<JwtSecret, SecretError> {
        match self {
            SecretStorage::PlainText { value } => {
                tracing::debug!("Resolving plain text JWT secret (length: {})", value.len());
                Ok(JwtSecret::Hmac(value.clone()))
            }
            SecretStorage::PemFile {
                private_key_path,
                public_key_path,
                key_id,
            } => {
                tracing::info!("Resolving PEM file-based JWT secret from private_key_path='{}', public_key_path='{}', key_id={:?}", 
                    private_key_path, public_key_path, key_id);

                tracing::debug!("Reading private key from: {}", private_key_path);
                let private_key = fs::read_to_string(private_key_path).map_err(|e| {
                    tracing::error!(
                        "Failed to read private key from {}: {}",
                        private_key_path,
                        e
                    );
                    SecretError::FileReadError(format!(
                        "Failed to read private key from {}: {}",
                        private_key_path, e
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
                        "Failed to read public key from {}: {}",
                        public_key_path, e
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
            SecretStorage::Vault { .. } => {
                tracing::warn!("Vault secret storage requested but not yet implemented");
                Err(SecretError::InvalidFormat(
                    "Vault secret storage not yet implemented".to_string(),
                ))
            }
            SecretStorage::GcpSecretManager { .. } => {
                tracing::warn!("GCP Secret Manager requested but not yet implemented");
                Err(SecretError::InvalidFormat(
                    "GCP Secret Manager not yet implemented".to_string(),
                ))
            }
        }
    }
}

/// OAuth configuration containing provider-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// GitHub OAuth configuration
    pub github: GitHubConfig,
    /// GitLab OAuth configuration
    pub gitlab: GitLabConfig,
}

/// GitHub OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    /// GitHub OAuth client ID
    pub client_id: String,
    /// GitHub OAuth client secret
    pub client_secret: String,
    /// OAuth redirect URI
    pub redirect_uri: String,
    /// GitHub authorization URL
    #[serde(default = "default_github_auth_url")]
    pub auth_url: String,
    /// GitHub token exchange URL
    #[serde(default = "default_github_token_url")]
    pub token_url: String,
    /// GitHub user info API URL
    #[serde(default = "default_github_user_url")]
    pub user_url: String,
}

/// GitLab OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabConfig {
    /// GitLab OAuth client ID
    pub client_id: String,
    /// GitLab OAuth client secret
    pub client_secret: String,
    /// OAuth redirect URI
    pub redirect_uri: String,
    /// GitLab authorization URL
    #[serde(default = "default_gitlab_auth_url")]
    pub auth_url: String,
    /// GitLab token exchange URL
    #[serde(default = "default_gitlab_token_url")]
    pub token_url: String,
    /// GitLab user info API URL
    #[serde(default = "default_gitlab_user_url")]
    pub user_url: String,
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
}

impl JwtConfig {
    /// Resolve the JWT secret from the configured storage
    pub fn resolve_secret(&self) -> Result<JwtSecret, SecretError> {
        self.secret.resolve()
    }

    /// Get the resolved secret as a string (for HMAC compatibility)
    /// This method provides backward compatibility for HMAC-based JWT services
    pub fn get_secret_string(&self) -> Result<String, SecretError> {
        match self.resolve_secret()? {
            JwtSecret::Hmac(secret) => Ok(secret),
            JwtSecret::Rsa { .. } => Err(SecretError::InvalidFormat(
                "Cannot convert RSA key pair to HMAC secret string".to_string(),
            )),
        }
    }

    /// Check if the configuration uses RSA keys
    pub fn uses_rsa(&self) -> bool {
        matches!(self.secret, SecretStorage::PemFile { .. })
    }

    /// Check if the configuration uses HMAC
    pub fn uses_hmac(&self) -> bool {
        matches!(self.secret, SecretStorage::PlainText { .. })
    }

    /// Create a JwtAlgorithm from this configuration
    /// This method bridges the configuration with the JWT encoder implementation
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
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: SecretStorage::PlainText {
                value: "".to_string(),
            },
            expiration_seconds: default_jwt_expiration(),
            refresh_token_expiration_seconds: default_refresh_token_expiration(),
        }
    }
}

/// JWT algorithm configuration
/// This is re-exported from the infra crate to avoid circular dependencies
/// It should match the JwtAlgorithm enum in infra::token::jwt_encoder
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
    /// Database configuration
    pub database: DatabaseConfig,
    /// OAuth provider configurations
    pub oauth: OAuthConfig,
    /// JWT configuration
    pub jwt: JwtConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Scaleway configuration
    pub scaleway: ScalewayConfig,
    /// Command configuration
    pub command: CommandConfig,
    /// Queue configuration (Kafka, SQS, or Disabled)
    pub queue: QueueConfig,
    /// Legacy Kafka configuration (for backward compatibility)
    #[serde(default)]
    pub kafka: KafkaConfig,
}

// Default value functions
fn default_github_auth_url() -> String {
    "https://github.com/login/oauth/authorize".to_string()
}

fn default_github_token_url() -> String {
    "https://github.com/login/oauth/access_token".to_string()
}

fn default_github_user_url() -> String {
    "https://api.github.com/user".to_string()
}

fn default_gitlab_auth_url() -> String {
    "https://gitlab.com/oauth/authorize".to_string()
}

fn default_gitlab_token_url() -> String {
    "https://gitlab.com/oauth/token".to_string()
}

fn default_gitlab_user_url() -> String {
    "https://gitlab.com/api/v4/user".to_string()
}

fn default_jwt_expiration() -> u64 {
    900 // 15 minutes
}

fn default_refresh_token_expiration() -> u64 {
    2_592_000 // 30 days (30 * 24 * 60 * 60)
}

/// Generic provider configuration for conversion utilities
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Redirect URI
    pub redirect_uri: String,
}

impl From<&GitHubConfig> for ProviderConfig {
    fn from(config: &GitHubConfig) -> Self {
        ProviderConfig {
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            redirect_uri: config.redirect_uri.clone(),
        }
    }
}

impl From<&GitLabConfig> for ProviderConfig {
    fn from(config: &GitLabConfig) -> Self {
        ProviderConfig {
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            redirect_uri: config.redirect_uri.clone(),
        }
    }
}

// Type aliases for backward compatibility
pub type GithubConfig = GitHubConfig;
pub type GitlabConfig = GitLabConfig;

/// Global configuration cache
static CONFIG_CACHE: OnceLock<Arc<Mutex<Option<AppConfig>>>> = OnceLock::new();

/// Configuration cache implementation for AppConfig
pub struct AppConfigCache;

impl ConfigCache<AppConfig> for AppConfigCache {
    fn get_cached() -> Option<AppConfig> {
        let cache = CONFIG_CACHE.get_or_init(|| Arc::new(Mutex::new(None)));
        let cached_config = cache.lock().unwrap();
        cached_config.clone()
    }

    fn set_cached(config: AppConfig) {
        let cache = CONFIG_CACHE.get_or_init(|| Arc::new(Mutex::new(None)));
        let mut cached_config = cache.lock().unwrap();
        *cached_config = Some(config);
    }

    fn clear_cached() {
        let cache = CONFIG_CACHE.get_or_init(|| Arc::new(Mutex::new(None)));
        let mut cached_config = cache.lock().unwrap();
        *cached_config = None;
        debug!("AppConfig cache cleared");
    }
}

/// Configuration loader implementation for AppConfig
impl ConfigLoader<AppConfig> for AppConfig {
    fn create_default() -> AppConfig {
        AppConfig {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            oauth: OAuthConfig {
                github: GitHubConfig {
                    client_id: "YOUR_GITHUB_CLIENT_ID".to_string(),
                    client_secret: "YOUR_GITHUB_CLIENT_SECRET".to_string(),
                    redirect_uri: "http://localhost:8080/auth/github/callback".to_string(),
                    auth_url: default_github_auth_url(),
                    token_url: default_github_token_url(),
                    user_url: default_github_user_url(),
                },
                gitlab: GitLabConfig {
                    client_id: "YOUR_GITLAB_CLIENT_ID".to_string(),
                    client_secret: "YOUR_GITLAB_CLIENT_SECRET".to_string(),
                    redirect_uri: "http://localhost:8080/auth/gitlab/callback".to_string(),
                    auth_url: default_gitlab_auth_url(),
                    token_url: default_gitlab_token_url(),
                    user_url: default_gitlab_user_url(),
                },
            },
            jwt: JwtConfig {
                secret: SecretStorage::PlainText {
                    value: "your-256-bit-secret-key-change-this-in-production".to_string(),
                },
                expiration_seconds: default_jwt_expiration(),
                refresh_token_expiration_seconds: default_refresh_token_expiration(),
            },
            logging: LoggingConfig::default(),
            scaleway: ScalewayConfig::default(),
            command: CommandConfig::default(),
            queue: QueueConfig::default(),
            kafka: KafkaConfig::default(),
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
pub fn load_config() -> Result<AppConfig, ConfigError> {
    load_config_with_cache::<AppConfig, AppConfigCache>()
}

/// Clear the configuration cache
pub fn clear_config_cache() {
    AppConfigCache::clear_cached();
}

/// Generate a default configuration file in TOML format
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
        assert!(toml_config.contains("[oauth.github]"));
        assert!(toml_config.contains("[jwt]"));
    }

    #[test]
    fn test_provider_config_conversion() {
        let github_config = GitHubConfig {
            client_id: "test_id".to_string(),
            client_secret: "test_secret".to_string(),
            redirect_uri: "http://localhost:8080/callback".to_string(),
            auth_url: default_github_auth_url(),
            token_url: default_github_token_url(),
            user_url: default_github_user_url(),
        };

        let provider_config: ProviderConfig = (&github_config).into();
        assert_eq!(provider_config.client_id, "test_id");
        assert_eq!(provider_config.client_secret, "test_secret");
        assert_eq!(
            provider_config.redirect_uri,
            "http://localhost:8080/callback"
        );
    }
}
