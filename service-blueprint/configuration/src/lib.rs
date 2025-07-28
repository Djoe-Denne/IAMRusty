//! {{SERVICE_NAME_PASCAL}} Service Configuration
//! 
//! This crate provides configuration structures for the {{SERVICE_NAME}} service,
//! including service-specific configurations and integration with rustycog-config.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use rustycog_config::{
    ConfigLoader, HasServerConfig, HasLoggingConfig, HasQueueConfig, HasDbConfig, DatabaseConfig,
    LoggingConfig, QueueConfig, ConfigError, load_config_fresh, ServerConfig,
};

pub use rustycog_config::{setup_logging};

/// Main {{SERVICE_NAME}} service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{SERVICE_NAME_PASCAL}}Config {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,
    
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    
    /// Queue configuration (SQS, Kafka, etc.)
    #[serde(default)]
    pub queue: QueueConfig,
    
    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,
    
    /// Service-specific configuration
    #[serde(default)]
    pub service: ServiceConfig,
}

/// Service-specific configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Feature flags
    #[serde(default)]
    pub features: FeatureFlags,
    
    /// External service configurations
    #[serde(default)]
    pub external_services: ExternalServicesConfig,
    
    /// Business logic configuration
    #[serde(default)]
    pub business: BusinessConfig,
}

/// Feature flags for enabling/disabling functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable caching
    #[serde(default = "default_true")]
    pub caching_enabled: bool,
    
    /// Enable audit logging
    #[serde(default = "default_true")]
    pub audit_logging_enabled: bool,
    
    /// Enable event publishing
    #[serde(default = "default_true")]
    pub event_publishing_enabled: bool,
    
    /// Enable metrics collection
    #[serde(default = "default_true")]
    pub metrics_enabled: bool,
}

/// External service configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServicesConfig {
    /// Email service configuration
    #[serde(default)]
    pub email: EmailServiceConfig,
    
    /// Notification service configuration
    #[serde(default)]
    pub notification: NotificationServiceConfig,
    
    /// Cache service configuration
    #[serde(default)]
    pub cache: CacheServiceConfig,
}

/// Email service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailServiceConfig {
    /// Email service provider (dummy, smtp, mailgun, etc.)
    #[serde(default = "default_email_provider")]
    pub provider: String,
    
    /// Default from address
    #[serde(default = "default_from_email")]
    pub from_address: String,
    
    /// Default from name
    #[serde(default = "default_from_name")]
    pub from_name: String,
    
    /// Provider-specific configuration
    #[serde(default)]
    pub provider_config: HashMap<String, String>,
}

/// Notification service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationServiceConfig {
    /// Notification service provider (dummy, fcm, apns, etc.)
    #[serde(default = "default_notification_provider")]
    pub provider: String,
    
    /// Provider-specific configuration
    #[serde(default)]
    pub provider_config: HashMap<String, String>,
}

/// Cache service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheServiceConfig {
    /// Cache service provider (memory, redis, etc.)
    #[serde(default = "default_cache_provider")]
    pub provider: String,
    
    /// Default TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub default_ttl: u32,
    
    /// Provider-specific configuration
    #[serde(default)]
    pub provider_config: HashMap<String, String>,
}

/// Business logic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessConfig {
    /// Maximum entities per user
    #[serde(default = "default_max_entities_per_user")]
    pub max_entities_per_user: u32,
    
    /// Default page size for pagination
    #[serde(default = "default_page_size")]
    pub default_page_size: u32,
    
    /// Maximum page size for pagination
    #[serde(default = "default_max_page_size")]
    pub max_page_size: u32,
    
    /// Entity name maximum length
    #[serde(default = "default_entity_name_max_length")]
    pub entity_name_max_length: usize,
    
    /// Entity description maximum length
    #[serde(default = "default_entity_description_max_length")]
    pub entity_description_max_length: usize,
}

// Default value functions
fn default_true() -> bool { true }
fn default_email_provider() -> String { "dummy".to_string() }
fn default_from_email() -> String { "noreply@{{SERVICE_NAME}}.com".to_string() }
fn default_from_name() -> String { "{{SERVICE_NAME_PASCAL}} Service".to_string() }
fn default_notification_provider() -> String { "dummy".to_string() }
fn default_cache_provider() -> String { "memory".to_string() }
fn default_cache_ttl() -> u32 { 3600 } // 1 hour
fn default_max_entities_per_user() -> u32 { 100 }
fn default_page_size() -> u32 { 20 }
fn default_max_page_size() -> u32 { 100 }
fn default_entity_name_max_length() -> usize { 255 }
fn default_entity_description_max_length() -> usize { 1000 }

