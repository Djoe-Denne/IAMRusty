//! Communication use case for Telegraph application

use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

use telegraph_domain::{
    DomainError, CommunicationService, MessageDelivery,
    CommunicationMessage, MessageRecipient, MessageContent
};
use crate::command::{SendMessageCommand, SendMessageContent};

/// Use case for handling communication operations
pub struct CommunicationUseCase {
    communication_service: Arc<dyn CommunicationService>,
}

impl CommunicationUseCase {
    /// Create a new communication use case
    pub fn new(communication_service: Arc<dyn CommunicationService>) -> Self {
        Self {
            communication_service,
        }
    }
    
    /// Send a message based on a command
    pub async fn send_message(&self, command: SendMessageCommand) -> Result<MessageDelivery, DomainError> {
        info!(
            mode = %command.mode,
            priority = ?command.priority,
            "Processing send message command"
        );
        
        // Convert command to domain message
        let message = self.convert_command_to_message(command)?;
        
        // Send the message
        let delivery = self.communication_service.send_message(&message).await?;
        
        info!(
            message_id = %message.id,
            delivery_id = %delivery.id,
            "Message sent successfully"
        );
        
        Ok(delivery)
    }
    
    /// Convert command to domain message
    fn convert_command_to_message(&self, command: SendMessageCommand) -> Result<CommunicationMessage, DomainError> {
        // Convert recipient
        let recipient = MessageRecipient {
            user_id: command.recipient.user_id,
            email: command.recipient.email,
            phone: command.recipient.phone,
            device_token: command.recipient.device_token,
            display_name: command.recipient.display_name,
        };
        
        // Convert content
        let content = match command.content {
            SendMessageContent::Email { subject, html_body, text_body } => {
                MessageContent::Email {
                    subject,
                    html_body,
                    text_body,
                    attachments: vec![], // No attachments in basic command
                }
            }
            SendMessageContent::Notification { title, body, data, icon, click_action } => {
                MessageContent::Notification {
                    title,
                    body,
                    data,
                    icon,
                    click_action,
                }
            }
            SendMessageContent::Sms { text } => {
                MessageContent::Sms { text }
            }
        };
        
        // Create message
        let mut message = CommunicationMessage::new(recipient, content, command.mode)?
            .with_priority(command.priority);
        
        // Add metadata
        for (key, value) in command.metadata {
            message = message.with_metadata(key, value);
        }
        
        Ok(message)
    }
}

/// Trait for communication use case
#[async_trait]
pub trait CommunicationUseCaseTrait: Send + Sync {
    /// Send a message
    async fn send_message(&self, command: SendMessageCommand) -> Result<MessageDelivery, DomainError>;
}

#[async_trait]
impl CommunicationUseCaseTrait for CommunicationUseCase {
    async fn send_message(&self, command: SendMessageCommand) -> Result<MessageDelivery, DomainError> {
        self.send_message(command).await
    }
} 