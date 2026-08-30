//! Communication port interfaces for Telegraph service

use async_trait::async_trait;

use crate::entity::communication::EmailCommunication;
use crate::error::DomainError;

/// Port for email communication
#[async_trait]
pub trait EmailProvider: Send + Sync {
    /// Send an email message
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] when the provider cannot send the email.
    async fn send_email(&self, email: &EmailCommunication) -> Result<String, DomainError>; // Returns provider message ID

    /// Verify email address format
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] when the address format is invalid.
    fn validate_email(&self, email: &str) -> Result<(), DomainError>;

    /// Check service health
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] when the email service is unreachable or unhealthy.
    async fn health_check(&self) -> Result<(), DomainError>;
}
