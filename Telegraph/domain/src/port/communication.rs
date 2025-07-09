//! Communication port interfaces for Telegraph service

use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::DomainError;
use crate::entity::{CommunicationMessage, MessageDelivery, CommunicationMode};

/// Port for sending communications via different channels
#[async_trait]
pub trait CommunicationService: Send + Sync {
    /// Send a communication message
    async fn send_message(&self, message: &CommunicationMessage) -> Result<MessageDelivery, DomainError>;
    
    /// Check if a communication mode is supported
    fn supports_mode(&self, mode: &CommunicationMode) -> bool;
    
    /// Get service health status
    async fn health_check(&self) -> Result<(), DomainError>;
}

/// Port for email communication
#[async_trait]
pub trait EmailService: Send + Sync {
    /// Send an email message
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        text_body: &str,
        html_body: Option<&str>,
        attachments: &[EmailAttachment],
    ) -> Result<String, DomainError>; // Returns provider message ID
    
    /// Verify email address format
    fn validate_email(&self, email: &str) -> Result<(), DomainError>;
    
    /// Check service health
    async fn health_check(&self) -> Result<(), DomainError>;
}

/// Port for push notification service
#[async_trait]
pub trait NotificationService: Send + Sync {
    /// Send a push notification
    async fn send_notification(
        &self,
        device_token: Option<&str>,
        user_id: Option<&str>,
        title: &str,
        body: &str,
        data: &HashMap<String, String>,
    ) -> Result<String, DomainError>; // Returns provider message ID
    
    /// Validate device token format
    fn validate_device_token(&self, token: &str) -> Result<(), DomainError>;
    
    /// Check service health
    async fn health_check(&self) -> Result<(), DomainError>;
}

/// Port for SMS service
#[async_trait]
pub trait SmsService: Send + Sync {
    /// Send an SMS message
    async fn send_sms(
        &self,
        to: &str,
        message: &str,
    ) -> Result<String, DomainError>; // Returns provider message ID
    
    /// Validate phone number format
    fn validate_phone_number(&self, phone: &str) -> Result<(), DomainError>;
    
    /// Check service health
    async fn health_check(&self) -> Result<(), DomainError>;
}

/// Email attachment for sending
#[derive(Debug, Clone)]
pub struct EmailAttachment {
    /// Attachment filename
    pub filename: String,
    /// MIME content type
    pub content_type: String,
    /// Attachment data
    pub data: Vec<u8>,
}

/// Communication provider response
#[derive(Debug, Clone)]
pub struct ProviderResponse {
    /// Provider message ID
    pub message_id: String,
    /// Response status
    pub status: String,
    /// Additional metadata from provider
    pub metadata: HashMap<String, String>,
} 