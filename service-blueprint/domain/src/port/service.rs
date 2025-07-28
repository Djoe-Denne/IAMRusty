use async_trait::async_trait;
use uuid::Uuid;

use crate::error::DomainError;

/// External notification service port
#[async_trait]
pub trait NotificationService: Send + Sync {
    /// Send a notification to a user
    async fn send_notification(
        &self,
        user_id: &Uuid,
        title: &str,
        message: &str,
    ) -> Result<(), DomainError>;

    /// Send a notification to multiple users
    async fn send_bulk_notification(
        &self,
        user_ids: &[Uuid],
        title: &str,
        message: &str,
    ) -> Result<(), DomainError>;
}

/// External email service port
#[async_trait]
pub trait EmailService: Send + Sync {
    /// Send an email
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        html_body: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Send a templated email
    async fn send_templated_email(
        &self,
        to: &str,
        template_id: &str,
        template_data: &serde_json::Value,
    ) -> Result<(), DomainError>;
}

/// External cache service port
#[async_trait]
pub trait CacheService: Send + Sync {
    /// Get a value from cache
    async fn get(&self, key: &str) -> Result<Option<String>, DomainError>;

    /// Set a value in cache with TTL
    async fn set(&self, key: &str, value: &str, ttl_seconds: u32) -> Result<(), DomainError>;

    /// Delete a value from cache
    async fn delete(&self, key: &str) -> Result<(), DomainError>;

    /// Check if a key exists in cache
    async fn exists(&self, key: &str) -> Result<bool, DomainError>;
}

/// External file storage service port
#[async_trait]
pub trait FileStorageService: Send + Sync {
    /// Upload a file
    async fn upload_file(
        &self,
        key: &str,
        content: &[u8],
        content_type: &str,
    ) -> Result<String, DomainError>;

    /// Download a file
    async fn download_file(&self, key: &str) -> Result<Vec<u8>, DomainError>;

    /// Delete a file
    async fn delete_file(&self, key: &str) -> Result<(), DomainError>;

    /// Generate a presigned URL for file access
    async fn generate_presigned_url(
        &self,
        key: &str,
        expires_in_seconds: u32,
    ) -> Result<String, DomainError>;
} 