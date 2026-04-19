use rustycog_config::{AuthConfig, HasQueueConfig, HasDbConfig, HasLoggingConfig, HasServerConfig, HasScalewayConfig};
use serde::{Deserialize, Serialize};


pub use rustycog_config::{AuthConfig as SharedAuthConfig, CommandConfig, DatabaseConfig, LoggingConfig, QueueConfig, ServerConfig, load_config_fresh, ConfigError, ConfigLoader, ScalewayConfig};

pub use rustycog_logger::{setup_logging};

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
    /// Shared authentication verifier configuration
    #[serde(default)]
    pub auth: AuthConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// IAM service configuration
    pub iam_service: IamServiceConfig,
    /// External Provider service configuration
    pub external_provider_service: ExternalProviderServiceConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Scaleway configuration
    pub scaleway: ScalewayConfig,
    /// Command configuration
    pub command: CommandConfig,
    /// Queue configuration (Kafka, SQS, or Disabled)
    pub queue: QueueConfig,
}

/***********************************
 *  Default implementation
 *********************************/

impl Default for IamServiceConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            api_key: "".to_string(),
            timeout_seconds: 10,
        }
    }
}

impl Default for ExternalProviderServiceConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            api_key: None,
            timeout_seconds: 10,
            max_retries: 3,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
            database: DatabaseConfig::default(),
            iam_service: IamServiceConfig::default(),
            external_provider_service: ExternalProviderServiceConfig::default(),
            logging: LoggingConfig::default(),
            scaleway: ScalewayConfig::default(),
            command: CommandConfig::default(),
            queue: QueueConfig::default(),
        }
    }
}

/***********************************
 *  ConfigLoader implementation
 *********************************/

impl ConfigLoader<AppConfig> for AppConfig {
    fn create_default() -> AppConfig {
        AppConfig::default()
    }

    fn config_prefix() -> &'static str {
        "HIVE"
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

impl HasScalewayConfig for AppConfig {
    fn scaleway_config(&self) -> &ScalewayConfig {
        &self.scaleway
    }

    fn set_scaleway_config(&mut self, config: ScalewayConfig) {
        self.scaleway = config;
    }
}

/// Load configuration from environment and config files
/// This function caches the configuration to ensure consistent behavior,
/// especially for random port generation in database configuration.
pub fn load_config() -> Result<AppConfig, ConfigError> {
    load_config_fresh::<AppConfig>()
}
