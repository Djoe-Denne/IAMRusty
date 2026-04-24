//! Telegraph Communication Service Configuration
//!
//! This crate provides configuration structures for the Telegraph communication service,
//! including queue configuration, event routing, and communication modes.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use rustycog_config::{
    load_config_fresh, AuthConfig, ConfigError, ConfigLoader, DatabaseConfig, HasDbConfig,
    HasLoggingConfig, HasOpenFgaConfig, HasQueueConfig, HasScalewayConfig, HasServerConfig,
    LoggingConfig, OpenFgaClientConfig, QueueConfig, ScalewayConfig,
};

pub use rustycog_config::{ServerConfig};

pub use rustycog_logger::{setup_logging};

/// Main Telegraph service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegraphConfig {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Shared authentication verifier configuration
    #[serde(default)]
    pub auth: AuthConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Scaleway configuration
    #[serde(default)]
    pub scaleway: ScalewayConfig,

    /// Queue configuration (SQS, Kafka, etc.)
    #[serde(default)]
    pub queue: QueueConfig,

    /// Telegraph-specific queue configurations
    #[serde(default)]
    pub queues: IndexMap<String, QueueEventConfig>,

    /// Communication configuration
    #[serde(default)]
    pub communication: CommunicationConfig,

    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,

    /// OpenFGA authorization checker configuration.
    #[serde(default)]
    pub openfga: OpenFgaClientConfig,
}

/// Configuration for a specific queue and its events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEventConfig {
    /// Events that this queue processes
    pub events: Vec<String>,

    /// Event-specific configurations
    #[serde(flatten)]
    pub event_configs: HashMap<String, EventConfig>,
}

/// Configuration for how to handle a specific event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventConfig {
    /// Communication modes to use for this event (notification, email, sms, etc.)
    pub modes: Vec<String>,

    /// Template name prefix for this event (e.g., "user_signed_up")
    #[serde(default)]
    pub template: Option<String>,

    /// Additional event-specific settings
    #[serde(default)]
    pub settings: HashMap<String, String>,
}

/// Communication service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationConfig {
    /// Email service configuration
    #[serde(default)]
    pub email: EmailConfig,

    /// Push notification service configuration
    #[serde(default)]
    pub notification: NotificationConfig,

    /// Template service configuration
    #[serde(default)]
    pub template: TemplateConfig,
}

/// Email service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// Whether email service is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Email service provider (dummy, smtp, ses, mailjet, etc.)
    #[serde(default = "default_email_provider")]
    pub provider: String,

    /// SMTP configuration (if using SMTP provider)
    #[serde(default)]
    pub smtp: SmtpConfig,

    /// Default from address
    #[serde(default = "default_from_email")]
    pub from_address: String,

    /// Default from name
    #[serde(default = "default_from_name")]
    pub from_name: String,
}

/// SMTP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    /// SMTP server host
    #[serde(default = "default_smtp_host")]
    pub host: String,

    /// SMTP server port
    #[serde(default = "default_smtp_port")]
    pub port: u16,

    /// Whether to use TLS
    #[serde(default)]
    pub use_tls: bool,

    /// SMTP username
    #[serde(default)]
    pub username: Option<String>,

    /// SMTP password
    #[serde(default)]
    pub password: Option<String>,
}

/// Push notification service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Whether push notification service is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Notification service provider (dummy, fcm, apns, etc.)
    #[serde(default = "default_notification_provider")]
    pub provider: String,

}

/// Template service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Whether template service is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Template directory path
    #[serde(default = "default_template_dir")]
    pub template_dir: String,

    /// Template file extensions for different formats
    #[serde(default)]
    pub extensions: TemplateExtensions,
}

/// Template file extensions configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateExtensions {
    /// Extension for HTML email templates
    #[serde(default = "default_html_extension")]
    pub html: String,

    /// Extension for plain text email templates
    #[serde(default = "default_text_extension")]
    pub text: String,
}

// Default value functions
fn default_true() -> bool {
    true
}
fn default_email_provider() -> String {
    "dummy".to_string()
}
fn default_from_email() -> String {
    "noreply@telegraph.com".to_string()
}
fn default_from_name() -> String {
    "Telegraph Service".to_string()
}
fn default_smtp_host() -> String {
    "localhost".to_string()
}
fn default_smtp_port() -> u16 {
    587
}
fn default_notification_provider() -> String {
    "dummy".to_string()
}
fn default_sms_provider() -> String {
    "dummy".to_string()
}
fn default_mailjet_version() -> String {
    "v3".to_string()
}
fn default_template_dir() -> String {
    "templates".to_string()
}
fn default_html_extension() -> String {
    "html".to_string()
}
fn default_text_extension() -> String {
    "txt".to_string()
}

