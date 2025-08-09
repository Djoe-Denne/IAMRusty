use serde::{Deserialize, Serialize};


pub use rustycog_config::{CommandConfig, DatabaseConfig, LoggingConfig, QueueConfig, ServerConfig, setup_logging};

/// IAM service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IamServiceConfig {
    pub base_url: String,
    pub api_key: String,
    pub timeout_seconds: u64,
}

/// External Provider service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalProviderServiceConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}


/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// IAM service configuration
    pub iam_service: IamServiceConfig,
    /// External Provider service configuration
    pub external_provider_service: ExternalProviderServiceConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Command configuration
    pub command: CommandConfig,
    /// Queue configuration (Kafka, SQS, or Disabled)
    pub queue: QueueConfig,
}