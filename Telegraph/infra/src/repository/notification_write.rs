use sea_orm::*;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, error};

use crate::repository::entity::{notifications, notification_deliveries};
use telegraph_domain::error::DomainError;

/// Repository for writing notifications
#[derive(Clone)]
pub struct NotificationWriteRepository {
    db: Arc<DatabaseConnection>,
}

impl NotificationWriteRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a database model to a domain model with proper DateTime conversion
    fn to_domain_notification(model: notifications::Model) -> notifications::Model {
        notifications::Model {
            id: model.id,
            user_id: model.user_id,
            title: model.title,
            content: model.content,
            content_type: model.content_type,
            is_read: model.is_read,
            priority: model.priority,
            expires_at: model.expires_at,
            created_at: model.created_at,
            updated_at: model.updated_at,
            read_at: model.read_at,
        }
    }

    /// Convert a database delivery model to a domain model with proper DateTime conversion
    fn to_domain_delivery(model: notification_deliveries::Model) -> notification_deliveries::Model {
        notification_deliveries::Model {
            id: model.id,
            notification_id: model.notification_id,
            delivery_method: model.delivery_method,
            status: model.status,
            attempt_count: model.attempt_count,
            last_attempt_at: model.last_attempt_at,
            delivered_at: model.delivered_at,
            error_message: model.error_message,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }

    /// Create a new notification
    pub async fn create_notification(
        &self,
        user_id: Uuid,
        title: String,
        content: Vec<u8>,
        content_type: String,
        priority: i16,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<notifications::Model, DomainError> {
        debug!("Creating new notification for user: {}", user_id);
        
        let notification = notifications::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            title: Set(title),
            content: Set(content),
            content_type: Set(content_type),
            is_read: Set(false),
            priority: Set(priority),
            expires_at: Set(expires_at.map(|dt| dt.naive_utc())),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            read_at: Set(None),
        };

        let result = notifications::Entity::insert(notification)
            .exec_with_returning(self.db.as_ref())
            .await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to create notification: {}", e)))?;

        Ok(Self::to_domain_notification(result))
    }

    /// Mark notification as read
    pub async fn mark_as_read(&self, notification_id: Uuid) -> Result<notifications::Model, DomainError> {
        debug!("Marking notification as read: {}", notification_id);
        
        let notification = notifications::Entity::find_by_id(notification_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to find notification: {}", e)))?
            .ok_or_else(|| {
                error!(notification_id = %notification_id, "Failed to mark notification as read: Notification not found");
                DomainError::invalid_message("Notification not found")
            })?;

        let mut notification: notifications::ActiveModel = notification.into();
        notification.is_read = Set(true);
        notification.read_at = Set(Some(Utc::now().naive_utc()));
        notification.updated_at = Set(Utc::now().naive_utc());

        let result = notification.update(self.db.as_ref()).await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to mark notification as read: {}", e)))?;

        Ok(Self::to_domain_notification(result))
    }

    /// Delete expired notifications
    pub async fn delete_expired_notifications(&self) -> Result<u64, DomainError> {
        debug!("Deleting expired notifications");
        
        let result = notifications::Entity::delete_many()
            .filter(notifications::Column::ExpiresAt.lt(Utc::now().naive_utc()))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to delete expired notifications: {}", e)))?;

        Ok(result.rows_affected)
    }

    /// Create a delivery record for a notification
    pub async fn create_delivery(
        &self,
        notification_id: Uuid,
        delivery_method: String,
    ) -> Result<notification_deliveries::Model, DomainError> {
        debug!("Creating delivery record for notification: {}", notification_id);
        
        let delivery = notification_deliveries::ActiveModel {
            id: Set(Uuid::new_v4()),
            notification_id: Set(notification_id),
            delivery_method: Set(delivery_method),
            status: Set("pending".to_string()),
            attempt_count: Set(0),
            last_attempt_at: Set(None),
            delivered_at: Set(None),
            error_message: Set(None),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
        };

        let result = notification_deliveries::Entity::insert(delivery)
            .exec_with_returning(self.db.as_ref())
            .await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to create delivery record: {}", e)))?;

        Ok(Self::to_domain_delivery(result))
    }

    /// Update delivery status
    pub async fn update_delivery_status(
        &self,
        delivery_id: Uuid,
        status: String,
        error_message: Option<String>,
    ) -> Result<notification_deliveries::Model, DomainError> {
        debug!("Updating delivery status for delivery: {}", delivery_id);
        
        let delivery = notification_deliveries::Entity::find_by_id(delivery_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to find delivery: {}", e)))?
            .ok_or_else(|| {
                error!(delivery_id = %delivery_id, "Failed to update delivery status: Delivery record not found");
                DomainError::invalid_message("Delivery record not found")
            })?;

        let mut delivery: notification_deliveries::ActiveModel = delivery.into();
        delivery.status = Set(status.clone());
        delivery.error_message = Set(error_message);
        delivery.updated_at = Set(Utc::now().naive_utc());
        
        if status == "delivered" {
            delivery.delivered_at = Set(Some(Utc::now().naive_utc()));
        }

        let result = delivery.update(self.db.as_ref()).await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to update delivery status: {}", e)))?;

        Ok(Self::to_domain_delivery(result))
    }

    /// Increment delivery attempt count
    pub async fn increment_delivery_attempt(
        &self,
        delivery_id: Uuid,
    ) -> Result<notification_deliveries::Model, DomainError> {
        debug!("Incrementing delivery attempt for delivery: {}", delivery_id);
        
        let delivery = notification_deliveries::Entity::find_by_id(delivery_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to find delivery: {}", e)))?
            .ok_or_else(|| {
                error!(delivery_id = %delivery_id, "Failed to increment delivery attempt: Delivery record not found");
                DomainError::invalid_message("Delivery record not found")
            })?;

        let mut delivery: notification_deliveries::ActiveModel = delivery.into();
        delivery.attempt_count = Set(delivery.attempt_count.unwrap() + 1);
        delivery.last_attempt_at = Set(Some(Utc::now().naive_utc()));
        delivery.updated_at = Set(Utc::now().naive_utc());

        let result = delivery.update(self.db.as_ref()).await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to increment delivery attempt: {}", e)))?;

        Ok(Self::to_domain_delivery(result))
    }
} 