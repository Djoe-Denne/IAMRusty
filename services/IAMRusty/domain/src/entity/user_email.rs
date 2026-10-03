use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents an email address associated with a user
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserEmail {
    /// Unique identifier for this email record
    pub id: Uuid,

    /// The user this email belongs to
    pub user_id: Uuid,

    /// The email address
    pub email: String,

    /// Whether this is the user's primary email
    pub is_primary: bool,

    /// Whether this email has been verified
    pub is_verified: bool,

    /// When this email was added
    pub created_at: DateTime<Utc>,

    /// When this email was last updated
    pub updated_at: DateTime<Utc>,
}

impl UserEmail {
    /// Creates a new user email
    #[must_use]
    pub fn new(user_id: Uuid, email: String, is_primary: bool, is_verified: bool) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            email,
            is_primary,
            is_verified,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new primary email (usually for new users)
    #[must_use]
    pub fn new_primary(user_id: Uuid, email: String, is_verified: bool) -> Self {
        Self::new(user_id, email, true, is_verified)
    }

    /// Creates a new secondary email
    #[must_use]
    pub fn new_secondary(user_id: Uuid, email: String, is_verified: bool) -> Self {
        Self::new(user_id, email, false, is_verified)
    }

    /// Marks this email as verified
    pub fn verify(&mut self) {
        self.is_verified = true;
        self.updated_at = Utc::now();
    }

    /// Sets this email as primary (note: application logic should ensure only one primary per user)
    pub fn set_as_primary(&mut self) {
        self.is_primary = true;
        self.updated_at = Utc::now();
    }

    /// Sets this email as secondary
    pub fn set_as_secondary(&mut self) {
        self.is_primary = false;
        self.updated_at = Utc::now();
    }
}
