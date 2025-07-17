use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::Result;

use crate::repository::entity::{notifications, notification_deliveries};
use crate::repository::notification_read::NotificationReadRepository;
use crate::repository::notification_write::NotificationWriteRepository;
use telegraph_domain::error::DomainError;

/// Combined Notification Repository that delegates to separate read/write implementations
#[derive(Clone)]
pub struct CombinedNotificationRepository {
    read_repo: NotificationReadRepository,
    write_repo: NotificationWriteRepository,
}

impl CombinedNotificationRepository {
    /// Create a new combined repository
    pub fn new(read_repo: NotificationReadRepository, write_repo: NotificationWriteRepository) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }

    // Read operations - delegate to read repository
    
    /// Get notifications for a user
    pub async fn get_user_notifications(
        &self,
        user_id: Uuid,
        page: u64,
        per_page: u64,
        unread_only: bool,
    ) -> Result<(Vec<notifications::Model>, u64), DomainError> {
        self.read_repo.get_user_notifications(user_id, page, per_page, unread_only).await
    }

    /// Get a notification by ID
    pub async fn get_notification(&self, notification_id: Uuid) -> Result<Option<notifications::Model>, DomainError> {
        self.read_repo.get_notification(notification_id).await
    }

    /// Get delivery records for a notification
    pub async fn get_notification_deliveries(
        &self,
        notification_id: Uuid,
    ) -> Result<Vec<notification_deliveries::Model>, DomainError> {
        self.read_repo.get_notification_deliveries(notification_id).await
    }

    /// Get pending deliveries for retry
    pub async fn get_pending_deliveries(&self) -> Result<Vec<notification_deliveries::Model>, DomainError> {
        self.read_repo.get_pending_deliveries().await
    }

    // Write operations - delegate to write repository

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
        self.write_repo.create_notification(user_id, title, content, content_type, priority, expires_at).await
    }

    /// Mark notification as read
    pub async fn mark_as_read(&self, notification_id: Uuid) -> Result<notifications::Model, DomainError> {
        self.write_repo.mark_as_read(notification_id).await
    }

    /// Delete expired notifications
    pub async fn delete_expired_notifications(&self) -> Result<u64, DomainError> {
        self.write_repo.delete_expired_notifications().await
    }

    /// Create a delivery record for a notification
    pub async fn create_delivery(
        &self,
        notification_id: Uuid,
        delivery_method: String,
    ) -> Result<notification_deliveries::Model, DomainError> {
        self.write_repo.create_delivery(notification_id, delivery_method).await
    }

    /// Update delivery status
    pub async fn update_delivery_status(
        &self,
        delivery_id: Uuid,
        status: String,
        error_message: Option<String>,
    ) -> Result<notification_deliveries::Model, DomainError> {
        self.write_repo.update_delivery_status(delivery_id, status, error_message).await
    }

    /// Increment delivery attempt count
    pub async fn increment_delivery_attempt(
        &self,
        delivery_id: Uuid,
    ) -> Result<notification_deliveries::Model, DomainError> {
        self.write_repo.increment_delivery_attempt(delivery_id).await
    }
} 