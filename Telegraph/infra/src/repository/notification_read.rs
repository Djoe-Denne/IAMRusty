use sea_orm::*;
use uuid::Uuid;

use anyhow::Result;
use std::sync::Arc;
use tracing::debug;

use crate::repository::entity::{notifications, notification_deliveries};
use crate::repository::mappers;
use telegraph_domain::error::DomainError;
use telegraph_domain::port::repository::NotificationReadRepository;
use telegraph_domain::entity::{communication::NotificationCommunication, delivery::MessageDelivery};

/// Repository for reading notifications
#[derive(Clone)]
pub struct NotificationReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl NotificationReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl NotificationReadRepository for NotificationReadRepositoryImpl {
    /// Get notifications for a user
    async fn get_user_notifications(
        &self,
        user_id: Uuid,
        page: u64,
        per_page: u64,
        unread_only: bool,
    ) -> Result<(Vec<NotificationCommunication>, u64), DomainError> {
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

        let domain_notifications: Result<Vec<NotificationCommunication>, DomainError> = notifications
            .into_iter()
            .map(mappers::to_domain_notification)
            .collect();

        Ok((domain_notifications?, total))
    }

    /// Get a notification by ID
    async fn get_notification(&self, notification_id: Uuid) -> Result<Option<NotificationCommunication>, DomainError> {
        debug!("Reading notification by ID: {}", notification_id);
        
        let notification = notifications::Entity::find_by_id(notification_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to get notification: {}", e)))?;

        match notification {
            Some(model) => Ok(Some(mappers::to_domain_notification(model)?)),
            None => Ok(None),
        }
    }

    /// Get delivery records for a notification
    async fn get_notification_deliveries(
        &self,
        notification_id: Uuid,
    ) -> Result<Vec<MessageDelivery>, DomainError> {
        debug!("Reading delivery records for notification: {}", notification_id);
        
        let deliveries = notification_deliveries::Entity::find()
            .filter(notification_deliveries::Column::NotificationId.eq(notification_id))
            .order_by_desc(notification_deliveries::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::infrastructure_error(format!("Failed to get delivery records: {}", e)))?;

        let domain_deliveries: Result<Vec<MessageDelivery>, DomainError> = deliveries
            .into_iter()
            .map(mappers::to_domain_delivery)
            .collect();

        domain_deliveries
    }
}
