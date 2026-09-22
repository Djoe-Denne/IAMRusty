//! Typed configuration for Lazaret.

use lazaret_domain::{DEFAULT_CERT_TTL_HOURS, DEFAULT_SESSION_TTL_MINUTES};
use rustycog::config::{
    AuthConfig, HasDbConfig, HasLoggingConfig, HasQueueConfig, HasServerConfig,
};
use serde::{Deserialize, Serialize};

pub use rustycog::config::{
    load_config_fresh, CommandConfig, ConfigError, ConfigLoader, DatabaseConfig, LoggingConfig,
    QueueConfig, ServerConfig,
};

pub use rustycog::logger::setup_logging;

/// Dedicated workload-identity settings. Distinct from [`AuthConfig`] / `[auth.jwt]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Session token TTL in minutes (implementation default [`DEFAULT_SESSION_TTL_MINUTES`]).
    #[serde(default = "default_session_ttl_minutes")]
    pub session_ttl_minutes: u64,
    /// Client certificate TTL in hours (implementation default [`DEFAULT_CERT_TTL_HOURS`]).
    #[serde(default = "default_cert_ttl_hours")]
    pub cert_ttl_hours: u64,
    /// Optional dedicated Ed25519 PKCS#8 PEM. Empty/dev → ephemeral key at boot.
    #[serde(default)]
    pub session_signing_key_pem: Option<String>,
    /// Platform CA certificate PEM path. Empty/whitespace → ephemeral in-process CA.
    #[serde(default)]
    pub ca_cert_pem_path: String,
    /// Platform CA private-key PEM path. Empty/whitespace → ephemeral in-process CA.
    #[serde(default)]
    pub ca_key_pem_path: String,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            session_ttl_minutes: default_session_ttl_minutes(),
            cert_ttl_hours: default_cert_ttl_hours(),
            session_signing_key_pem: None,
            ca_cert_pem_path: String::new(),
            ca_key_pem_path: String::new(),
        }
    }
}

const fn default_session_ttl_minutes() -> u64 {
    DEFAULT_SESSION_TTL_MINUTES
}

const fn default_cert_ttl_hours() -> u64 {
    DEFAULT_CERT_TTL_HOURS
}

/// Manifesto live consult (binding / consents / grants). Distinct from `[auth.jwt]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestoServiceConfig {
    /// Manifesto HTTP prefix (`http://manifesto-service:8080/manifesto` in development).
    #[serde(default = "default_manifesto_base_url")]
    pub base_url: String,
    /// HTTP timeout in seconds.
    #[serde(default = "default_manifesto_timeout_seconds")]
    pub timeout_seconds: u64,
    /// Pre-signed Bearer override. Non-empty skips HS256 signing.
    #[serde(default)]
    pub bearer_token: String,
    /// HS256 secret used to sign the snapshot GET when `bearer_token` is empty.
    #[serde(default)]
    pub hs256_secret: Option<String>,
    /// Snapshot JWT `iss` claim.
    #[serde(default = "default_grant_snapshot_issuer")]
    pub issuer: String,
    /// Snapshot JWT `aud` claim.
    #[serde(default = "default_grant_snapshot_audience")]
    pub audience: String,
}

impl Default for ManifestoServiceConfig {
    fn default() -> Self {
        Self {
            base_url: default_manifesto_base_url(),
            timeout_seconds: default_manifesto_timeout_seconds(),
            bearer_token: String::new(),
            hs256_secret: None,
            issuer: default_grant_snapshot_issuer(),
            audience: default_grant_snapshot_audience(),
        }
    }
}

fn default_manifesto_base_url() -> String {
    "http://127.0.0.1:8080/manifesto".to_owned()
}

const fn default_manifesto_timeout_seconds() -> u64 {
    10
}

fn default_grant_snapshot_issuer() -> String {
    "aiforall-platform".to_owned()
}

fn default_grant_snapshot_audience() -> String {
    "manifesto-bindings".to_owned()
}

/// Which KV adapter is active (`postgres` or `redis`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvConfig {
    /// Adapter name.
    #[serde(default = "default_kv_backend")]
    pub backend: String,
}

impl Default for KvConfig {
    fn default() -> Self {
        Self {
            backend: default_kv_backend(),
        }
    }
}

fn default_kv_backend() -> String {
    "postgres".to_owned()
}

