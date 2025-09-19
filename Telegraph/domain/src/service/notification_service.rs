use crate::entity::communication::NotificationCommunication;
use crate::entity::delivery::MessageDelivery;
use crate::error::DomainError;
use crate::port::repository::NotificationRepository;
use std::sync::Arc;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait NotificationService: Send + Sync {
    async fn create_notification(
        &self,
        notification: NotificationCommunication,
    ) -> Result<NotificationCommunication, DomainError>;
    
    async fn create_delivery(
        &self,
        delivery: MessageDelivery,
    ) -> Result<MessageDelivery, DomainError>;
    
    
    async fn get_user_notifications(
        &self,
        user_id: Uuid,
        page: u8,
        per_page: u8,
        unread_only: bool,
    ) -> Result<(Vec<NotificationCommunication>, u64), DomainError>;
    
    
    async fn count_unread_notifications(&self, user_id: Uuid) -> Result<u64, DomainError>;
    
    async fn mark_notification_as_read(
        &self,
        notification_id: Uuid,
        user_id: Uuid,
    ) -> Result<NotificationCommunication, DomainError>;

    async fn user_has_notification(&self, user_id: Uuid, notification_id: Uuid) -> bool;
    
}

pub struct NotificationServiceImpl<NR> {
    notification_repo: Arc<NR>,
}

impl<NR> NotificationServiceImpl<NR> {
    pub fn new(notification_repo: Arc<NR>) -> Self {
        Self { notification_repo }
    }
}

#[async_trait::async_trait]
impl<NR> NotificationService for NotificationServiceImpl<NR>
where
    NR: NotificationRepository,
{

    async fn create_notification(
        &self,
        notification: NotificationCommunication,
    ) -> Result<NotificationCommunication, DomainError> {
        self.notification_repo
            .create_notification(notification.clone())
            .await
    }

    async fn create_delivery(
        &self,
        delivery: MessageDelivery,
    ) -> Result<MessageDelivery, DomainError> {
        self.notification_repo.create_delivery(delivery).await
    }

    /// Get notifications for a user with pagination and filtering
    async fn get_user_notifications(
        &self,
        user_id: Uuid,
        page: u8,
        per_page: u8,
        unread_only: bool,
    ) -> Result<(Vec<NotificationCommunication>, u64), DomainError> {
        self.notification_repo
            .get_user_notifications(user_id, page, per_page, unread_only)
            .await
    }

    /// Count unread notifications for a user
    async fn count_unread_notifications(&self, user_id: Uuid) -> Result<u64, DomainError> {
        self.notification_repo
            .count_unread_notifications(user_id)
            .await
    }

    /// Mark a notification as read
    async fn mark_notification_as_read(
        &self,
        notification_id: Uuid,
        user_id: Uuid,
    ) -> Result<NotificationCommunication, DomainError> {
        // First check if the notification exists and belongs to the user
        if let Some(notification) = self
            .notification_repo
            .get_notification(notification_id)
            .await?
        {
            if notification.recipient.user_id != Some(user_id) {
                return Err(DomainError::unauthorized(
                    "Notification does not belong to this user".to_string(),
                ));
            }

            // Mark as read
            self.notification_repo.mark_as_read(notification_id).await
        } else {
            Err(DomainError::notification_not_found(format!(
                "Notification not found: {}",
                notification_id
            )))
        }
    }

    async fn user_has_notification(&self, user_id: Uuid, notification_id: Uuid) -> bool  {
        self.notification_repo.user_has_notification(user_id, notification_id).await.unwrap_or(false)
    }
}
