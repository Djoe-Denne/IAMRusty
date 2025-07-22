use std::sync::Arc;
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
}