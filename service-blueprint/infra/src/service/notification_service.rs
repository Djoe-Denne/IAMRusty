use async_trait::async_trait;
use uuid::Uuid;

use {{SERVICE_NAME}}_domain::{DomainError, NotificationService};

/// Dummy notification service for testing and development
pub struct DummyNotificationService;

impl DummyNotificationService {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NotificationService for DummyNotificationService {
    async fn send_notification(
        &self,
        user_id: &Uuid,
        title: &str,
        message: &str,
    ) -> Result<(), DomainError> {
        tracing::info!(
            user_id = %user_id,
            title = title,
            message = message,
            "Dummy notification sent"
        );
        Ok(())
    }

    async fn send_bulk_notification(
        &self,
        user_ids: &[Uuid],
        title: &str,
        message: &str,
    ) -> Result<(), DomainError> {
        tracing::info!(
            user_count = user_ids.len(),
            title = title,
            message = message,
            "Dummy bulk notification sent"
        );
        Ok(())
    }
}

/// Firebase Cloud Messaging notification service
#[cfg(feature = "fcm")]
pub struct FcmNotificationService {
    // FCM client would go here
}

#[cfg(feature = "fcm")]
impl FcmNotificationService {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(feature = "fcm")]
#[async_trait]
impl NotificationService for FcmNotificationService {
    async fn send_notification(
        &self,
        _user_id: &Uuid,
        _title: &str,
        _message: &str,
    ) -> Result<(), DomainError> {
        // Implementation would use FCM SDK
        todo!("Implement FCM notification sending")
    }

    async fn send_bulk_notification(
        &self,
        _user_ids: &[Uuid],
        _title: &str,
        _message: &str,
    ) -> Result<(), DomainError> {
        // Implementation would use FCM SDK
        todo!("Implement FCM bulk notification sending")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dummy_notification_service() {
        let service = DummyNotificationService::new();
        let user_id = Uuid::new_v4();

        let result = service
            .send_notification(&user_id, "Test Title", "Test Message")
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dummy_bulk_notification_service() {
        let service = DummyNotificationService::new();
        let user_ids = vec![Uuid::new_v4(), Uuid::new_v4()];

        let result = service
            .send_bulk_notification(&user_ids, "Test Title", "Test Message")
            .await;

        assert!(result.is_ok());
    }
} 