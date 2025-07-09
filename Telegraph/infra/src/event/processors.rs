//! Event processors for Telegraph communication service

use async_trait::async_trait;
use domain::{DomainError, EmailService, SmsService, NotificationService};
use iam_events::{IamDomainEvent, UserSignedUpEvent, UserEmailVerifiedEvent, PasswordResetRequestedEvent};
use rustycog_events::DomainEvent;
use std::sync::Arc;
use tracing::{info, error, warn};

/// Communication event processor trait
#[async_trait]
pub trait CommunicationEventProcessor: Send + Sync {
    /// Process an IAM domain event
    async fn process_event(&self, event: &IamDomainEvent) -> Result<(), DomainError>;
    
    /// Check if this processor supports the given event type
    fn supports_event_type(&self, event_type: &str) -> bool;
}

/// Email communication event processor
pub struct EmailEventProcessor {
    email_service: Arc<dyn EmailService>,
}

impl EmailEventProcessor {
    /// Create a new email event processor
    pub fn new(email_service: Arc<dyn EmailService>) -> Self {
        Self {
            email_service,
        }
    }
    
    /// Send welcome email for new user signup
    async fn send_welcome_email(&self, event: &UserSignedUpEvent) -> Result<(), DomainError> {
        info!(
            user_id = %event.user_id,
            email = %event.email,
            username = %event.username,
            "Sending welcome email for new user signup"
        );
        
        let subject = "Welcome to Telegraph!";
        let text_body = format!(
            "Hello {}!\n\nWelcome to Telegraph! We're excited to have you on board.\n\nBest regards,\nThe Telegraph Team",
            event.username
        );
        let html_body = format!(
            r#"<html><body>
            <h1>Welcome to Telegraph!</h1>
            <p>Hello <strong>{}</strong>!</p>
            <p>Welcome to Telegraph! We're excited to have you on board.</p>
            <p>Best regards,<br>The Telegraph Team</p>
            </body></html>"#,
            event.username
        );
        
        self.email_service
            .send_email(&event.email, subject, &text_body, Some(&html_body), &[])
            .await?;
        
        info!(
            user_id = %event.user_id,
            email = %event.email,
            "Welcome email sent successfully"
        );
        
        Ok(())
    }
    
    /// Send email verification confirmation
    async fn send_verification_confirmation(&self, event: &UserEmailVerifiedEvent) -> Result<(), DomainError> {
        info!(
            user_id = %event.user_id,
            email = %event.email,
            "Sending email verification confirmation"
        );
        
        let subject = "Email Verified Successfully";
        let text_body = format!(
            "Hello!\n\nYour email address {} has been successfully verified.\n\nBest regards,\nThe Telegraph Team",
            event.email
        );
        let html_body = format!(
            r#"<html><body>
            <h1>Email Verified Successfully</h1>
            <p>Hello!</p>
            <p>Your email address <strong>{}</strong> has been successfully verified.</p>
            <p>Best regards,<br>The Telegraph Team</p>
            </body></html>"#,
            event.email
        );
        
        self.email_service
            .send_email(&event.email, subject, &text_body, Some(&html_body), &[])
            .await?;
        
        info!(
            user_id = %event.user_id,
            email = %event.email,
            "Email verification confirmation sent successfully"
        );
        
        Ok(())
    }
    
    /// Send password reset email
    async fn send_password_reset_email(&self, event: &PasswordResetRequestedEvent) -> Result<(), DomainError> {
        info!(
            user_id = %event.user_id,
            email = %event.email,
            expires_at = %event.expires_at,
            "Sending password reset email"
        );
        
        let subject = "Password Reset Request";
        let text_body = format!(
            "Hello!\n\nYou requested a password reset for your Telegraph account.\n\nReset Token: {}\nThis token expires at: {}\n\nIf you didn't request this, please ignore this email.\n\nBest regards,\nThe Telegraph Team",
            event.reset_token,
            event.expires_at.format("%Y-%m-%d %H:%M:%S UTC")
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
            event.reset_token,
            event.expires_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
        
        self.email_service
            .send_email(&event.email, subject, &text_body, Some(&html_body), &[])
            .await?;
        
        info!(
            user_id = %event.user_id,
            email = %event.email,
            "Password reset email sent successfully"
        );
        
        Ok(())
    }
}

#[async_trait]
impl CommunicationEventProcessor for EmailEventProcessor {
    async fn process_event(&self, event: &IamDomainEvent) -> Result<(), DomainError> {
        match event {
            IamDomainEvent::UserSignedUp(signup_event) => {
                self.send_welcome_email(signup_event).await
            }
            IamDomainEvent::UserEmailVerified(verification_event) => {
                self.send_verification_confirmation(verification_event).await
            }
            IamDomainEvent::PasswordResetRequested(reset_event) => {
                self.send_password_reset_email(reset_event).await
            }
            IamDomainEvent::UserLoggedIn(_) => {
                // Email notifications for login events are typically optional
                // and can be enabled via configuration if needed
                Ok(())
            }
        }
    }
    
