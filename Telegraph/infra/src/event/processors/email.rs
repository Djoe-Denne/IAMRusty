//! Email event processor for Telegraph communication service

use async_trait::async_trait;
use std::sync::Arc;
use telegraph_domain::service::EmailService;
use telegraph_domain::{CommunicationFactory, DomainError, EventContext, EventHandler};
use tracing::info;

/// Email communication event processor
pub struct EmailEventProcessor {
    email_service: Arc<EmailService>,
    communication_factory: Arc<CommunicationFactory>,
}

impl EmailEventProcessor {
    /// Create a new email event processor
    pub fn new(
        email_service: Arc<EmailService>,
        communication_factory: Arc<CommunicationFactory>,
    ) -> Self {
        Self {
            email_service,
            communication_factory,
        }
    }

    /// Process an IAM domain event with the communication factory
    pub async fn process(&self, event: &EventContext) -> Result<(), DomainError> {
        info!(
            event_id = %event.event_id,
            event_type = event.event_type.to_string(),
            user_id = %event.recipient.user_id.unwrap_or_default(),
            "Processing email event"
        );

        // Build email communication using the factory
        let email_communication = self
            .communication_factory
            .build_email_communication(event)
            .await?;

        // Validate recipient has email
        let email = email_communication.recipient.email.as_ref().ok_or(
            DomainError::EventProcessingError(
                "No email address found in communication".to_string(),
            ),
        )?;

        // Send the email using the communication content
        self.email_service.send_email(&email_communication).await?;

        info!(
            event_id = %event.event_id,
            event_type = event.event_type.to_string(),
            email = %email,
            subject = %email_communication.subject,
            "Email sent successfully using communication factory"
        );

        Ok(())
    }
}

#[async_trait]
impl EventHandler for EmailEventProcessor {
    async fn handle_event(&self, event: &EventContext) -> Result<(), DomainError> {
        self.process(event).await
    }

    fn priority(&self) -> u32 {
        100 // Default priority
    }
}
