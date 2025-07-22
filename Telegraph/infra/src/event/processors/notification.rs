//! Notification event processors for Telegraph communication service

use async_trait::async_trait;
use std::collections::HashMap;
use telegraph_domain::{DomainError, NotificationService, EventHandler, EventContext, CommunicationFactory, MessageDelivery, CommunicationMode};
use std::sync::Arc;
use tracing::info;
use serde_json::json;

/// Database notification event processor - creates notification records in database
pub struct DatabaseNotificationProcessor {
    notification_service: Arc<NotificationService>,
    communication_factory: Arc<CommunicationFactory>,
}

impl DatabaseNotificationProcessor {
    /// Create a new database notification event processor
    pub fn new(notification_service: Arc<NotificationService>, communication_factory: Arc<CommunicationFactory>) -> Self {
        Self {
            notification_service,
            communication_factory,
        }
    }
    
    /// Create database notification from communication
    async fn save_notification_communication(&self, event: &EventContext) -> Result<(), DomainError> {
        let notification_communication = self.communication_factory
            .build_notification_communication(event)
            .await?;

        let user_id = notification_communication.recipient.user_id
            .ok_or(DomainError::EventProcessingError("No user ID found in notification communication".to_string()))?;

        info!(
            event_id = %event.event_id,
            event_type = event.event_type.to_string(),
            user_id = %user_id,
            title = %notification_communication.title,
            "Creating database notification"
        );
        
        let notification = self.notification_service  
            .create_notification( notification_communication)
            .await?;
        
        // Create delivery record for in-app notification
        let _delivery = self.notification_service
            .create_delivery(MessageDelivery::new(notification.id.unwrap(), CommunicationMode::Notification))
            .await?;
        
        info!(
            event_id = %event.event_id,
            user_id = %user_id,
            notification_id = %notification.id.unwrap(),
            "Database notification created successfully"
        );
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler for DatabaseNotificationProcessor {
    async fn handle_event(&self, event: &EventContext) -> Result<(), DomainError> {
        info!(
            event_id = %event.event_id,
            event_type = event.event_type.to_string(),
            user_id = %event.recipient.user_id.unwrap_or_default(),
            "Processing database notification event"
        );

        // Save the notification to the database
        self.save_notification_communication(event).await?;

        Ok(())
    }
    
    fn priority(&self) -> u32 {
        100 // Default priority
    }
} 