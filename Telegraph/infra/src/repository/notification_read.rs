use sea_orm::*;
use uuid::Uuid;

use anyhow::Result;
use std::sync::Arc;
use tracing::debug;

use crate::repository::entity::{notifications, notification_deliveries};
use telegraph_domain::error::DomainError;

/// Repository for reading notifications
#[derive(Clone)]
pub struct NotificationReadRepository {
    db: Arc<DatabaseConnection>,
}

impl NotificationReadRepository {
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

    /// Get notifications for a user
    pub async fn get_user_notifications(
        &self,
        user_id: Uuid,
        page: u64,
        per_page: u64,
        unread_only: bool,
    ) -> Result<(Vec<notifications::Model>, u64), DomainError> {
        debug!("Reading notifications for user: {}", user_id);
        
        let mut query = notifications::Entity::find()
            .filter(notifications::Column::UserId.eq(user_id))
            .order_by_desc(notifications::Column::CreatedAt);

        if unread_only {
            query = query.filter(notifications::Column::IsRead.eq(false));
        }

        let paginator = query.paginate(self.db.as_ref(), per_page);
        let total = paginator.num_items().await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to count notifications: {}", e)))?;

        let notifications = paginator.fetch_page(page).await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to fetch notifications: {}", e)))?;

        Ok((notifications.into_iter().map(Self::to_domain_notification).collect(), total))
    }

    /// Get a notification by ID
    pub async fn get_notification(&self, notification_id: Uuid) -> Result<Option<notifications::Model>, DomainError> {
        debug!("Reading notification by ID: {}", notification_id);
        
        let notification = notifications::Entity::find_by_id(notification_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to get notification: {}", e)))?;

        Ok(notification.map(Self::to_domain_notification))
    }

    /// Get delivery records for a notification
    pub async fn get_notification_deliveries(
        &self,
        notification_id: Uuid,
    ) -> Result<Vec<notification_deliveries::Model>, DomainError> {
        debug!("Reading delivery records for notification: {}", notification_id);
        
        let deliveries = notification_deliveries::Entity::find()
            .filter(notification_deliveries::Column::NotificationId.eq(notification_id))
            .order_by_desc(notification_deliveries::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to get delivery records: {}", e)))?;

        Ok(deliveries.into_iter().map(Self::to_domain_delivery).collect())
    }

    /// Get pending deliveries for retry
    pub async fn get_pending_deliveries(&self) -> Result<Vec<notification_deliveries::Model>, DomainError> {
        debug!("Reading pending deliveries for retry");
        
        let deliveries = notification_deliveries::Entity::find()
            .filter(notification_deliveries::Column::Status.eq("pending"))
            .filter(notification_deliveries::Column::AttemptCount.lt(3))
            .order_by_asc(notification_deliveries::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to get pending deliveries: {}", e)))?;

        Ok(deliveries.into_iter().map(Self::to_domain_delivery).collect())
    }
} 