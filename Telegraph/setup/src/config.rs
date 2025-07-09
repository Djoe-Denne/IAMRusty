//! Configuration setup for Telegraph

use serde::{Deserialize, Serialize};
use rustycog_config::QueueConfig as RustycogQueueConfig;

/// Telegraph application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegraphConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Communication providers configuration
    pub communication: CommunicationConfig,
    /// Event configuration  
    pub events: EventConfig,
    /// Queue configuration for rustycog-events
    pub queue: RustycogQueueConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
}

/// Communication providers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationConfig {
    /// Email provider settings
    pub email: EmailConfig,
    /// SMS provider settings
    pub sms: SmsConfig,
    /// Push notification settings
    pub notifications: NotificationConfig,
}

/// Email configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// Email provider type (ses, smtp)
    pub provider: String,
    /// From email address
    pub from_address: String,
}

/// SMS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConfig {
    /// SMS provider type (sns, twilio)
    pub provider: String,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Notification provider type (fcm, apns)
    pub provider: String,
}

/// Event configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventConfig {
    /// Event queue configuration
    pub queues: Vec<QueueConfig>,
}

/// Queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Queue name
    pub name: String,
    /// Queue URL
    pub url: String,
    /// Queue type (sqs, kafka)
    pub queue_type: String,
}

impl Default for TelegraphConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            communication: CommunicationConfig::default(),
            events: EventConfig::default(),
            queue: RustycogQueueConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
        }
    }
}

impl Default for CommunicationConfig {
    fn default() -> Self {
        Self {
            email: EmailConfig::default(),
            sms: SmsConfig::default(),
            notifications: NotificationConfig::default(),
        }
    }
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            provider: "dummy".to_string(),
            from_address: "noreply@telegraph.service".to_string(),
        }
    }
}

impl Default for SmsConfig {
    fn default() -> Self {
        Self {
            provider: "dummy".to_string(),
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            provider: "dummy".to_string(),
        }
    }
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            queues: vec![],
        }
    }
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            url: "localhost:9092".to_string(),
            queue_type: "kafka".to_string(),
        }
    }
} 