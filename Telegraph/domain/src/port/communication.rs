//! Communication port interfaces for Telegraph service

use async_trait::async_trait;

use crate::error::DomainError;
use crate::entity::communication::EmailCommunication;

/// Port for email communication
#[async_trait]
pub trait EmailProvider: Send + Sync {
    /// Send an email message
    async fn send_email(
        &self,
        email: &EmailCommunication,
    ) -> Result<String, DomainError>; // Returns provider message ID
    
    /// Verify email address format
    fn validate_email(&self, email: &str) -> Result<(), DomainError>;
    
    /// Check service health
    async fn health_check(&self) -> Result<(), DomainError>;
}
