use std::sync::Arc;
use crate::port::communication::EmailProvider;
use crate::error::DomainError;
use crate::entity::{communication::EmailCommunication};

pub struct EmailService {
    email_provider: Arc<dyn EmailProvider>,
}

impl EmailService {
    pub fn new(email_provider: Arc<dyn EmailProvider>) -> Self {
        Self { email_provider }
    }

    pub async fn send_email(&self, email: &EmailCommunication) -> Result<String, DomainError> {
        self.email_provider.send_email(email).await
    }
}