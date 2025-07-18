//! Notification event processors for Telegraph communication service

use async_trait::async_trait;
use telegraph_domain::{DomainError, NotificationService, EventHandler};
use iam_events::{IamDomainEvent, UserSignedUpEvent, UserEmailVerifiedEvent, PasswordResetRequestedEvent};
use std::sync::Arc;
use tracing::info;
use crate::repository::combined_notification_repository::CombinedNotificationRepository;
use serde_json::json;

/// Message content for send message command
#[derive(Debug, Clone)]
pub struct  EventContentNotification {
    /// Notification title
    title: String,
    /// Notification body
    body: String,
    /// Notification data payload
    data: HashMap<String, String>,
    /// Notification icon
    icon: Option<String>,
    /// Click action
    click_action: Option<String>,
    
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
impl EventHandler for NotificationEventProcessor {
    async fn handle_event(&self, event: &IamDomainEvent) -> Result<(), DomainError> {
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
    
    fn priority(&self) -> u32 {
        100 // Default priority
    }
}

/// Database notification event processor - creates notification records in database
pub struct DatabaseNotificationProcessor {
    notification_repo: Arc<CombinedNotificationRepository>,
}

impl DatabaseNotificationProcessor {
    /// Create a new database notification event processor
    pub fn new(notification_repo: Arc<CombinedNotificationRepository>) -> Self {
        Self {
            notification_repo,
        }
    }
    
    /// Create notification for email verified event
    async fn create_email_verification_notification(&self, event: &UserEmailVerifiedEvent) -> Result<(), DomainError> {
        info!(
            user_id = %event.user_id,
            email = %event.email,
            "Creating database notification for email verification"
        );
        
        let title = "Email Verified Successfully";
        let content_json = json!({
            "event_type": "user_email_verified",
            "user_id": event.user_id,
            "email": event.email,
            "message": "Your email address has been successfully verified.",
            "action": "email_verification_completed"
        });
        
        let content = content_json.to_string().into_bytes();
        let content_type = "application/json".to_string();
        let priority = 2; // Medium priority for verification events
        let expires_at = None; // No expiration for verification notifications
        
        let notification = self.notification_repo
            .create_notification(
                event.user_id,
                title.to_string(),
                content,
                content_type,
                priority,
                expires_at,
            )
            .await?;
        
        // Create delivery record for in-app notification
        let _delivery = self.notification_repo
            .create_delivery(notification.id, "in_app".to_string())
            .await?;
        
        info!(
            user_id = %event.user_id,
            notification_id = %notification.id,
            "Database notification created successfully for email verification"
        );
        
        Ok(())
    }
    
    /// Create notification for user signup event  
    async fn create_signup_notification(&self, event: &UserSignedUpEvent) -> Result<(), DomainError> {
        info!(
            user_id = %event.user_id,
            email = %event.email,
            username = %event.username,
            "Creating database notification for user signup"
        );
        
        let title = "Welcome to Telegraph!";
        let content_json = json!({
            "event_type": "user_signed_up",
            "user_id": event.user_id,
            "email": event.email,
            "username": event.username,
            "email_verified": event.email_verified,
            "message": format!("Welcome {}! Your account has been created successfully.", event.username),
            "action": "account_created"
        });
        
        let content = content_json.to_string().into_bytes();
        let content_type = "application/json".to_string();
        let priority = 3; // Normal priority for welcome notifications
        let expires_at = None; // No expiration for welcome notifications
        
        let notification = self.notification_repo
            .create_notification(
                event.user_id,
                title.to_string(),
                content,
                content_type,
                priority,
                expires_at,
            )
            .await?;
        
        // Create delivery record for in-app notification
        let _delivery = self.notification_repo
            .create_delivery(notification.id, "in_app".to_string())
            .await?;
        
        info!(
            user_id = %event.user_id,
            notification_id = %notification.id,
            "Database notification created successfully for user signup"
        );
        
        Ok(())
    }
    
    /// Create notification for password reset requested event
    async fn create_password_reset_notification(&self, event: &PasswordResetRequestedEvent) -> Result<(), DomainError> {
        info!(
            user_id = %event.user_id,
            email = %event.email,
            expires_at = %event.expires_at,
            "Creating database notification for password reset request"
        );
        
        let title = "Password Reset Requested";
        let content_json = json!({
            "event_type": "password_reset_requested",
            "user_id": event.user_id,
            "email": event.email,
            "expires_at": event.expires_at,
            "message": "A password reset has been requested for your account.",
            "action": "password_reset_requested"
        });
        
        let content = content_json.to_string().into_bytes();
        let content_type = "application/json".to_string();
        let priority = 1; // High priority for security events
        let expires_at = Some(event.expires_at); // Expire when reset token expires
        
        let notification = self.notification_repo
            .create_notification(
                event.user_id,
                title.to_string(),
                content,
                content_type,
                priority,
                expires_at,
            )
            .await?;
        
        // Create delivery record for in-app notification
        let _delivery = self.notification_repo
            .create_delivery(notification.id, "in_app".to_string())
            .await?;
        
        info!(
            user_id = %event.user_id,
            notification_id = %notification.id,
            "Database notification created successfully for password reset request"
        );
        
        Ok(())
    }
}

#[async_trait]
impl EventHandler for DatabaseNotificationProcessor {
    async fn handle_event(&self, event: &IamDomainEvent) -> Result<(), DomainError> {
        match event {
            IamDomainEvent::UserEmailVerified(verification_event) => {
                self.create_email_verification_notification(verification_event).await
            }
            IamDomainEvent::UserSignedUp(signup_event) => {
                self.create_signup_notification(signup_event).await
            }
            IamDomainEvent::PasswordResetRequested(reset_event) => {
                self.create_password_reset_notification(reset_event).await
            }
            IamDomainEvent::UserLoggedIn(_) => {
                // Login events can create notifications if needed
                // For now, we'll skip them to avoid notification overload
                Ok(())
            }
        }
    }
    
    fn supports_event_type(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            "user_email_verified" | "user_signed_up" | "password_reset_requested"
        )
    }
    
    fn priority(&self) -> u32 {
        100 // Default priority
    }
} 