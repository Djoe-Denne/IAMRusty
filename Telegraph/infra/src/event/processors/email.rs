//! Email event processor for Telegraph communication service

use async_trait::async_trait;
use telegraph_domain::{DomainError, EmailService};
use iam_events::{IamDomainEvent, UserSignedUpEvent, UserEmailVerifiedEvent, PasswordResetRequestedEvent};
use std::sync::Arc;
use tracing::info;
use super::processor::CommunicationEventProcessor;

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