/// Redis connection (used when [`KvConfig::backend`] is `redis`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis hostname.
    #[serde(default = "default_redis_hostname")]
    pub hostname: String,
    /// TCP port (`0` unused in tests; the fixture supplies the mapped port).
    #[serde(default = "default_redis_port")]
    pub port: u16,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            hostname: default_redis_hostname(),
            port: default_redis_port(),
        }
    }
}

fn default_redis_hostname() -> String {
    "127.0.0.1".to_owned()
}

const fn default_redis_port() -> u16 {
    6379
}

impl RedisConfig {
    /// `redis://hostname:port`
    #[must_use]
    pub fn url(&self) -> String {
        format!("redis://{}:{}", self.hostname, self.port)
    }
}

/// OpenBao/Vault HTTP KV API.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultConfig {
    /// Base URL (`http://127.0.0.1:8200`).
    #[serde(default)]
    pub base_url: String,
    /// Token header. Empty fails closed.
    #[serde(default)]
    pub token: String,
    /// KV mount (`secret`).
    #[serde(default = "default_vault_mount")]
    pub mount: String,
}

fn default_vault_mount() -> String {
    "secret".to_owned()
}

/// Operator-admitted named connector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorEntry {
    /// Plugin-visible name.
    pub name: String,
    /// Admitted base URL.
    pub url: String,
}

/// Main application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// HTTP server bind.
    pub server: ServerConfig,
    /// JWT consumer (`hs256_secret`, issuer, audience) for rustycog `UserIdExtractor` only.
    #[serde(default)]
    pub auth: AuthConfig,
    /// Workload identity (dedicated session key, TTLs). Distinct from `[auth.jwt]`.
    #[serde(default)]
    pub identity: IdentityConfig,
    /// Manifesto live consult for binding grant snapshots.
    #[serde(default)]
    pub manifesto_service: ManifestoServiceConfig,
    /// Database configuration.
    pub database: DatabaseConfig,
    /// Logging configuration.
    pub logging: LoggingConfig,
    /// Command retry configuration.
    #[serde(default)]
    pub command: CommandConfig,
    /// Queue configuration (disabled this slice).
    pub queue: QueueConfig,
    /// Active KV adapter.
    #[serde(default)]
    pub kv: KvConfig,
    /// Redis (when `kv.backend = redis`).
    #[serde(default)]
    pub redis: RedisConfig,
    /// Vault/OpenBao HTTP.
    #[serde(default)]
    pub vault: VaultConfig,
    /// Operator-admitted named connectors.
    #[serde(default)]
    pub connectors: Vec<ConnectorEntry>,
    /// Optional isolated-plugin HTTP hop. Empty `endpoint_url` → in-process dispatch.
    #[serde(default)]
    pub plugin_hop: PluginHopConfig,
}

/// Locator config for the Lazaret → plugin HTTP hop (M5).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginHopConfig {
    /// Plugin base URL (`http://host:port`). Empty disables the hop.
    #[serde(default)]
    pub endpoint_url: String,
}

impl ConfigLoader<Self> for AppConfig {
    fn create_default() -> Self {
        Self::default()
    }

    fn config_prefix() -> &'static str {
        "LAZARET"
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

/// Load configuration from environment and config files.
///
/// # Errors
///
/// Returns [`ConfigError`] if the configuration cannot be loaded.
pub fn load_config() -> Result<AppConfig, ConfigError> {
    load_config_fresh::<AppConfig>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_config_is_not_iam_jwt() {
        let identity = IdentityConfig::default();
        assert_eq!(identity.session_ttl_minutes, DEFAULT_SESSION_TTL_MINUTES);
        assert_eq!(identity.cert_ttl_hours, DEFAULT_CERT_TTL_HOURS);
        assert!(identity.session_signing_key_pem.is_none());
        assert!(identity.ca_cert_pem_path.is_empty());
        assert!(identity.ca_key_pem_path.is_empty());
        let app = AppConfig::default();
        assert!(app.identity.session_signing_key_pem.is_none());
        assert!(app.identity.ca_cert_pem_path.is_empty());
        assert!(app.identity.ca_key_pem_path.is_empty());
        assert_ne!(
            format!("{:?}", app.identity),
            format!("{:?}", app.auth),
            "[identity] must stay distinct from [auth.jwt]"
        );
    }
}
