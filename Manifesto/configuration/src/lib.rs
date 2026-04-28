//! Manifesto Service Configuration
//!
//! This crate provides configuration structures for the Manifesto service,
//! including service-specific configurations and integration with rustycog-config.

use serde::{Deserialize, Serialize};

use rustycog_config::{
    load_config_fresh, AuthConfig, CommandConfig, ConfigError, ConfigLoader, DatabaseConfig,
    HasDbConfig, HasLoggingConfig, HasOpenFgaConfig, HasQueueConfig, HasScalewayConfig,
    HasServerConfig, LoggingConfig, OpenFgaClientConfig, QueueConfig, ScalewayConfig, ServerConfig,
};

pub use rustycog_logger::setup_logging;

/// Type alias for backward compatibility
pub type AppConfig = ManifestoConfig;

/// Main Manifesto service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestoConfig {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Shared authentication verifier configuration
    #[serde(default)]
    pub auth: AuthConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Command execution configuration
    #[serde(default)]
    pub command: CommandConfig,

    /// Queue configuration (SQS, Kafka, etc.)
    #[serde(default)]
    pub queue: QueueConfig,

    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Scaleway configuration
    #[serde(default)]
    pub scaleway: ScalewayConfig,

    /// Service-specific configuration
    #[serde(default)]
    pub service: ServiceConfig,

    /// OpenFGA authorization checker configuration.
    #[serde(default)]
    pub openfga: OpenFgaClientConfig,
}

/// Component service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentServiceConfig {
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

fn default_timeout_seconds() -> u64 {
    10
}

impl Default for ComponentServiceConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:9000".to_string(),
            api_key: None,
            timeout_seconds: default_timeout_seconds(),
        }
    }
}

/// Service-specific configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Component service configuration
    #[serde(default)]
    pub component_service: ComponentServiceConfig,

    /// Business logic configuration
    #[serde(default)]
    pub business: BusinessConfig,
}

/// Business logic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessConfig {
    /// Maximum projects per user
    #[serde(default = "default_max_projects_per_user")]
    pub max_projects_per_user: u32,

    /// Maximum projects per organization
    #[serde(default = "default_max_projects_per_org")]
    pub max_projects_per_org: u32,

    /// Maximum members per project
    #[serde(default = "default_max_members_per_project")]
    pub max_members_per_project: u32,

    /// Maximum components per project
    #[serde(default = "default_max_components_per_project")]
    pub max_components_per_project: u32,

    /// Default page size for pagination
    #[serde(default = "default_page_size")]
    pub default_page_size: u32,

    /// Maximum page size for pagination
    #[serde(default = "default_max_page_size")]
    pub max_page_size: u32,

    /// Project name maximum length
    #[serde(default = "default_project_name_max_length")]
    pub project_name_max_length: usize,

    /// Project description maximum length
    #[serde(default = "default_project_description_max_length")]
    pub project_description_max_length: usize,

    /// Grace period for member removal (in days)
    #[serde(default = "default_member_removal_grace_period_days")]
    pub member_removal_grace_period_days: u32,
}

// Default value functions
fn default_true() -> bool {
    true
}
fn default_email_provider() -> String {
    "dummy".to_string()
}
fn default_from_email() -> String {
    "noreply@manifesto.com".to_string()
}
fn default_from_name() -> String {
    "Manifesto Service".to_string()
}
fn default_notification_provider() -> String {
    "dummy".to_string()
}
fn default_cache_provider() -> String {
    "memory".to_string()
}
fn default_cache_ttl() -> u32 {
    3600
} // 1 hour
fn default_max_projects_per_user() -> u32 {
    100
}
fn default_max_projects_per_org() -> u32 {
    500
}
fn default_max_members_per_project() -> u32 {
    100
}
fn default_max_components_per_project() -> u32 {
    50
}
fn default_page_size() -> u32 {
    20
}
fn default_max_page_size() -> u32 {
    100
}
fn default_project_name_max_length() -> usize {
    255
}
fn default_project_description_max_length() -> usize {
    2000
}
fn default_member_removal_grace_period_days() -> u32 {
    30
}

// Default implementations
impl Default for ManifestoConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
            logging: LoggingConfig::default(),
            command: CommandConfig::default(),
            queue: QueueConfig::default(),
            database: DatabaseConfig::default(),
            scaleway: ScalewayConfig::default(),
            service: ServiceConfig::default(),
            openfga: OpenFgaClientConfig::default(),
        }
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            component_service: ComponentServiceConfig::default(),
            business: BusinessConfig::default(),
        }
    }
}

impl Default for BusinessConfig {
    fn default() -> Self {
        Self {
            max_projects_per_user: default_max_projects_per_user(),
            max_projects_per_org: default_max_projects_per_org(),
            max_members_per_project: default_max_members_per_project(),
            max_components_per_project: default_max_components_per_project(),
            default_page_size: default_page_size(),
            max_page_size: default_max_page_size(),
            project_name_max_length: default_project_name_max_length(),
            project_description_max_length: default_project_description_max_length(),
            member_removal_grace_period_days: default_member_removal_grace_period_days(),
        }
    }
}

// Trait implementations for rustycog-config integration
impl ConfigLoader<ManifestoConfig> for ManifestoConfig {
    fn create_default() -> ManifestoConfig {
        ManifestoConfig::default()
    }

    fn config_prefix() -> &'static str {
        "MANIFESTO"
    }
}

impl HasServerConfig for ManifestoConfig {
    fn server_config(&self) -> &ServerConfig {
        &self.server
    }

    fn set_server_config(&mut self, config: ServerConfig) {
        self.server = config;
    }
}

impl HasLoggingConfig for ManifestoConfig {
    fn logging_config(&self) -> &LoggingConfig {
        &self.logging
    }

    fn set_logging_config(&mut self, config: LoggingConfig) {
        self.logging = config;
    }
}

impl HasScalewayConfig for ManifestoConfig {
    fn scaleway_config(&self) -> &ScalewayConfig {
        &self.scaleway
    }

    fn set_scaleway_config(&mut self, config: ScalewayConfig) {
        self.scaleway = config;
    }
}

impl HasQueueConfig for ManifestoConfig {
    fn queue_config(&self) -> &QueueConfig {
        &self.queue
    }

    fn set_queue_config(&mut self, config: QueueConfig) {
        self.queue = config;
    }
}

impl HasDbConfig for ManifestoConfig {
    fn db_config(&self) -> &DatabaseConfig {
        &self.database
    }

    fn set_db_config(&mut self, config: DatabaseConfig) {
        self.database = config;
    }
}

impl HasOpenFgaConfig for ManifestoConfig {
    fn openfga_config(&self) -> &OpenFgaClientConfig {
        &self.openfga
    }

    fn set_openfga_config(&mut self, config: OpenFgaClientConfig) {
        self.openfga = config;
    }
}

/// Load configuration from environment and config files
pub fn load_config() -> Result<ManifestoConfig, ConfigError> {
    load_config_fresh::<ManifestoConfig>()
}
