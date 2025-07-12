//! Composite communication service implementation

use async_trait::async_trait;
use telegraph_domain::{
    DomainError, CommunicationService, EmailService, SmsService, NotificationService,
    CommunicationMessage, MessageDelivery, CommunicationMode, MessageContent
};
use telegraph_domain::port::EmailAttachment as PortEmailAttachment;
use telegraph_domain::entity::EmailAttachment as EntityEmailAttachment;
use std::sync::Arc;
use tracing::{info, error};

/// Composite communication service that coordinates multiple communication channels
pub struct CompositeCommunicationService {
    email_service: Arc<dyn EmailService>,
    sms_service: Arc<dyn SmsService>,
    notification_service: Arc<dyn NotificationService>,
}

impl CompositeCommunicationService {
    /// Create a new composite communication service
    pub fn new(
        email_service: Arc<dyn EmailService>,
        sms_service: Arc<dyn SmsService>,
        notification_service: Arc<dyn NotificationService>,
    ) -> Self {
        Self {
            email_service,
            sms_service,
            notification_service,
        }
    }
    
    /// Convert entity EmailAttachment to port EmailAttachment
    fn convert_attachments(entity_attachments: &[EntityEmailAttachment]) -> Result<Vec<PortEmailAttachment>, DomainError> {
        entity_attachments
            .iter()
            .map(|attachment| {
                // Decode base64 data
                let data = base64::decode(&attachment.data)
                    .map_err(|e| DomainError::infrastructure_error(format!("Failed to decode attachment data: {}", e)))?;
                
                Ok(PortEmailAttachment {
                    filename: attachment.filename.clone(),
                    content_type: attachment.content_type.clone(),
                    data,
                })
            })
            .collect()
    }
}

#[async_trait]
impl CommunicationService for CompositeCommunicationService {
    async fn send_message(&self, message: &CommunicationMessage) -> Result<MessageDelivery, DomainError> {
        info!(
            message_id = %message.id,
            mode = ?message.mode,
            recipient_email = message.recipient.email.as_deref(),
            "Sending communication message"
        );
        
        let provider_message_id = match &message.content {
            MessageContent::Email { subject, html_body, text_body, attachments } => {
                if let Some(email) = &message.recipient.email {
                    // Convert attachments from entity type to port type
                    let port_attachments = Self::convert_attachments(attachments)?;
                    
                    self.email_service
                        .send_email(email, subject, text_body, html_body.as_deref(), &port_attachments)
                        .await?
                } else {
                    return Err(DomainError::invalid_recipient("Email address required for email message".to_string()));
                }
            }
            MessageContent::Sms { text } => {
                if let Some(phone) = &message.recipient.phone {
                    self.sms_service
                        .send_sms(phone, text)
                        .await?
                } else {
                    return Err(DomainError::invalid_recipient("Phone number required for SMS message".to_string()));
                }
            }
            MessageContent::Notification { title, body, data, .. } => {
                self.notification_service
                    .send_notification(
                        message.recipient.device_token.as_deref(),
                        message.recipient.user_id.as_ref().map(|u| u.to_string()).as_deref(),
                        title,
                        body,
                        data,
                    )
                    .await?
            }
        };
        
        // Create delivery record and mark as sent
        let mut delivery = MessageDelivery::new(message.id, message.mode.clone());
        delivery.mark_sent(Some(provider_message_id));
        
        info!(
            message_id = %message.id,
            delivery_id = %delivery.id,
            provider_message_id = ?delivery.provider_message_id,
            "Message sent successfully"
        );
        
        Ok(delivery)
    }
    
    fn supports_mode(&self, mode: &CommunicationMode) -> bool {
        // Support all communication modes since we have all services
        matches!(mode, CommunicationMode::Email | CommunicationMode::Sms | CommunicationMode::Notification)
    }
    
    async fn health_check(&self) -> Result<(), DomainError> {
        info!("Performing health check for all communication services");
        
        let mut errors = Vec::new();
        
        // Check email service
        if let Err(e) = self.email_service.health_check().await {
            error!(error = %e, "Email service health check failed");
            errors.push(format!("Email service: {}", e));
        }
        
        // Check SMS service
        if let Err(e) = self.sms_service.health_check().await {
            error!(error = %e, "SMS service health check failed");
            errors.push(format!("SMS service: {}", e));
        }
        
        // Check notification service
        if let Err(e) = self.notification_service.health_check().await {
            error!(error = %e, "Notification service health check failed");
            errors.push(format!("Notification service: {}", e));
        }
        
        if !errors.is_empty() {
            let error_msg = format!("Communication service health check failed: {}", errors.join(", "));
            error!("{}", error_msg);
            return Err(DomainError::service_unavailable(error_msg));
        }
        
        info!("✅ All communication services are healthy");
        Ok(())
    }
} 