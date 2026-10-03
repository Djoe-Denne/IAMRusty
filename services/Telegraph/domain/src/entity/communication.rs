//! Communication entity structures for different communication modes

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Recipient information for communications
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunicationRecipient {
    pub user_id: Option<uuid::Uuid>,
    pub email: Option<String>,
}

/// Email communication structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailCommunication {
    pub recipient: CommunicationRecipient,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
}

/// Notification communication structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationCommunication {
    pub recipient: CommunicationRecipient,
    pub id: Option<Uuid>,
    pub title: String,
    pub body: String,
    pub data: HashMap<String, String>,
    pub is_read: Option<bool>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Communication enum for different modes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Communication {
    Email(EmailCommunication),
    Notification(NotificationCommunication),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommunicationMode {
    Email,
    Notification,
}

impl CommunicationMode {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Notification => "notification",
        }
    }
}

impl Communication {
    /// Get the recipient from any communication type
    #[must_use]
    pub const fn recipient(&self) -> &CommunicationRecipient {
        match self {
            Self::Email(email) => &email.recipient,
            Self::Notification(notification) => &notification.recipient,
        }
    }
}

/// Communication descriptor loaded from TOML files
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunicationDescriptor {
    pub email: Option<EmailDescriptor>,
    pub notification: Option<NotificationDescriptor>,
}

/// Email configuration from TOML descriptor
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailDescriptor {
    pub subject: String,
    pub template: String,
    pub mode: Option<String>, // "html" or "text"
}

/// Notification configuration from TOML descriptor
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationDescriptor {
    pub title: String,
    pub template: String,
}

impl std::fmt::Display for CommunicationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
