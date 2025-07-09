//! Message domain entities for Telegraph communication service

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::error::DomainError;

/// Communication message that can be sent via different channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationMessage {
    /// Unique message ID
    pub id: Uuid,
    /// Message recipient information
    pub recipient: MessageRecipient,
    /// Message content based on communication mode
    pub content: MessageContent,
    /// Communication mode (email, notification, sms)
    pub mode: CommunicationMode,
    /// Message priority
    pub priority: MessagePriority,
    /// Message metadata
    pub metadata: HashMap<String, String>,
    /// When the message was created
    pub created_at: DateTime<Utc>,
    /// When the message should be sent (for scheduling)
    pub send_at: Option<DateTime<Utc>>,
}

/// Communication modes supported by Telegraph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommunicationMode {
    /// Email notification
    Email,
    /// Push notification
    Notification,
    /// SMS message
    Sms,
}

/// Message priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessagePriority {
    /// Low priority message
    Low,
    /// Normal priority message
    Normal,
    /// High priority message
    High,
    /// Critical/urgent message
    Critical,
}

/// Message recipient information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecipient {
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

/// Message content based on communication mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageContent {
    /// Email message content
    Email {
        /// Email subject
        subject: String,
        /// HTML content
        html_body: Option<String>,
        /// Plain text content
        text_body: String,
        /// Email attachments
        attachments: Vec<EmailAttachment>,
    },
    /// Push notification content
    Notification {
        /// Notification title
        title: String,
        /// Notification body
        body: String,
        /// Notification data payload
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

/// Email attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAttachment {
    /// Attachment filename
    pub filename: String,
    /// MIME content type
    pub content_type: String,
    /// Attachment data (base64 encoded)
    pub data: String,
}

impl CommunicationMessage {
    /// Create a new communication message
    pub fn new(
        recipient: MessageRecipient,
        content: MessageContent,
        mode: CommunicationMode,
    ) -> Result<Self, DomainError> {
        // Validate that the mode matches the content
        Self::validate_mode_content_match(&mode, &content)?;
        
        // Validate that the recipient has the required contact information for the mode
        Self::validate_recipient_for_mode(&recipient, &mode)?;
        
        Ok(Self {
            id: Uuid::new_v4(),
            recipient,
            content,
            mode,
            priority: MessagePriority::Normal,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            send_at: None,
        })
    }
    
    /// Set message priority
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Add metadata to the message
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// Schedule message for later delivery
    pub fn with_send_at(mut self, send_at: DateTime<Utc>) -> Self {
        self.send_at = Some(send_at);
        self
    }
    
    /// Check if message is scheduled for future delivery
    pub fn is_scheduled(&self) -> bool {
        self.send_at.map(|send_at| send_at > Utc::now()).unwrap_or(false)
    }
    
    /// Check if message is ready to be sent
    pub fn is_ready_to_send(&self) -> bool {
        self.send_at.map(|send_at| send_at <= Utc::now()).unwrap_or(true)
    }
    
    /// Validate that the communication mode matches the message content
    fn validate_mode_content_match(mode: &CommunicationMode, content: &MessageContent) -> Result<(), DomainError> {
        match (mode, content) {
            (CommunicationMode::Email, MessageContent::Email { .. }) => Ok(()),
            (CommunicationMode::Notification, MessageContent::Notification { .. }) => Ok(()),
            (CommunicationMode::Sms, MessageContent::Sms { .. }) => Ok(()),
            _ => Err(DomainError::invalid_message(
                format!("Communication mode {:?} does not match message content type", mode)
            )),
        }
    }
    
    /// Validate that the recipient has the required contact information for the mode
    fn validate_recipient_for_mode(recipient: &MessageRecipient, mode: &CommunicationMode) -> Result<(), DomainError> {
        match mode {
            CommunicationMode::Email => {
                if recipient.email.is_none() {
                    return Err(DomainError::invalid_recipient("Email address is required for email messages".to_string()));
                }
                // Basic email validation
                if let Some(email) = &recipient.email {
                    if !email.contains('@') || email.len() < 5 {
                        return Err(DomainError::invalid_email(email.clone()));
                    }
                }
            }
            CommunicationMode::Notification => {
                if recipient.device_token.is_none() && recipient.user_id.is_none() {
                    return Err(DomainError::invalid_recipient("Device token or user ID is required for push notifications".to_string()));
                }
            }
            CommunicationMode::Sms => {
                if recipient.phone.is_none() {
                    return Err(DomainError::invalid_recipient("Phone number is required for SMS messages".to_string()));
                }
                // Basic phone validation
                if let Some(phone) = &recipient.phone {
                    if phone.len() < 10 {
                        return Err(DomainError::invalid_phone_number(phone.clone()));
                    }
                }
            }
        }
        
        Ok(())
    }
}

impl std::fmt::Display for CommunicationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommunicationMode::Email => write!(f, "email"),
            CommunicationMode::Notification => write!(f, "notification"),
            CommunicationMode::Sms => write!(f, "sms"),
        }
    }
}

impl std::str::FromStr for CommunicationMode {
    type Err = DomainError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "email" => Ok(CommunicationMode::Email),
            "notification" => Ok(CommunicationMode::Notification),
            "sms" => Ok(CommunicationMode::Sms),
            _ => Err(DomainError::unsupported_mode(s.to_string())),
        }
    }
}

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
} 