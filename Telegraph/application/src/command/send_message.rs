//! Send message command for Telegraph application

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use telegraph_domain::{CommunicationMode, MessagePriority};

/// Command to send a communication message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageCommand {
    /// Recipient information
    pub recipient: SendMessageRecipient,
    /// Message content
    pub content: SendMessageContent,
    /// Communication mode
    pub mode: CommunicationMode,
    /// Message priority
    #[serde(default)]
    pub priority: MessagePriority,
    /// Message metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Recipient information for send message command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRecipient {
    /// User ID (if known)
    pub user_id: Option<Uuid>,
    /// Email address (for email messages)
    pub email: Option<String>,
    /// Phone number (for SMS messages)
    pub phone: Option<String>,
    /// Device token (for push notifications)
    pub device_token: Option<String>,
    /// Display name for the recipient
    pub display_name: Option<String>,
}

/// Message content for send message command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SendMessageContent {
    /// Email message content
    Email {
        /// Email subject
        subject: String,
        /// HTML content
        html_body: Option<String>,
        /// Plain text content
        text_body: String,
    },
    /// Push notification content
    Notification {
        /// Notification title
        title: String,
        /// Notification body
        body: String,
        /// Notification data payload
        #[serde(default)]
        data: HashMap<String, String>,
        /// Notification icon
        icon: Option<String>,
        /// Click action
        click_action: Option<String>,
    },
    /// SMS message content
    Sms {
        /// SMS message text
        text: String,
    },
}

impl SendMessageCommand {
    /// Create a new send message command
    pub fn new(
        recipient: SendMessageRecipient,
        content: SendMessageContent,
        mode: CommunicationMode,
    ) -> Self {
        Self {
            recipient,
            content,
            mode,
            priority: MessagePriority::Normal,
            metadata: HashMap::new(),
        }
    }
    
    /// Set message priority
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
} 