    fn supports_event_type(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            "user_signed_up" | "user_email_verified" | "password_reset_requested"
        )
    }
}

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

/// Push notification event processor
pub struct NotificationEventProcessor {
    notification_service: Arc<dyn NotificationService>,
}

impl NotificationEventProcessor {
    /// Create a new notification event processor
    pub fn new(notification_service: Arc<dyn NotificationService>) -> Self {
        Self {
            notification_service,
        }
    }
}

#[async_trait]
impl CommunicationEventProcessor for NotificationEventProcessor {
    async fn process_event(&self, event: &IamDomainEvent) -> Result<(), DomainError> {
        match event {
            IamDomainEvent::UserSignedUp(signup_event) => {
                // Push notifications for welcome/signup events
                info!(
                    user_id = %signup_event.user_id,
                    email = %signup_event.email,
                    username = %signup_event.username,
                    "Push notification for signup not implemented - would need device token"
                );
                
                // Note: Push notifications require device token which isn't in the current IAM events
                // This would need to be enhanced to include device tokens
                Ok(())
            }
            IamDomainEvent::UserLoggedIn(login_event) => {
                // Push notifications for security events
                info!(
                    user_id = %login_event.user_id,
                    email = %login_event.email,
                    login_method = %login_event.login_method,
                    "Push notification for login not implemented - would need device token"
                );
                
                // Note: Push notifications require device token which isn't in the current IAM events
                // This would need to be enhanced to include device tokens
                Ok(())
            }
            _ => {
                // Other events don't typically require push notifications
                Ok(())
            }
        }
    }
    
    fn supports_event_type(&self, event_type: &str) -> bool {
        matches!(event_type, "user_signed_up" | "user_logged_in")
    }
}

/// Composite event processor that routes events to multiple processors
pub struct CompositeEventProcessor {
    processors: Vec<Arc<dyn CommunicationEventProcessor>>,
}

impl CompositeEventProcessor {
    /// Create a new composite event processor
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }
    
    /// Add a processor to the composite
    pub fn add_processor(mut self, processor: Arc<dyn CommunicationEventProcessor>) -> Self {
        self.processors.push(processor);
        self
    }
    
    /// Create a composite processor with all communication types
    pub fn with_all_processors(
        email_service: Arc<dyn EmailService>,
        sms_service: Arc<dyn SmsService>,
        notification_service: Arc<dyn NotificationService>,
    ) -> Self {
        Self::new()
            .add_processor(Arc::new(EmailEventProcessor::new(email_service)))
            .add_processor(Arc::new(SmsEventProcessor::new(sms_service)))
            .add_processor(Arc::new(NotificationEventProcessor::new(notification_service)))
    }
}

#[async_trait]
impl CommunicationEventProcessor for CompositeEventProcessor {
    async fn process_event(&self, event: &IamDomainEvent) -> Result<(), DomainError> {
        let mut errors = Vec::new();
        let mut processed_count = 0;
        
        for processor in &self.processors {
            if processor.supports_event_type(event.event_type()) {
                match processor.process_event(event).await {
                    Ok(()) => {
                        processed_count += 1;
                    }
                    Err(e) => {
                        error!(
                            event_type = event.event_type(),
                            event_id = %event.event_id(),
                            error = %e,
                            "Processor failed to handle event"
                        );
                        errors.push(e);
                    }
                }
            }
        }
        
        if !errors.is_empty() {
            warn!(
                event_type = event.event_type(),
                event_id = %event.event_id(),
                errors_count = errors.len(),
                processed_count = processed_count,
                "Some processors failed to handle event"
            );
            
            // Return the first error, but log all of them
            return Err(errors.into_iter().next().unwrap());
        }
        
        if processed_count == 0 {
            warn!(
                event_type = event.event_type(),
                event_id = %event.event_id(),
                "No processors handled this event type"
            );
        } else {
            info!(
                event_type = event.event_type(),
                event_id = %event.event_id(),
                processed_count = processed_count,
                "Event processed successfully by all applicable processors"
            );
        }
        
        Ok(())
    }
    
    fn supports_event_type(&self, event_type: &str) -> bool {
        self.processors.iter().any(|p| p.supports_event_type(event_type))
    }
} 