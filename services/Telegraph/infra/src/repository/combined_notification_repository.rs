use anyhow::Result;
use std::sync::Arc;
use telegraph_domain::entity::{
    communication::{CommunicationMode, NotificationCommunication},
    delivery::MessageDelivery,
};
use telegraph_domain::error::DomainError;
use telegraph_domain::port::repository::{
    NotificationReadRepository, NotificationRepository, NotificationWriteRepository,
};
use uuid::Uuid;

/// Combined Notification Repository that delegates to separate read/write implementations
#[derive(Clone)]
pub struct CombinedNotificationRepositoryImpl {
    read_repo: Arc<dyn NotificationReadRepository>,
    write_repo: Arc<dyn NotificationWriteRepository>,
}

impl CombinedNotificationRepositoryImpl {
    /// Create a new combined repository
    pub fn new(
        read_repo: Arc<dyn NotificationReadRepository>,
        write_repo: Arc<dyn NotificationWriteRepository>,
    ) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait::async_trait]
impl NotificationReadRepository for CombinedNotificationRepositoryImpl {
    // Read operations - delegate to read repository
    /// Get notifications for a user
    async fn get_user_notifications(
        &self,
        user_id: Uuid,
        page: u8,
        per_page: u8,
        unread_only: bool,
    ) -> Result<(Vec<NotificationCommunication>, u64), DomainError> {
        self.read_repo
            .get_user_notifications(user_id, page, per_page, unread_only)
            .await
    }

    /// Get a notification by ID
    async fn get_notification(
        &self,
        notification_id: Uuid,
    ) -> Result<Option<NotificationCommunication>, DomainError> {
        self.read_repo.get_notification(notification_id).await
    }

    /// Get delivery records for a notification
    async fn get_notification_deliveries(
        &self,
        notification_id: Uuid,
    ) -> Result<Vec<MessageDelivery>, DomainError> {
        self.read_repo
            .get_notification_deliveries(notification_id)
            .await
    }

    async fn count_unread_notifications(&self, user_id: Uuid) -> Result<u64, DomainError> {
        self.read_repo.count_unread_notifications(user_id).await
    }

    async fn user_has_notification(
        &self,
        user_id: Uuid,
        notification_id: Uuid,
    ) -> Result<bool, DomainError> {
        self.read_repo
            .user_has_notification(user_id, notification_id)
            .await
    }
}

#[async_trait::async_trait]
impl NotificationWriteRepository for CombinedNotificationRepositoryImpl {
    // Write operations - delegate to write repository

    /// Create a new notification
    async fn create_notification(
        &self,
        notification: NotificationCommunication,
    ) -> Result<NotificationCommunication, DomainError> {
        self.write_repo.create_notification(notification).await
    }

    async fn create_notification_with_delivery(
        &self,
        notification: NotificationCommunication,
        delivery_mode: CommunicationMode,
    ) -> Result<(NotificationCommunication, MessageDelivery), DomainError> {
        self.write_repo
            .create_notification_with_delivery(notification, delivery_mode)
            .await
    }

    /// Mark notification as read
    async fn mark_as_read(
        &self,
        notification_id: Uuid,
    ) -> Result<NotificationCommunication, DomainError> {
        self.write_repo.mark_as_read(notification_id).await
    }

    /// Delete expired notifications
    async fn delete_expired_notifications(&self) -> Result<u64, DomainError> {
        self.write_repo.delete_expired_notifications().await
    }

    /// Create a delivery record for a notification
    async fn create_delivery(
        &self,
        delivery: MessageDelivery,
    ) -> Result<MessageDelivery, DomainError> {
        self.write_repo.create_delivery(delivery).await
    }

    /// Update delivery status
    async fn update_delivery_status(
        &self,
        delivery_id: Uuid,
        status: String,
        error_message: Option<String>,
    ) -> Result<MessageDelivery, DomainError> {
        self.write_repo
            .update_delivery_status(delivery_id, status, error_message)
            .await
    }

    /// Increment delivery attempt count
    async fn increment_delivery_attempt(
        &self,
        delivery_id: Uuid,
    ) -> Result<MessageDelivery, DomainError> {
        self.write_repo
            .increment_delivery_attempt(delivery_id)
            .await
    }
}

impl NotificationRepository for CombinedNotificationRepositoryImpl {}
