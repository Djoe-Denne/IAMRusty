use crate::entity::communication::EmailCommunication;
use crate::error::DomainError;
use crate::port::communication::EmailProvider;
use std::sync::Arc;

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
