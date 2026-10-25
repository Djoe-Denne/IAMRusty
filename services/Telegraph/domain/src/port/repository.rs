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
    #[must_use = "await the future"]
    async fn get_user_notifications(
        &self,
        user_id: Uuid,
        page: u8,
        per_page: u8,
        unread_only: bool,
    ) -> Result<(Vec<NotificationCommunication>, u64), DomainError>;

    /// Get a notification by id
    #[must_use = "await the future"]
    async fn get_notification(
        &self,
        notification_id: Uuid,
    ) -> Result<Option<NotificationCommunication>, DomainError>;

    /// Get notification deliveries
    #[must_use = "await the future"]
    async fn get_notification_deliveries(
        &self,
        notification_id: Uuid,
    ) -> Result<Vec<MessageDelivery>, DomainError>;

    /// Count unread notifications for a user
    #[must_use = "await the future"]
    async fn count_unread_notifications(&self, user_id: Uuid) -> Result<u64, DomainError>;

    /// Check if a user has a notification
    #[must_use = "await the future"]
    async fn user_has_notification(
        &self,
        user_id: Uuid,
        notification_id: Uuid,
    ) -> Result<bool, DomainError>;
}

#[async_trait::async_trait]
pub trait NotificationWriteRepository: Send + Sync {
    /// Atomically create a notification and its durable `NotificationCreated`
    /// intention. Success commits both, even when publication is suspended.
    /// Validation or persistence failure commits neither; this port never
    /// publishes directly. An ambiguous commit cannot promise zero writes.
    #[must_use = "await the future"]
    async fn create_notification(
        &self,
        notification: NotificationCommunication,
    ) -> Result<NotificationCommunication, DomainError>;

    /// Atomically create a notification, its delivery, and exactly one durable
    /// `NotificationCreated` intention on the writer, or commit none of them.
    /// Publication availability must not change the creation result.
    #[must_use = "await the future"]
    async fn create_notification_with_delivery(
        &self,
        notification: NotificationCommunication,
        delivery_mode: CommunicationMode,
    ) -> Result<(NotificationCommunication, MessageDelivery), DomainError>;

    /// Mark notification as read
    #[must_use = "await the future"]
    async fn mark_as_read(
        &self,
        notification_id: Uuid,
    ) -> Result<NotificationCommunication, DomainError>;

    /// Delete expired notifications
    #[must_use = "await the future"]
    async fn delete_expired_notifications(&self) -> Result<u64, DomainError>;

    /// Create a delivery record
    #[must_use = "await the future"]
    async fn create_delivery(
        &self,
        delivery: MessageDelivery,
    ) -> Result<MessageDelivery, DomainError>;

    /// Update a delivery record
    #[must_use = "await the future"]
    async fn update_delivery_status(
        &self,
        delivery_id: Uuid,
        status: String,
        error_message: Option<String>,
    ) -> Result<MessageDelivery, DomainError>;

    /// Increment delivery attempt
    #[must_use = "await the future"]
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
