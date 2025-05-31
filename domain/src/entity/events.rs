use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use rustycog_events::event::{DomainEvent as DomainEventTrait, BaseEvent};
use rustycog_core::error::ServiceError;

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
    /// Base event information
    #[serde(flatten)]
    pub base: BaseEvent,
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
    /// Base event information
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Email that was verified
    pub email: String,
}

/// Event triggered when a user logs in successfully
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLoggedInEvent {
    /// Base event information
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Email used for login
    pub email: String,
    /// Login method (email_password, oauth_github, oauth_gitlab, etc.)
    pub login_method: String,
}

impl UserSignedUpEvent {
    /// Create a new UserSignedUp event
    pub fn new(user_id: Uuid, email: String, username: String, email_verified: bool) -> Self {
        let mut base = BaseEvent::new("user_signed_up".to_string(), user_id);
        base = base.with_metadata("email".to_string(), email.clone());
        base = base.with_metadata("username".to_string(), username.clone());
        base = base.with_metadata("email_verified".to_string(), email_verified.to_string());
        
        Self {
            base,
            email,
            username,
            email_verified,
        }
    }
}

impl UserEmailVerifiedEvent {
    /// Create a new UserEmailVerified event
    pub fn new(user_id: Uuid, email: String) -> Self {
        let mut base = BaseEvent::new("user_email_verified".to_string(), user_id);
        base = base.with_metadata("email".to_string(), email.clone());
        
        Self {
            base,
            email,
        }
    }
}

impl UserLoggedInEvent {
    /// Create a new UserLoggedIn event
    pub fn new(user_id: Uuid, email: String, login_method: String) -> Self {
        let mut base = BaseEvent::new("user_logged_in".to_string(), user_id);
        base = base.with_metadata("email".to_string(), email.clone());
        base = base.with_metadata("login_method".to_string(), login_method.clone());
        
        Self {
            base,
            email,
            login_method,
        }
    }
}

// Implement DomainEventTrait for UserSignedUpEvent
impl DomainEventTrait for UserSignedUpEvent {
    fn event_type(&self) -> &'static str {
        "user_signed_up"
    }
    
    fn event_id(&self) -> Uuid {
        self.base.event_id
    }
    
    fn aggregate_id(&self) -> Uuid {
        self.base.aggregate_id
    }
    
    fn occurred_at(&self) -> DateTime<Utc> {
        self.base.occurred_at
    }
    
    fn version(&self) -> u32 {
        self.base.version
    }
    
    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {}", e)))
    }
    
    fn metadata(&self) -> HashMap<String, String> {
        self.base.metadata.clone()
    }
}

// Implement DomainEventTrait for UserEmailVerifiedEvent
impl DomainEventTrait for UserEmailVerifiedEvent {
    fn event_type(&self) -> &'static str {
        "user_email_verified"
    }
    
    fn event_id(&self) -> Uuid {
        self.base.event_id
    }
    
    fn aggregate_id(&self) -> Uuid {
        self.base.aggregate_id
    }
    
    fn occurred_at(&self) -> DateTime<Utc> {
        self.base.occurred_at
    }
    
    fn version(&self) -> u32 {
        self.base.version
    }
    
    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {}", e)))
    }
    
    fn metadata(&self) -> HashMap<String, String> {
        self.base.metadata.clone()
    }
}

// Implement DomainEventTrait for UserLoggedInEvent
impl DomainEventTrait for UserLoggedInEvent {
    fn event_type(&self) -> &'static str {
        "user_logged_in"
    }
    
    fn event_id(&self) -> Uuid {
        self.base.event_id
    }
    
    fn aggregate_id(&self) -> Uuid {
        self.base.aggregate_id
    }
    
    fn occurred_at(&self) -> DateTime<Utc> {
        self.base.occurred_at
    }
    
    fn version(&self) -> u32 {
        self.base.version
    }
    
    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {}", e)))
    }
    
    fn metadata(&self) -> HashMap<String, String> {
        self.base.metadata.clone()
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
            DomainEvent::UserSignedUp(event) => event.base.aggregate_id,
            DomainEvent::UserEmailVerified(event) => event.base.aggregate_id,
            DomainEvent::UserLoggedIn(event) => event.base.aggregate_id,
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