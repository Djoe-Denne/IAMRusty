//! SMS event processor for Telegraph communication service

use async_trait::async_trait;
use telegraph_domain::{DomainError, SmsService};
use iam_events::IamDomainEvent;
use std::sync::Arc;
use tracing::info;
use super::processor::CommunicationEventProcessor;

/// SMS communication event processor
pub struct SmsEventProcessor {
    sms_service: Arc<dyn SmsService>,
}

impl SmsEventProcessor {
    /// Create a new SMS event processor
    pub fn new(sms_service: Arc<dyn SmsService>) -> Self {
        Self {
            sms_service,
        }
    }
}

#[async_trait]
impl CommunicationEventProcessor for SmsEventProcessor {
    async fn process_event(&self, event: &IamDomainEvent) -> Result<(), DomainError> {
        match event {
            IamDomainEvent::UserLoggedIn(login_event) => {
                // SMS notifications for security-sensitive events like login
                info!(
                    user_id = %login_event.user_id,
                    email = %login_event.email,
                    "SMS notification for login not implemented - would need phone number"
                );
                
                // Note: SMS requires phone number which isn't in the current IAM events
                // This would need to be enhanced to include phone numbers
                Ok(())
            }
            _ => {
                // Other events don't typically require SMS notifications
                Ok(())
            }
        }
    }
    
    fn supports_event_type(&self, event_type: &str) -> bool {
        matches!(event_type, "user_logged_in")
    }
} 