// Default implementations
impl Default for {{SERVICE_NAME_PASCAL}}Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
            queue: QueueConfig::default(),
            database: DatabaseConfig::default(),
            service: ServiceConfig::default(),
        }
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            features: FeatureFlags::default(),
            external_services: ExternalServicesConfig::default(),
            business: BusinessConfig::default(),
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            caching_enabled: default_true(),
            audit_logging_enabled: default_true(),
            event_publishing_enabled: default_true(),
            metrics_enabled: default_true(),
        }
    }
}

impl Default for ExternalServicesConfig {
    fn default() -> Self {
        Self {
            email: EmailServiceConfig::default(),
            notification: NotificationServiceConfig::default(),
            cache: CacheServiceConfig::default(),
        }
    }
}

impl Default for EmailServiceConfig {
    fn default() -> Self {
        Self {
            provider: default_email_provider(),
            from_address: default_from_email(),
            from_name: default_from_name(),
            provider_config: HashMap::new(),
        }
    }
}

impl Default for NotificationServiceConfig {
    fn default() -> Self {
        Self {
            provider: default_notification_provider(),
            provider_config: HashMap::new(),
        }
    }
}

impl Default for CacheServiceConfig {
    fn default() -> Self {
        Self {
            provider: default_cache_provider(),
            default_ttl: default_cache_ttl(),
            provider_config: HashMap::new(),
        }
    }
}

impl Default for BusinessConfig {
    fn default() -> Self {
        Self {
            max_entities_per_user: default_max_entities_per_user(),
            default_page_size: default_page_size(),
            max_page_size: default_max_page_size(),
            entity_name_max_length: default_entity_name_max_length(),
            entity_description_max_length: default_entity_description_max_length(),
        }
    }
}

// Trait implementations for rustycog-config integration
impl ConfigLoader<{{SERVICE_NAME_PASCAL}}Config> for {{SERVICE_NAME_PASCAL}}Config {
    fn create_default() -> {{SERVICE_NAME_PASCAL}}Config {
        {{SERVICE_NAME_PASCAL}}Config::default()
    }
    
    fn config_prefix() -> &'static str {
        "{{SERVICE_NAME_UPPER}}"
    }
}

impl HasServerConfig for {{SERVICE_NAME_PASCAL}}Config {
    fn server_config(&self) -> &ServerConfig {
        &self.server
    }
    
    fn set_server_config(&mut self, config: ServerConfig) {
        self.server = config;
    }
}

impl HasLoggingConfig for {{SERVICE_NAME_PASCAL}}Config {
    fn logging_config(&self) -> &LoggingConfig {
        &self.logging
    }
    
    fn set_logging_config(&mut self, config: LoggingConfig) {
        self.logging = config;
    }
}

impl HasQueueConfig for {{SERVICE_NAME_PASCAL}}Config {
    fn queue_config(&self) -> &QueueConfig {
        &self.queue
    }
    
    fn set_queue_config(&mut self, config: QueueConfig) {
        self.queue = config;
    }
}

impl HasDbConfig for {{SERVICE_NAME_PASCAL}}Config {
    fn db_config(&self) -> &DatabaseConfig {
        &self.database
    }
    
    fn set_db_config(&mut self, config: DatabaseConfig) {
        self.database = config;
    }
}

/// Load configuration from environment and config files
pub fn load_config() -> Result<{{SERVICE_NAME_PASCAL}}Config, ConfigError> {
    load_config_fresh::<{{SERVICE_NAME_PASCAL}}Config>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = {{SERVICE_NAME_PASCAL}}Config::default();
        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.service.business.default_page_size, 20);
        assert!(config.service.features.caching_enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = {{SERVICE_NAME_PASCAL}}Config::default();
        let serialized = toml::to_string(&config).unwrap();
        assert!(serialized.contains("[server]"));
        assert!(serialized.contains("[database]"));
        assert!(serialized.contains("[service.features]"));
    }

    #[test]
    fn test_business_config_defaults() {
        let business_config = BusinessConfig::default();
        assert_eq!(business_config.max_entities_per_user, 100);
        assert_eq!(business_config.default_page_size, 20);
        assert_eq!(business_config.max_page_size, 100);
        assert_eq!(business_config.entity_name_max_length, 255);
        assert_eq!(business_config.entity_description_max_length, 1000);
    }
} 