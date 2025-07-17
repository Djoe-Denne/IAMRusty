//! Email event processor for Telegraph communication service

use async_trait::async_trait;
use telegraph_domain::{DomainError, EmailService, TemplateService, CommunicationMode};
use iam_events::IamDomainEvent;
use rustycog_events::DomainEvent;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error};
use super::processor::CommunicationEventProcessor;

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
    pub async fn process(&self, event: &IamDomainEvent, template_name: &str) -> Result<(), DomainError> {
        info!(
            event_id = %event.event_id(),
            event_type = event.event_type(),
            user_id = %event.user_id(),
            template = template_name,
            "Processing email event"
        );
        
        // Extract email and prepare variables based on event type
        let (email, variables) = self.extract_event_data(event)?;
        
        // Render the template
        match self.template_service.render_template(template_name, &CommunicationMode::Email, &variables).await {
            Ok(rendered) => {
                match rendered {
                    telegraph_domain::RenderedTemplate::Email { subject, html_body, text_body } => {
                        self.email_service
                            .send_email(&email, &subject, &text_body, html_body.as_deref(), &[])
                            .await?;
                        
                        info!(
                            event_id = %event.event_id(),
                            event_type = event.event_type(),
                            email = %email,
                            template = template_name,
                            "Email sent successfully using template"
                        );
                    }
                    _ => {
                        return Err(DomainError::template_render_error(
                            "Expected email template but got different type".to_string()
                        ));
                    }
                }
            }
            Err(e) => {
                error!(
                    event_id = %event.event_id(),
                    event_type = event.event_type(),
                    email = %email,
                    template = template_name,
                    error = %e,
                    "Failed to render email template, falling back to default content"
                );
                
                // Generate fallback content based on event type
                let (subject, text_body, html_body) = self.generate_fallback_content(event)?;
                
                self.email_service
                    .send_email(&email, &subject, &text_body, Some(&html_body), &[])
                    .await?;
                
                warn!(
                    event_id = %event.event_id(),
                    event_type = event.event_type(),
                    email = %email,
                    template = template_name,
                    "Email sent using fallback content"
                );
            }
        }
        
        Ok(())
    }
    
    /// Extract email address and template variables from an IAM domain event
    fn extract_event_data(&self, event: &IamDomainEvent) -> Result<(String, HashMap<String, String>), DomainError> {
        let mut variables = HashMap::new();
        
        match event {
            IamDomainEvent::UserSignedUp(signup_event) => {
                variables.insert("username".to_string(), signup_event.username.clone());
                variables.insert("email".to_string(), signup_event.email.clone());
                variables.insert("user_id".to_string(), signup_event.user_id.to_string());
                variables.insert("email_verified".to_string(), signup_event.email_verified.to_string());
                variables.insert("subject".to_string(), "Welcome to Telegraph!".to_string());
                
                Ok((signup_event.email.clone(), variables))
            }
            IamDomainEvent::UserEmailVerified(verification_event) => {
                variables.insert("email".to_string(), verification_event.email.clone());
                variables.insert("user_id".to_string(), verification_event.user_id.to_string());
                variables.insert("subject".to_string(), "Email Verified Successfully".to_string());
                
                Ok((verification_event.email.clone(), variables))
            }
            IamDomainEvent::PasswordResetRequested(reset_event) => {
                variables.insert("email".to_string(), reset_event.email.clone());
                variables.insert("user_id".to_string(), reset_event.user_id.to_string());
                variables.insert("reset_token".to_string(), reset_event.reset_token.clone());
                variables.insert("expires_at".to_string(), reset_event.expires_at.format("%Y-%m-%d %H:%M:%S UTC").to_string());
                variables.insert("subject".to_string(), "Password Reset Request".to_string());
                
                Ok((reset_event.email.clone(), variables))
            }
            IamDomainEvent::UserLoggedIn(login_event) => {
                variables.insert("email".to_string(), login_event.email.clone());
                variables.insert("user_id".to_string(), login_event.user_id.to_string());
                variables.insert("login_method".to_string(), login_event.login_method.clone());
                variables.insert("subject".to_string(), "Security Alert: New Login".to_string());
                
                Ok((login_event.email.clone(), variables))
            }
        }
    }
    
    /// Generate fallback email content when template rendering fails
    fn generate_fallback_content(&self, event: &IamDomainEvent) -> Result<(String, String, String), DomainError> {
        match event {
            IamDomainEvent::UserSignedUp(signup_event) => {
                let subject = "Welcome to Telegraph!";
                let text_body = format!(
                    "Hello {}!\n\nWelcome to Telegraph! We're excited to have you on board.\n\nBest regards,\nThe Telegraph Team",
                    signup_event.username
                );
                let html_body = format!(
                    r#"<html><body>
                    <h1>Welcome to Telegraph!</h1>
                    <p>Hello <strong>{}</strong>!</p>
                    <p>Welcome to Telegraph! We're excited to have you on board.</p>
                    <p>Best regards,<br>The Telegraph Team</p>
                    </body></html>"#,
                    signup_event.username
                );
                
                Ok((subject.to_string(), text_body, html_body))
            }
            IamDomainEvent::UserEmailVerified(verification_event) => {
                let subject = "Email Verified Successfully";
                let text_body = format!(
                    "Hello!\n\nYour email address {} has been successfully verified.\n\nBest regards,\nThe Telegraph Team",
                    verification_event.email
                );
                let html_body = format!(
                    r#"<html><body>
                    <h1>Email Verified Successfully</h1>
                    <p>Hello!</p>
                    <p>Your email address <strong>{}</strong> has been successfully verified.</p>
                    <p>Best regards,<br>The Telegraph Team</p>
                    </body></html>"#,
                    verification_event.email
                );
                
                Ok((subject.to_string(), text_body, html_body))
            }
            IamDomainEvent::PasswordResetRequested(reset_event) => {
                let subject = "Password Reset Request";
                let text_body = format!(
                    "Hello!\n\nYou requested a password reset for your Telegraph account.\n\nReset Token: {}\nThis token expires at: {}\n\nIf you didn't request this, please ignore this email.\n\nBest regards,\nThe Telegraph Team",
                    reset_event.reset_token,
                    reset_event.expires_at.format("%Y-%m-%d %H:%M:%S UTC")
                );
                let html_body = format!(
                    r#"<html><body>
                    <h1>Password Reset Request</h1>
                    <p>Hello!</p>
                    <p>You requested a password reset for your Telegraph account.</p>
                    <p><strong>Reset Token:</strong> <code>{}</code></p>
                    <p><strong>Expires at:</strong> {}</p>
                    <p>If you didn't request this, please ignore this email.</p>
                    <p>Best regards,<br>The Telegraph Team</p>
                    </body></html>"#,
                    reset_event.reset_token,
                    reset_event.expires_at.format("%Y-%m-%d %H:%M:%S UTC")
                );
                
                Ok((subject.to_string(), text_body, html_body))
            }
            IamDomainEvent::UserLoggedIn(login_event) => {
                let subject = "Security Alert: New Login";
                let text_body = format!(
                    "Hello!\n\nWe detected a new login to your Telegraph account.\n\nLogin method: {}\n\nIf this was not you, please secure your account immediately.\n\nBest regards,\nThe Telegraph Team",
                    login_event.login_method
                );
                let html_body = format!(
                    r#"<html><body>
                    <h1>Security Alert: New Login</h1>
                    <p>Hello!</p>
                    <p>We detected a new login to your Telegraph account.</p>
                    <p><strong>Login method:</strong> {}</p>
                    <p>If this was not you, please secure your account immediately.</p>
                    <p>Best regards,<br>The Telegraph Team</p>
                    </body></html>"#,
                    login_event.login_method
                );
                
                Ok((subject.to_string(), text_body, html_body))
            }
        }
    }
}

#[async_trait]
impl CommunicationEventProcessor for EmailEventProcessor {
    async fn process_event(&self, event: &IamDomainEvent) -> Result<(), DomainError> {
        // For now, use a default template naming convention
        // In a complete implementation, this would come from configuration
        // The template name should be provided by the caller who has access to configuration
        let template_name = format!("{}_email", event.event_type());
        
        self.process(event, &template_name).await
    }
    
    fn supports_event_type(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            "user_signed_up" | "user_email_verified" | "password_reset_requested" | "user_logged_in"
        )
    }
} 