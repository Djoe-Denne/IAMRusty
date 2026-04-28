use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use rustycog_core::error::ServiceError;
use rustycog_events::event::{BaseEvent, DomainEvent};

/// IAM domain events that can be published to external systems
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum IamDomainEvent {
    /// User signed up with email/password
    #[serde(rename = "user_signed_up")]
    UserSignedUp(UserSignedUpEvent),
    /// User verified their email
    #[serde(rename = "user_email_verified")]
    UserEmailVerified(UserEmailVerifiedEvent),
    /// User logged in successfully
    #[serde(rename = "user_logged_in")]
    UserLoggedIn(UserLoggedInEvent),
    /// User requested password reset
    #[serde(rename = "password_reset_requested")]
    PasswordResetRequested(PasswordResetRequestedEvent),
}

/// Event triggered when a user signs up with email and password
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSignedUpEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    /// User ID
    pub user_id: Uuid,
    /// User's email address
    pub email: String,
    /// Username
    pub username: String,
    /// Whether the email is verified
    pub email_verified: bool,
    /// Email verification token (for unverified emails)
    pub verification_token: Option<String>,
    /// Email verification URL (for unverified emails)
    pub verification_url: Option<String>,
}

/// Event triggered when a user verifies their email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEmailVerifiedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    /// User ID
    pub user_id: Uuid,
    /// Email that was verified
    pub email: String,
}

/// Event triggered when a user logs in successfully
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLoggedInEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    /// User ID
    pub user_id: Uuid,
    /// Email used for login
    pub email: String,
    /// Login method (`email_password`, `oauth_github`, `oauth_gitlab`, etc.)
    pub login_method: String,
}

/// Event triggered when a user requests password reset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetRequestedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    /// User ID
    pub user_id: Uuid,
    /// User's email address
    pub email: String,
    /// Raw reset token (not hashed)
    pub reset_token: String,
    /// When the token expires
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

// Event constructors
impl UserSignedUpEvent {
    /// Create a new `UserSignedUp` event
    #[must_use]
    pub fn new(
        user_id: Uuid,
        email: String,
        username: String,
        email_verified: bool,
        verification_token: Option<String>,
        verification_url: Option<String>,
    ) -> Self {
        Self {
            base: BaseEvent::new("user_signed_up".to_string(), user_id),
            user_id,
            email,
            username,
            email_verified,
            verification_token,
            verification_url,
        }
    }
}

impl UserEmailVerifiedEvent {
    /// Create a new `UserEmailVerified` event
    #[must_use]
    pub fn new(user_id: Uuid, email: String) -> Self {
        Self {
            base: BaseEvent::new("user_email_verified".to_string(), user_id),
            user_id,
            email,
        }
    }
}

impl UserLoggedInEvent {
    /// Create a new `UserLoggedIn` event
    #[must_use]
    pub fn new(user_id: Uuid, email: String, login_method: String) -> Self {
        Self {
            base: BaseEvent::new("user_logged_in".to_string(), user_id),
            user_id,
            email,
            login_method,
        }
    }
}

impl PasswordResetRequestedEvent {
    /// Create a new `PasswordResetRequested` event
    #[must_use]
    pub fn new(
        user_id: Uuid,
        email: String,
        reset_token: String,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("password_reset_requested".to_string(), user_id),
            user_id,
            email,
            reset_token,
            expires_at,
        }
    }
}

// DomainEvent trait implementations
impl DomainEvent for UserSignedUpEvent {
    fn event_type(&self) -> &str {
        &self.base.event_type
    }

    fn event_id(&self) -> Uuid {
        self.base.event_id
    }

    fn aggregate_id(&self) -> Uuid {
        self.base.aggregate_id
    }

    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.base.occurred_at
    }

    fn version(&self) -> u32 {
        self.base.version
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {e}")))
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.base.metadata.clone()
    }
}

