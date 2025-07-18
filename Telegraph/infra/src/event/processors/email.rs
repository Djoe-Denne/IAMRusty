//! Email event processor for Telegraph communication service

use async_trait::async_trait;
use telegraph_domain::{DomainError, EmailService, TemplateService, CommunicationMode, EventHandler, EventContext, RenderedTemplate};
use iam_events::IamDomainEvent;
use rustycog_events::DomainEvent;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error};
use serde_json::{self, Value};
use crate::event::json_utils::json_to_string_map;

/// Email communication event processor
pub struct EmailEventProcessor {
    email_service: Arc<dyn EmailService>,
    template_service: Arc<dyn TemplateService>,
}

impl EmailEventProcessor {
    /// Create a new email event processor
    pub fn new(email_service: Arc<dyn EmailService>, template_service: Arc<dyn TemplateService>) -> Self {
        Self {
            email_service,
            template_service,
        }
    }
    
    /// Process an IAM domain event with the specified template
    pub async fn process(&self, event: &EventContext) -> Result<(), DomainError> {
        info!(
            event_id = %event.event_id,
            event_type = event.event_type.to_string(),
            user_id = %event.recipient.user_id.unwrap_or_default(),
            "Processing email event"
        );

        let email = event.recipient.email.as_ref().ok_or(DomainError::EventProcessingError("No email address found in event".to_string()))?;

        // Serialize the IAM domain event to JSON
        let event_json = serde_json::from_str(&event.event.to_json()
        .map_err(|e| DomainError::EventProcessingError(format!("Failed to serialize event to JSON: {}", e)))?)
        .map_err(|e| DomainError::EventProcessingError(format!("Failed to serialize event to JSON: {}", e)))?;

        // Convert JSON to HashMap<String, String> for template variables
        let variables = json_to_string_map(&event_json)?;

        // Find the appropriate template name using the template service
        let template_name = self.template_service
            .find_template(&event.event_type, &CommunicationMode::Email)
            .await?;

        // Render the template using the template service
        let rendered_template = self.template_service
            .render_template(&template_name, &CommunicationMode::Email, &variables)
            .await?;

        // Extract email content from rendered template
        let (subject, html_body, text_body) = match rendered_template {
            RenderedTemplate::Email { subject, html_body, text_body } => {
                (subject, html_body, text_body)
            }
            _ => return Err(DomainError::EventProcessingError("Template did not render to email content".to_string())),
        };

        // Send the email using the rendered content
        self.email_service
            .send_email(&email, &subject, &text_body, html_body.as_deref(), &[])
            .await?;

        info!(
            event_id = %event.event_id,
            event_type = event.event_type.to_string(),
            email = %email,
            template = %template_name,
            "Email sent successfully using template"
        );

        Ok(())
    }
}

#[async_trait]
impl EventHandler for EmailEventProcessor {
    async fn handle_event(&self, event: &EventContext) -> Result<(), DomainError> {
        self.process(event).await
    }
    
    fn supports_event_type(&self, event_type: &str) -> bool {
        // TODO: check config to know supported event types
        matches!(
            event_type,
            "user_signed_up" | "user_email_verified" | "password_reset_requested" | "user_logged_in"
        )
    }
    
    fn priority(&self) -> u32 {
        100 // Default priority
    }
} 