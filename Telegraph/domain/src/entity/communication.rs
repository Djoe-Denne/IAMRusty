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
    pub fn to_string(&self) -> String {
        match self {
            CommunicationMode::Email => "email".to_string(),
            CommunicationMode::Notification => "notification".to_string(),
        }
    }
}

impl Communication {
    /// Get the recipient from any communication type
    pub fn recipient(&self) -> &CommunicationRecipient {
        match self {
            Communication::Email(email) => &email.recipient,
            Communication::Notification(notification) => &notification.recipient,
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
        write!(f, "{}", self.to_string())
    }
}