impl DomainEvent for UserEmailVerifiedEvent {
    fn event_type(&self) -> &str {
        &self.base.event_type
    }

    fn event_id(&self) -> Uuid {
        self.base.event_id
    }

    fn aggregate_id(&self) -> Uuid {
        self.base.aggregate_id
    }

    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.base.occurred_at
    }

    fn version(&self) -> u32 {
        self.base.version
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {e}")))
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.base.metadata.clone()
    }
}

impl DomainEvent for UserLoggedInEvent {
    fn event_type(&self) -> &str {
        &self.base.event_type
    }

    fn event_id(&self) -> Uuid {
        self.base.event_id
    }

    fn aggregate_id(&self) -> Uuid {
        self.base.aggregate_id
    }

    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.base.occurred_at
    }

    fn version(&self) -> u32 {
        self.base.version
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {e}")))
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.base.metadata.clone()
    }
}

impl DomainEvent for PasswordResetRequestedEvent {
    fn event_type(&self) -> &str {
        &self.base.event_type
    }

    fn event_id(&self) -> Uuid {
        self.base.event_id
    }

    fn aggregate_id(&self) -> Uuid {
        self.base.aggregate_id
    }

    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.base.occurred_at
    }

    fn version(&self) -> u32 {
        self.base.version
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {e}")))
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.base.metadata.clone()
    }
}

impl DomainEvent for IamDomainEvent {
    fn event_type(&self) -> &str {
        match self {
            Self::UserSignedUp(event) => event.event_type(),
            Self::UserEmailVerified(event) => event.event_type(),
            Self::UserLoggedIn(event) => event.event_type(),
            Self::PasswordResetRequested(event) => event.event_type(),
        }
    }

    fn event_id(&self) -> Uuid {
        match self {
            Self::UserSignedUp(event) => event.event_id(),
            Self::UserEmailVerified(event) => event.event_id(),
            Self::UserLoggedIn(event) => event.event_id(),
            Self::PasswordResetRequested(event) => event.event_id(),
        }
    }

    fn aggregate_id(&self) -> Uuid {
        match self {
            Self::UserSignedUp(event) => event.aggregate_id(),
            Self::UserEmailVerified(event) => event.aggregate_id(),
            Self::UserLoggedIn(event) => event.aggregate_id(),
            Self::PasswordResetRequested(event) => event.aggregate_id(),
        }
    }

    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc> {
        match self {
            Self::UserSignedUp(event) => event.occurred_at(),
            Self::UserEmailVerified(event) => event.occurred_at(),
            Self::UserLoggedIn(event) => event.occurred_at(),
            Self::PasswordResetRequested(event) => event.occurred_at(),
        }
    }

    fn version(&self) -> u32 {
        match self {
            Self::UserSignedUp(event) => event.version(),
            Self::UserEmailVerified(event) => event.version(),
            Self::UserLoggedIn(event) => event.version(),
            Self::PasswordResetRequested(event) => event.version(),
        }
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {e}")))
    }

    fn metadata(&self) -> HashMap<String, String> {
        match self {
            Self::UserSignedUp(event) => event.metadata(),
            Self::UserEmailVerified(event) => event.metadata(),
            Self::UserLoggedIn(event) => event.metadata(),
            Self::PasswordResetRequested(event) => event.metadata(),
        }
    }
}

// Convenience methods for IamDomainEvent
impl IamDomainEvent {
    /// Get the user ID associated with this event
    #[must_use]
    pub const fn user_id(&self) -> Uuid {
        match self {
            Self::UserSignedUp(event) => event.user_id,
            Self::UserEmailVerified(event) => event.user_id,
            Self::UserLoggedIn(event) => event.user_id,
            Self::PasswordResetRequested(event) => event.user_id,
        }
    }
}

impl From<IamDomainEvent> for Box<dyn DomainEvent + 'static> {
    fn from(event: IamDomainEvent) -> Self {
        Box::new(event)
    }
}
