//! Telegraph Communication Service Configuration
//! 
//! This crate provides configuration structures for the Telegraph communication service,
//! including queue configuration, event routing, and communication modes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use indexmap::IndexMap;

use rustycog_config::{
    ConfigLoader, HasServerConfig, HasLoggingConfig, HasQueueConfig, HasDbConfig, DatabaseConfig,
    LoggingConfig, QueueConfig, ConfigError, load_config_fresh
};

pub use rustycog_config::{ServerConfig, setup_logging};

/// Main Telegraph service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegraphConfig {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,
    
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    
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

/// Mailjet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailjetConfig {
    /// Mailjet API public key 
    #[serde(default)]
    pub public_key: String,
    
    /// Mailjet API private key (MJ_APIKEY_PRIVATE)
    #[serde(default)]
    pub private_key: String,
    
    /// Mailjet API version (v3 or v3.1)
    #[serde(default = "default_mailjet_version")]
    pub version: String,
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
    
    /// Firebase Cloud Messaging configuration
    #[serde(default)]
    pub fcm: FcmConfig,
    
    /// Apple Push Notification Service configuration
    #[serde(default)]
    pub apns: ApnsConfig,
}

/// Firebase Cloud Messaging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmConfig {
    /// FCM project ID
    #[serde(default)]
    pub project_id: String,
    
    /// FCM service account key path
    #[serde(default)]
    pub service_account_key_path: Option<String>,
}

/// Apple Push Notification Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApnsConfig {
    /// APNS key ID
    #[serde(default)]
    pub key_id: String,
    
    /// APNS team ID
    #[serde(default)]
    pub team_id: String,
    
    /// APNS private key path
    #[serde(default)]
    pub private_key_path: Option<String>,
    
    /// Whether to use sandbox environment
    #[serde(default)]
    pub sandbox: bool,
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
fn default_true() -> bool { true }
fn default_email_provider() -> String { "dummy".to_string() }
fn default_from_email() -> String { "noreply@telegraph.com".to_string() }
fn default_from_name() -> String { "Telegraph Service".to_string() }
fn default_smtp_host() -> String { "localhost".to_string() }
fn default_smtp_port() -> u16 { 587 }
fn default_notification_provider() -> String { "dummy".to_string() }
fn default_sms_provider() -> String { "dummy".to_string() }
fn default_mailjet_version() -> String { "v3".to_string() }
fn default_template_dir() -> String { "templates".to_string() }
fn default_html_extension() -> String { "html".to_string() }
fn default_text_extension() -> String { "txt".to_string() }

// Default implementations
impl Default for TelegraphConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
            queue: QueueConfig::default(),
            queues: IndexMap::new(),
            communication: CommunicationConfig::default(),
            database: DatabaseConfig::default(),
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

impl Default for MailjetConfig {
    fn default() -> Self {
        Self {
            public_key: String::new(),
            private_key: String::new(),
            version: default_mailjet_version(),
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            provider: default_notification_provider(),
            fcm: FcmConfig::default(),
            apns: ApnsConfig::default(),
        }
    }
}

impl Default for FcmConfig {
    fn default() -> Self {
        Self {
            project_id: String::new(),
            service_account_key_path: None,
        }
    }
}

impl Default for ApnsConfig {
    fn default() -> Self {
        Self {
            key_id: String::new(),
            team_id: String::new(),
            private_key_path: None,
            sandbox: true,
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

impl TelegraphConfig {
    /// Get the configuration for a specific queue
    pub fn get_queue_config(&self, queue_name: &str) -> Option<&QueueEventConfig> {
        self.queues.get(queue_name)
    }
    
    /// Get the event configuration for a specific event in a queue
    pub fn get_event_config(&self, queue_name: &str, event_name: &str) -> Option<&EventConfig> {
        self.get_queue_config(queue_name)?
            .event_configs.get(event_name)
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
