//! Typed configuration for the sentinel-sync worker.
//!
//! The worker reuses `rustycog-config`'s shared building blocks (logging,
//! queue) and adds its own `OpenFgaConfig` section.

use rustycog_config::{LoggingConfig, QueueConfig};
use serde::{Deserialize, Serialize};

/// Top-level configuration for the sentinel-sync worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelSyncConfig {
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Queue (Kafka/SQS/Disabled) the worker consumes from.
    pub queue: QueueConfig,
    /// OpenFGA server the worker writes tuples into.
    pub openfga: OpenFgaConfig,
    /// Idempotency ledger configuration.
    #[serde(default)]
    pub idempotency: IdempotencyConfig,
}

/// OpenFGA connection settings shared with `rustycog-permission`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFgaConfig {
    pub api_url: String,
    pub store_id: String,
    #[serde(default)]
    pub authorization_model_id: Option<String>,
    #[serde(default)]
    pub api_token: Option<String>,
}

/// Idempotency-ledger settings. The ledger records processed `event_id`s so
/// retries and replays never re-apply the same write.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyConfig {
    /// Backing store kind. Only `in-memory` is implemented today; `postgres`
    /// is the production target and will be added in a follow-up.
    #[serde(default = "default_backend")]
    pub backend: String,
    /// Postgres connection string when `backend = "postgres"`.
    #[serde(default)]
    pub database_url: Option<String>,
}

impl Default for IdempotencyConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            database_url: None,
        }
    }
}

fn default_backend() -> String {
    "in-memory".to_string()
}

impl SentinelSyncConfig {
    /// Load config from `config/sentinel-sync.toml` and `SENTINEL_SYNC__*`
    /// env vars. Mirrors the convention used by every other RustyCog service.
    pub fn load() -> Result<Self, rustycog_config::ConfigError> {
        use rustycog_config::{Config, Environment, File, FileFormat};

        let _ = rustycog_config::dotenv();

        let builder = Config::builder()
            .add_source(
                File::with_name("config/sentinel-sync")
                    .format(FileFormat::Toml)
                    .required(false),
            )
            .add_source(
                Environment::with_prefix("SENTINEL_SYNC")
                    .separator("__")
                    .try_parsing(true),
            );

        builder.build()?.try_deserialize::<SentinelSyncConfig>()
    }
}
