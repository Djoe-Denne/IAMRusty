use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog_events::event::BaseEvent;

/// Domain events that can be published to external systems
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DomainEvent {
    /// User signed up with email/password
    UserSignedUp(UserSignedUpEvent),
    /// User verified their email
    UserEmailVerified(UserEmailVerifiedEvent),
    /// User logged in successfully
    UserLoggedIn(UserLoggedInEvent),
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
    /// Login method (email_password, oauth_github, oauth_gitlab, etc.)
    pub login_method: String,
}

impl UserSignedUpEvent {
    /// Create a new UserSignedUp event
    pub fn new(user_id: Uuid, email: String, username: String, email_verified: bool) -> Self {
        Self {
            base: BaseEvent::new("user_signed_up".to_string(), user_id),
            user_id,
            email,
            username,
            email_verified,
        }
    }
}

impl UserEmailVerifiedEvent {
    /// Create a new UserEmailVerified event
    pub fn new(user_id: Uuid, email: String) -> Self {
        Self {
            base: BaseEvent::new("user_email_verified".to_string(), user_id),
            user_id,
            email,
        }
    }
}

impl UserLoggedInEvent {
    /// Create a new UserLoggedIn event
    pub fn new(user_id: Uuid, email: String, login_method: String) -> Self {
        Self {
            base: BaseEvent::new("user_logged_in".to_string(), user_id),
            user_id,
            email,
            login_method,
        }
    }
}

impl DomainEvent {
    /// Get the event ID for tracking
    pub fn event_id(&self) -> Uuid {
        match self {
            DomainEvent::UserSignedUp(event) => event.base.event_id,
            DomainEvent::UserEmailVerified(event) => event.base.event_id,
            DomainEvent::UserLoggedIn(event) => event.base.event_id,
        }
    }

    /// Get the user ID associated with this event
    pub fn user_id(&self) -> Uuid {
        match self {
            DomainEvent::UserSignedUp(event) => event.user_id,
            DomainEvent::UserEmailVerified(event) => event.user_id,
            DomainEvent::UserLoggedIn(event) => event.user_id,
        }
    }

    /// Get the event type as a string for routing
    pub fn event_type(&self) -> &str {
        match self {
            DomainEvent::UserSignedUp(event) => &event.base.event_type,
            DomainEvent::UserEmailVerified(event) => &event.base.event_type,
            DomainEvent::UserLoggedIn(event) => &event.base.event_type,
        }
    }
}
