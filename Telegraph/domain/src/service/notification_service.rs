use crate::entity::communication::{CommunicationMode, NotificationCommunication};
use crate::entity::delivery::MessageDelivery;
use crate::error::DomainError;
use crate::port::repository::NotificationRepository;
use rustycog::events::{DomainEvent, EventPublisher};
use std::sync::Arc;
use telegraph_events::{NotificationCreatedEvent, TelegraphDomainEvent};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait NotificationService: Send + Sync {
    async fn create_notification(
        &self,
        notification: NotificationCommunication,
    ) -> Result<NotificationCommunication, DomainError>;

    async fn create_notification_with_delivery(
        &self,
        notification: NotificationCommunication,
        delivery_mode: CommunicationMode,
    ) -> Result<(NotificationCommunication, MessageDelivery), DomainError>;

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
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
}

impl<NR> NotificationServiceImpl<NR> {
    pub const fn new(
        notification_repo: Arc<NR>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
    ) -> Self {
        Self {
            notification_repo,
            event_publisher,
        }
    }

    async fn publish_notification_created(
        &self,
        notification: &NotificationCommunication,
    ) -> Result<(), DomainError> {
        let notification_id = notification.id.ok_or_else(|| {
            DomainError::internal_error("Notification created without an assigned id")
        })?;
        let user_id = notification.recipient.user_id.ok_or_else(|| {
            DomainError::event_processing_error(
                "Cannot publish NotificationCreated without a recipient user id",
            )
        })?;
        let created_at = notification.created_at.unwrap_or_else(chrono::Utc::now);
        let event = TelegraphDomainEvent::NotificationCreated(NotificationCreatedEvent::new(
            notification_id,
            user_id,
            created_at,
        ));
        let domain_ev: Box<dyn DomainEvent> = event.into();
        self.event_publisher.publish(domain_ev.as_ref()).await
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
        let created = self
            .notification_repo
            .create_notification(notification)
            .await?;
        self.publish_notification_created(&created).await?;
        Ok(created)
    }

    async fn create_notification_with_delivery(
        &self,
        notification: NotificationCommunication,
        delivery_mode: CommunicationMode,
    ) -> Result<(NotificationCommunication, MessageDelivery), DomainError> {
        let created = self
            .notification_repo
            .create_notification_with_delivery(notification, delivery_mode)
            .await?;
        self.publish_notification_created(&created.0).await?;
        Ok(created)
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
                "Notification not found: {notification_id}"
            )))
        }
    }

    async fn user_has_notification(&self, user_id: Uuid, notification_id: Uuid) -> bool {
        self.notification_repo
            .user_has_notification(user_id, notification_id)
            .await
            .unwrap_or(false)
    }
}
