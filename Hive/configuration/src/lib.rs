use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use rustycog_config::{DatabaseConfig, LoggingConfig, QueueConfig, ServerConfig};

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to load configuration: {message}")]
    LoadError { message: String },

    #[error("Invalid configuration: {message}")]
    ValidationError { message: String },

    #[error("Missing required configuration: {key}")]
    MissingRequired { key: String },
}

/// Main service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service name and metadata
    pub service: ServiceInfo,

    /// HTTP server configuration
    pub server: ServerConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Queue configuration for events
    pub queue: QueueConfig,
}

/// Service metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub version: String,
    pub environment: String,
}

/// IAM service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IamServiceConfig {
    pub base_url: String,
    pub api_key: String,
    pub timeout_seconds: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            service: ServiceInfo {
                name: "hive".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                environment: "development".to_string(),
            },
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            logging: LoggingConfig::default(),
            queue: QueueConfig::default(),
        }
    }
}

impl ServiceConfig {
    /// Load configuration from environment and config files
    pub fn load() -> Result<Self, ConfigError> {
        // TODO: Implement actual configuration loading from:
        // 1. Default values
        // 2. Config files (TOML/YAML)
        // 3. Environment variables
        // 4. Command line arguments

        let config = Self::default();
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate server configuration
        if self.server.port == 0 {
            return Err(ConfigError::ValidationError {
                message: "Server port must be greater than 0".to_string(),
            });
        }

        // Validate database configuration
        if self.database.url().is_empty() {
            return Err(ConfigError::MissingRequired {
                key: "database.url".to_string(),
            });
        }

        Ok(())
    }

    /// Get database URL
    pub fn database_url(&self) -> String {
        self.database.url().clone()
    }

    /// Get server bind address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    pub fn queue_config(&self) -> &QueueConfig {
        &self.queue
    }
}
