
//! Communication domain service for Telegraph

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, error};
use base64::Engine;

use crate::error::DomainError;
use crate::entity::{CommunicationMessage, MessageDelivery, CommunicationMode, MessageContent};
use crate::port::{CommunicationService, EmailService, NotificationService, SmsService, EmailAttachment};

/// Main communication service that routes messages to appropriate providers
pub struct CommunicationServiceImpl {
    email_service: Arc<dyn EmailService>,
    notification_service: Arc<dyn NotificationService>,
    sms_service: Arc<dyn SmsService>,
}

impl CommunicationServiceImpl {
    /// Create a new communication service
    pub fn new(
        email_service: Arc<dyn EmailService>,
        notification_service: Arc<dyn NotificationService>,
        sms_service: Arc<dyn SmsService>,
    ) -> Self {
        Self {
            email_service,
            notification_service,
            sms_service,
        }
    }
}

#[async_trait]
impl CommunicationService for CommunicationServiceImpl {
    async fn send_message(&self, message: &CommunicationMessage) -> Result<MessageDelivery, DomainError> {
        info!(
            message_id = %message.id,
            mode = %message.mode,
            priority = ?message.priority,
            "Sending communication message"
        );
        
        let mut delivery = MessageDelivery::new(message.id, message.mode.clone());
        delivery.mark_processing();
        
        let result = match &message.mode {
            CommunicationMode::Email => self.send_email_message(message).await,
            CommunicationMode::Notification => self.send_notification_message(message).await,
            CommunicationMode::Sms => self.send_sms_message(message).await,
        };
        
        match result {
            Ok(provider_message_id) => {
                delivery.mark_sent(Some(provider_message_id));
                info!(
                    message_id = %message.id,
                    delivery_id = %delivery.id,
                    "Message sent successfully"
                );
            }
            Err(e) => {
                let error_details = e.to_string();
                delivery.mark_failed(error_details);
                error!(
                    message_id = %message.id,
                    delivery_id = %delivery.id,
                    error = %e,
                    "Message delivery failed"
                );
                return Err(e);
            }
        }
        
        Ok(delivery)
    }
    
    fn supports_mode(&self, mode: &CommunicationMode) -> bool {
        match mode {
            CommunicationMode::Email => true,
            CommunicationMode::Notification => true,
            CommunicationMode::Sms => true,
        }
    }
    
    async fn health_check(&self) -> Result<(), DomainError> {
        let mut errors = Vec::new();
        
        if let Err(e) = self.email_service.health_check().await {
            errors.push(format!("Email service: {}", e));
        }
        
        if let Err(e) = self.notification_service.health_check().await {
            errors.push(format!("Notification service: {}", e));
        }
        
        if let Err(e) = self.sms_service.health_check().await {
            errors.push(format!("SMS service: {}", e));
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(DomainError::service_unavailable(errors.join(", ")))
        }
    }
}

impl CommunicationServiceImpl {
    /// Send email message
    async fn send_email_message(&self, message: &CommunicationMessage) -> Result<String, DomainError> {
        let email = message.recipient.email.as_ref()
            .ok_or_else(|| DomainError::invalid_recipient("Email address is required for email messages".to_string()))?;
        
        match &message.content {
            MessageContent::Email { subject, html_body, text_body, attachments } => {
                // Convert domain attachments to service attachments
                let service_attachments: Vec<EmailAttachment> = attachments.iter()
                    .map(|att| -> Result<EmailAttachment, DomainError> {
                        Ok(EmailAttachment {
                            filename: att.filename.clone(),
                            content_type: att.content_type.clone(),
                            data: base64::engine::general_purpose::STANDARD
                                .decode(&att.data)
                                .map_err(|e| DomainError::invalid_message(format!("Invalid attachment data: {}", e)))?,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                
                self.email_service.send_email(
                    email,
                    subject,
                    text_body,
                    html_body.as_deref(),
                    &service_attachments,
                ).await
            }
            _ => Err(DomainError::invalid_message("Invalid content type for email message".to_string())),
        }
    }
    
    /// Send notification message
    async fn send_notification_message(&self, message: &CommunicationMessage) -> Result<String, DomainError> {
        match &message.content {
            MessageContent::Notification { title, body, data, .. } => {
                self.notification_service.send_notification(
                    message.recipient.device_token.as_deref(),
                    message.recipient.user_id.as_ref().map(|id| id.to_string()).as_deref(),
                    title,
                    body,
                    data,
                ).await
            }
            _ => Err(DomainError::invalid_message("Invalid content type for notification message".to_string())),
        }
    }
    
    /// Send SMS message
    async fn send_sms_message(&self, message: &CommunicationMessage) -> Result<String, DomainError> {
        let phone = message.recipient.phone.as_ref()
            .ok_or_else(|| DomainError::invalid_recipient("Phone number is required for SMS messages".to_string()))?;
        
        match &message.content {
            MessageContent::Sms { text } => {
                self.sms_service.send_sms(phone, text).await
            }
            _ => Err(DomainError::invalid_message("Invalid content type for SMS message".to_string())),
        }
    }
} 