// Default implementations
impl Default for TelegraphConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
            logging: LoggingConfig::default(),
            scaleway: ScalewayConfig::default(),
            queue: QueueConfig::default(),
            queues: IndexMap::new(),
            communication: CommunicationConfig::default(),
            database: DatabaseConfig::default(),
            openfga: OpenFgaClientConfig::default(),
        }
    }
}

impl Default for QueueEventConfig {
    fn default() -> Self {
        Self {
            events: vec![],
            event_configs: HashMap::new(),
        }
    }
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            modes: vec!["notification".to_string()],
            template: None,
            settings: HashMap::new(),
        }
    }
}

impl Default for CommunicationConfig {
    fn default() -> Self {
        Self {
            email: EmailConfig::default(),
            notification: NotificationConfig::default(),
            template: TemplateConfig::default(),
        }
    }
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            provider: default_email_provider(),
            smtp: SmtpConfig::default(),
            from_address: default_from_email(),
            from_name: default_from_name(),
        }
    }
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: default_smtp_host(),
            port: default_smtp_port(),
            use_tls: false,
            username: None,
            password: None,
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            provider: default_notification_provider(),
        }
    }
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            template_dir: default_template_dir(),
            extensions: TemplateExtensions::default(),
        }
    }
}

impl Default for TemplateExtensions {
    fn default() -> Self {
        Self {
            html: default_html_extension(),
            text: default_text_extension(),
        }
    }
}

// Trait implementations for rustycog-config integration
impl ConfigLoader<TelegraphConfig> for TelegraphConfig {
    fn create_default() -> TelegraphConfig {
        TelegraphConfig::default()
    }

    fn config_prefix() -> &'static str {
        "TELEGRAPH"
    }
}

impl HasServerConfig for TelegraphConfig {
    fn server_config(&self) -> &ServerConfig {
        &self.server
    }

    fn set_server_config(&mut self, config: ServerConfig) {
        self.server = config;
    }
}

impl HasLoggingConfig for TelegraphConfig {
    fn logging_config(&self) -> &LoggingConfig {
        &self.logging
    }

    fn set_logging_config(&mut self, config: LoggingConfig) {
        self.logging = config;
    }
}

impl HasScalewayConfig for TelegraphConfig {
    fn scaleway_config(&self) -> &ScalewayConfig {
        &self.scaleway
    }

    fn set_scaleway_config(&mut self, config: ScalewayConfig) {
        self.scaleway = config;
    }
}

impl HasQueueConfig for TelegraphConfig {
    fn queue_config(&self) -> &QueueConfig {
        &self.queue
    }

    fn set_queue_config(&mut self, config: QueueConfig) {
        self.queue = config;
    }
}

impl HasDbConfig for TelegraphConfig {
    fn db_config(&self) -> &DatabaseConfig {
        &self.database
    }

    fn set_db_config(&mut self, config: DatabaseConfig) {
        self.database = config;
    }
}

impl HasOpenFgaConfig for TelegraphConfig {
    fn openfga_config(&self) -> &OpenFgaClientConfig {
        &self.openfga
    }

    fn set_openfga_config(&mut self, config: OpenFgaClientConfig) {
        self.openfga = config;
    }
}

impl TelegraphConfig {
    /// Get the configuration for a specific queue
    pub fn get_queue_config(&self, queue_name: &str) -> Option<&QueueEventConfig> {
        self.queues.get(queue_name)
    }

    /// Get the event configuration for a specific event in a queue
    pub fn get_event_config(&self, queue_name: &str, event_name: &str) -> Option<&EventConfig> {
        self.get_queue_config(queue_name)?
            .event_configs
            .get(event_name)
    }

    /// Check if a queue should process a specific event
    pub fn queue_handles_event(&self, queue_name: &str, event_name: &str) -> bool {
        self.get_queue_config(queue_name)
            .map(|config| config.events.contains(&event_name.to_string()))
            .unwrap_or(false)
    }

    /// Get all queues that handle a specific event
    pub fn queues_for_event(&self, event_name: &str) -> Vec<&str> {
        self.queues
            .iter()
            .filter(|(_, config)| config.events.contains(&event_name.to_string()))
            .map(|(name, _)| name.as_str())
            .collect()
    }
}

/// Load configuration from environment and config files
/// This function caches the configuration to ensure consistent behavior,
/// especially for random port generation in database configuration.
pub fn load_config() -> Result<TelegraphConfig, ConfigError> {
    load_config_fresh::<TelegraphConfig>()
}
