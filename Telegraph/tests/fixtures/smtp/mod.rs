pub mod resources;
pub mod service;
pub mod testcontainer;

/// Main SMTP fixtures namespace for testing email sending
pub struct SmtpFixtures;

impl SmtpFixtures {
    /// Create a new SMTP service instance
    pub async fn service() -> service::SmtpService {
        service::SmtpService::new().await
    }
} 