//! Communication HTTP handlers

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

use telegraph_domain::{CommunicationMode};

/// Request to send a message via HTTP API
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    /// Recipient information
    pub recipient: ApiRecipient,
    /// Message content
    pub content: ApiMessageContent,
    /// Communication mode
    pub mode: CommunicationMode,
    /// Message metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// API recipient information
#[derive(Debug, Deserialize)]
pub struct ApiRecipient {
    /// User ID (if known)
    pub user_id: Option<Uuid>,
    /// Email address
    pub email: Option<String>,
    /// Phone number
    pub phone: Option<String>,
    /// Device token
    pub device_token: Option<String>,
    /// Display name
    pub display_name: Option<String>,
}

/// API message content
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ApiMessageContent {
    /// Email message
    Email {
        subject: String,
        html_body: Option<String>,
        text_body: String,
    },
    /// Push notification
    Notification {
        title: String,
        body: String,
        #[serde(default)]
        data: HashMap<String, String>,
        icon: Option<String>,
        click_action: Option<String>,
    },
    /// SMS message
    Sms {
        text: String,
    },
}

/// Response for successful message sending
#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    /// Message ID
    pub message_id: Uuid,
    /// Delivery ID
    pub delivery_id: Uuid,
    /// Delivery status
    pub status: String,
} 