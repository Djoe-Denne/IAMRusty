use serde::{Deserialize, Serialize};
use std::sync::Arc;
use telegraph_domain::entity::communication::NotificationCommunication;
use telegraph_domain::error::DomainError;
use telegraph_domain::service::NotificationService;
use uuid::Uuid;

/// Input for getting user notifications
#[derive(Debug, Clone, Deserialize)]
pub struct GetNotificationsInput {
    pub user_id: Uuid,
    pub page: Option<u8>,
    pub per_page: Option<u8>,
    pub unread_only: Option<bool>,
}

/// Response for getting user notifications
#[derive(Debug, Clone, Serialize)]
pub struct GetNotificationsResponse {
    pub notifications: Vec<NotificationResponse>,
    pub total_count: u64,
    pub page: u8,
    pub per_page: u8,
    pub has_more: bool,
}

/// Individual notification response
#[derive(Debug, Clone, Serialize)]
pub struct NotificationResponse {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub is_read: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Input for counting unread notifications
#[derive(Debug, Clone, Deserialize)]
pub struct GetUnreadCountInput {
    pub user_id: Uuid,
}

/// Response for unread count
#[derive(Debug, Clone, Serialize)]
pub struct GetUnreadCountResponse {
    pub unread_count: u64,
}

/// Input for marking notification as read
#[derive(Debug, Clone, Deserialize)]
pub struct MarkNotificationReadInput {
    pub notification_id: Uuid,
    pub user_id: Uuid,
}

/// Response for marking notification as read
#[derive(Debug, Clone, Serialize)]
pub struct MarkNotificationReadResponse {
    pub notification: NotificationResponse,
    pub success: bool,
}

/// Error types for notification usecases
#[derive(Debug, Clone)]
pub enum NotificationUseCaseError {
    Domain(DomainError),
    ValidationError(String),
}

impl From<DomainError> for NotificationUseCaseError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

/// Trait for notification-related usecases
#[async_trait::async_trait]
pub trait NotificationUseCaseTrait: Send + Sync {
    async fn get_notifications(
        &self,
        input: GetNotificationsInput,
    ) -> Result<GetNotificationsResponse, NotificationUseCaseError>;
    async fn get_unread_count(
        &self,
        input: GetUnreadCountInput,
    ) -> Result<GetUnreadCountResponse, NotificationUseCaseError>;
    async fn mark_notification_read(
        &self,
        input: MarkNotificationReadInput,
    ) -> Result<MarkNotificationReadResponse, NotificationUseCaseError>;
}

/// Implementation of notification usecases
pub struct NotificationUseCaseImpl {
    notification_service: Arc<dyn NotificationService>,
}

impl NotificationUseCaseImpl {
    pub fn new(notification_service: Arc<dyn NotificationService>) -> Self {
        Self {
            notification_service,
        }
    }

    /// Convert domain notification to response
    fn to_notification_response(notification: NotificationCommunication) -> NotificationResponse {
        NotificationResponse {
            id: notification.id.unwrap_or_else(|| Uuid::new_v4()),
            title: notification.title,
            body: notification.body,
            is_read: notification.is_read.unwrap_or(false),
            created_at: notification.created_at.unwrap_or_else(chrono::Utc::now),
            read_at: notification.read_at,
        }
    }
}

#[async_trait::async_trait]
impl NotificationUseCaseTrait for NotificationUseCaseImpl {
    async fn get_notifications(
        &self,
        input: GetNotificationsInput,
    ) -> Result<GetNotificationsResponse, NotificationUseCaseError> {
        // Set defaults
        let page = input.page.unwrap_or(0);
        let per_page = input.per_page.unwrap_or(20);
        let unread_only = input.unread_only.unwrap_or(false);

        // Validate per_page limit
        if per_page > 100 {
            return Err(NotificationUseCaseError::ValidationError(
                "per_page cannot exceed 100".to_string(),
            ));
        }

        // Get notifications from domain service
        let (notifications, total_count) = self
            .notification_service
            .get_user_notifications(input.user_id, page, per_page, unread_only)
            .await?;

        // Convert to response format
        let notification_responses: Vec<NotificationResponse> = notifications
            .into_iter()
            .map(Self::to_notification_response)
            .collect();

        let current_last_index = (page + 1) as u64 * per_page as u64;
        let has_more = current_last_index < total_count;

        Ok(GetNotificationsResponse {
            notifications: notification_responses,
            total_count,
            page,
            per_page,
            has_more,
        })
    }

    async fn get_unread_count(
        &self,
        input: GetUnreadCountInput,
    ) -> Result<GetUnreadCountResponse, NotificationUseCaseError> {
        let unread_count = self
            .notification_service
            .count_unread_notifications(input.user_id)
            .await?;

        Ok(GetUnreadCountResponse { unread_count })
    }

    async fn mark_notification_read(
        &self,
        input: MarkNotificationReadInput,
    ) -> Result<MarkNotificationReadResponse, NotificationUseCaseError> {
        let updated_notification = self
            .notification_service
            .mark_notification_as_read(input.notification_id, input.user_id)
            .await?;

        let notification_response = Self::to_notification_response(updated_notification);

        Ok(MarkNotificationReadResponse {
            notification: notification_response,
            success: true,
        })
    }
}
