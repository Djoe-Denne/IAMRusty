//! Communication factory service for building communications from event descriptors

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::{
    Communication, CommunicationDescriptor, CommunicationMode, CommunicationRecipient, DomainError,
    EmailCommunication, EventContext, EventExtractor, NotificationCommunication, RenderedTemplate,
    TemplateService,
};

/// Communication factory service that builds communications from TOML descriptors
pub struct CommunicationFactory {
    template_service: Arc<dyn TemplateService>,
    event_extractor: Arc<dyn EventExtractor>,
    descriptor_dir: PathBuf,
}

impl CommunicationFactory {
    /// Create a new communication factory
    pub fn new(
        template_service: Arc<dyn TemplateService>,
        event_extractor: Arc<dyn EventExtractor>,
        descriptor_dir: PathBuf,
    ) -> Self {
        Self {
            template_service,
            event_extractor,
            descriptor_dir,
        }
    }

    /// Build email communication from event context
    pub async fn build_email_communication(
        &self,
        event: &EventContext,
    ) -> Result<EmailCommunication, DomainError> {
        // Load communication descriptor for the event type
        let descriptor = self.load_descriptor(&event.event_type).await?;

        let email_desc = descriptor.email.ok_or_else(|| {
            DomainError::EventProcessingError(format!(
                "No email configuration found for event type '{}'",
                event.event_type
            ))
        })?;

        // Extract variables from the event
        let variables = self
            .event_extractor
            .extract_variables(event.event.as_ref())
            .await?;

        // Render the template using the template service
        let rendered_template = self
            .template_service
            .render_template(&email_desc.template, &CommunicationMode::Email, &variables)
            .await?;

        // Extract email content from rendered template
        let (subject, html_body, text_body) = match rendered_template {
            RenderedTemplate::Email {
                subject,
                html_body,
                text_body,
            } => {
                // Use subject from descriptor or template variables, fallback to rendered subject
                let final_subject = if !email_desc.subject.is_empty() {
                    self.interpolate_string(&email_desc.subject, &variables)
                } else {
                    subject
                };
                (final_subject, html_body, text_body)
            }
            _ => {
                return Err(DomainError::EventProcessingError(
                    "Template did not render to email content".to_string(),
                ))
            }
        };

        // Build recipient from event context
        let recipient = CommunicationRecipient {
            user_id: event.recipient.user_id,
            email: event.recipient.email.clone(),
        };

        Ok(EmailCommunication {
            recipient,
            subject,
            text_body,
            html_body,
        })
    }

    /// Build notification communication from event context
    pub async fn build_notification_communication(
        &self,
        event: &EventContext,
    ) -> Result<NotificationCommunication, DomainError> {
        // Load communication descriptor for the event type
        let descriptor = self.load_descriptor(&event.event_type).await?;

        let notification_desc = descriptor.notification.ok_or_else(|| {
            DomainError::EventProcessingError(format!(
                "No notification configuration found for event type '{}'",
                event.event_type
            ))
        })?;

        // Extract variables from the event
        let variables = self
            .event_extractor
            .extract_variables(event.event.as_ref())
            .await?;

        // Render the template using the template service
        let rendered_template = self
            .template_service
            .render_template(
                &notification_desc.template,
                &CommunicationMode::Notification,
                &variables,
            )
            .await?;

        // Extract notification content from rendered template
        let (title, body, data) = match rendered_template {
            RenderedTemplate::Notification { title, body, data } => {
                // Use title from descriptor or template variables, fallback to rendered title
                let final_title = if !notification_desc.title.is_empty() {
                    self.interpolate_string(&notification_desc.title, &variables)
                } else {
                    title
                };
                (final_title, body, data)
            }
            _ => {
                return Err(DomainError::EventProcessingError(
                    "Template did not render to notification content".to_string(),
                ))
            }
        };

        // Build recipient from event context
        let recipient = CommunicationRecipient {
            user_id: event.recipient.user_id,
            email: event.recipient.email.clone(),
        };

        Ok(NotificationCommunication {
            id: None,
            recipient,
            title,
            body,
            data,
            is_read: None,
            created_at: None,
            updated_at: None,
            read_at: None,
        })
    }

    /// Build any communication based on the available descriptors
    pub async fn build_communication(
        &self,
        event: &EventContext,
    ) -> Result<Vec<Communication>, DomainError> {
        let descriptor = self.load_descriptor(&event.event_type).await?;
        let mut communications = Vec::new();

        // Build email communication if configured
        if descriptor.email.is_some() {
            match self.build_email_communication(event).await {
                Ok(email_comm) => communications.push(Communication::Email(email_comm)),
                Err(e) => {
                    warn!(
                        event_type = %event.event_type,
                        error = %e,
                        "Failed to build email communication"
                    );
                }
            }
        }

        // Build notification communication if configured
        if descriptor.notification.is_some() {
            match self.build_notification_communication(event).await {
                Ok(notification_comm) => {
                    communications.push(Communication::Notification(notification_comm))
                }
                Err(e) => {
                    warn!(
                        event_type = %event.event_type,
                        error = %e,
                        "Failed to build notification communication"
                    );
                }
            }
        }

        if communications.is_empty() {
            return Err(DomainError::EventProcessingError(format!(
                "No valid communication configurations found for event type '{}'",
                event.event_type
            )));
        }

        debug!(
            event_type = %event.event_type,
            communication_count = communications.len(),
            "Built communications for event"
        );

        Ok(communications)
    }

    /// Load communication descriptor from TOML file
    async fn load_descriptor(
        &self,
        event_type: &str,
    ) -> Result<CommunicationDescriptor, DomainError> {
        let descriptor_path = self.descriptor_dir.join(format!("{}.toml", event_type));

        if !descriptor_path.exists() {
            return Err(DomainError::EventProcessingError(format!(
                "Communication descriptor not found for event type '{}' at path '{}'",
                event_type,
                descriptor_path.display()
            )));
        }

        let content = tokio::fs::read_to_string(&descriptor_path)
            .await
            .map_err(|e| {
                DomainError::EventProcessingError(format!(
                    "Failed to read descriptor file '{}': {}",
                    descriptor_path.display(),
                    e
                ))
            })?;

        let descriptor: CommunicationDescriptor = toml::from_str(&content).map_err(|e| {
            DomainError::EventProcessingError(format!(
                "Failed to parse descriptor file '{}': {}",
                descriptor_path.display(),
                e
            ))
        })?;

        debug!(
            event_type = %event_type,
            descriptor_path = %descriptor_path.display(),
            "Loaded communication descriptor"
        );

        Ok(descriptor)
    }

    /// Simple string interpolation for descriptor fields
    fn interpolate_string(
        &self,
        template_str: &str,
        variables: &HashMap<String, String>,
    ) -> String {
        let mut result = template_str.to_string();
        for (key, value) in variables {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}
