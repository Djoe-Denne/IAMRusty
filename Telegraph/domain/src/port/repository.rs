//! Repository port traits for Telegraph communication service
//!
//! These traits define the repository interface using domain entities only.
//! Infrastructure implementations should handle mapping between domain and infrastructure entities.

use crate::entity::communication::{CommunicationMode, NotificationCommunication};
use crate::entity::delivery::MessageDelivery;
use crate::error::DomainError;
use uuid::Uuid;

/// Read operations for `CommunicationMessage` entity
#[async_trait::async_trait]
pub trait NotificationReadRepository: Send + Sync {
    /// Get notifications for a user
    async fn get_user_notifications(
        &self,
        user_id: Uuid,
        page: u8,
        per_page: u8,
        unread_only: bool,
    ) -> Result<(Vec<NotificationCommunication>, u64), DomainError>;

    /// Get a notification by id
    async fn get_notification(
        &self,
        notification_id: Uuid,
    ) -> Result<Option<NotificationCommunication>, DomainError>;

    /// Get notification deliveries
    async fn get_notification_deliveries(
        &self,
        notification_id: Uuid,
    ) -> Result<Vec<MessageDelivery>, DomainError>;

    /// Count unread notifications for a user
    async fn count_unread_notifications(&self, user_id: Uuid) -> Result<u64, DomainError>;

    /// Check if a user has a notification
    async fn user_has_notification(
        &self,
        user_id: Uuid,
        notification_id: Uuid,
    ) -> Result<bool, DomainError>;
}

#[async_trait::async_trait]
pub trait NotificationWriteRepository: Send + Sync {
    /// Create a notification
    async fn create_notification(
        &self,
        notification: NotificationCommunication,
    ) -> Result<NotificationCommunication, DomainError>;

    /// Create a notification and its delivery record atomically.
    async fn create_notification_with_delivery(
        &self,
        notification: NotificationCommunication,
        delivery_mode: CommunicationMode,
    ) -> Result<(NotificationCommunication, MessageDelivery), DomainError>;

    /// Mark notification as read
    async fn mark_as_read(
        &self,
        notification_id: Uuid,
    ) -> Result<NotificationCommunication, DomainError>;

    /// Delete expired notifications
    async fn delete_expired_notifications(&self) -> Result<u64, DomainError>;

    /// Create a delivery record    
    async fn create_delivery(
        &self,
        delivery: MessageDelivery,
    ) -> Result<MessageDelivery, DomainError>;

    /// Update a delivery record
    async fn update_delivery_status(
        &self,
        delivery_id: Uuid,
        status: String,
        error_message: Option<String>,
    ) -> Result<MessageDelivery, DomainError>;

    /// Increment delivery attempt
    async fn increment_delivery_attempt(
        &self,
        delivery_id: Uuid,
    ) -> Result<MessageDelivery, DomainError>;
}

#[async_trait::async_trait]
pub trait NotificationRepository:
    Send + Sync + NotificationReadRepository + NotificationWriteRepository
{
}
