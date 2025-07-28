use std::sync::Arc;
use uuid::Uuid;
use crate::port::repository::NotificationRepository;
use crate::error::DomainError;
use crate::entity::communication::NotificationCommunication;
use crate::entity::delivery::MessageDelivery;

pub struct NotificationService {
    notification_repo: Arc<dyn NotificationRepository>,
}

impl NotificationService {
    pub fn new(notification_repo: Arc<dyn NotificationRepository>) -> Self {
        Self { notification_repo }
    }

    pub async fn create_notification(&self, notification: NotificationCommunication) -> Result<NotificationCommunication, DomainError> {
        self.notification_repo.create_notification(notification.clone()).await
    }

    pub async fn create_delivery(&self, delivery: MessageDelivery) -> Result<MessageDelivery, DomainError> {
        self.notification_repo.create_delivery(delivery).await
    }

    /// Get notifications for a user with pagination and filtering
    pub async fn get_user_notifications(
        &self,
        user_id: Uuid,
        page: u8,
        per_page: u8,
        unread_only: bool,
    ) -> Result<(Vec<NotificationCommunication>, u64), DomainError> {
        self.notification_repo.get_user_notifications(user_id, page, per_page, unread_only).await
    }

    /// Count unread notifications for a user
    pub async fn count_unread_notifications(&self, user_id: Uuid) -> Result<u64, DomainError> {
        self.notification_repo.count_unread_notifications(user_id).await
    }

    /// Mark a notification as read
    pub async fn mark_notification_as_read(&self, notification_id: Uuid, user_id: Uuid) -> Result<NotificationCommunication, DomainError> {
        // First check if the notification exists and belongs to the user
        if let Some(notification) = self.notification_repo.get_notification(notification_id).await? {
            if notification.recipient.user_id != Some(user_id) {
                return Err(DomainError::unauthorized("Notification does not belong to this user".to_string()));
            }
            
            // Mark as read
            self.notification_repo.mark_as_read(notification_id).await
        } else {
            Err(DomainError::notification_not_found(format!("Notification not found: {}", notification_id)))
        }
    }
}