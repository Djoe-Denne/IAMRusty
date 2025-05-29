use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Domain events that can be published to external systems
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
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
    /// Event ID for idempotency
    pub event_id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// User's email address
    pub email: String,
    /// Username
    pub username: String,
    /// When the event occurred
    pub occurred_at: DateTime<Utc>,
    /// Whether the email is verified
    pub email_verified: bool,
}

/// Event triggered when a user verifies their email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEmailVerifiedEvent {
    /// Event ID for idempotency
    pub event_id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// Email that was verified
    pub email: String,
    /// When the event occurred
    pub occurred_at: DateTime<Utc>,
}

/// Event triggered when a user logs in successfully
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLoggedInEvent {
    /// Event ID for idempotency
    pub event_id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// Email used for login
    pub email: String,
    /// When the event occurred
    pub occurred_at: DateTime<Utc>,
    /// Login method (email_password, oauth_github, oauth_gitlab, etc.)
    pub login_method: String,
}

impl UserSignedUpEvent {
    /// Create a new UserSignedUp event
    pub fn new(user_id: Uuid, email: String, username: String, email_verified: bool) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            user_id,
            email,
            username,
            occurred_at: Utc::now(),
            email_verified,
        }
    }
}

impl UserEmailVerifiedEvent {
    /// Create a new UserEmailVerified event
    pub fn new(user_id: Uuid, email: String) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            user_id,
            email,
            occurred_at: Utc::now(),
        }
    }
}

impl UserLoggedInEvent {
    /// Create a new UserLoggedIn event
    pub fn new(user_id: Uuid, email: String, login_method: String) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            user_id,
            email,
            occurred_at: Utc::now(),
            login_method,
        }
    }
}

impl DomainEvent {
    /// Get the event ID for tracking
    pub fn event_id(&self) -> Uuid {
        match self {
            DomainEvent::UserSignedUp(event) => event.event_id,
            DomainEvent::UserEmailVerified(event) => event.event_id,
            DomainEvent::UserLoggedIn(event) => event.event_id,
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
    pub fn event_type(&self) -> &'static str {
        match self {
            DomainEvent::UserSignedUp(_) => "user_signed_up",
            DomainEvent::UserEmailVerified(_) => "user_email_verified",
            DomainEvent::UserLoggedIn(_) => "user_logged_in",
        }
    }
} 