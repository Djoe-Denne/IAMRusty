use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Represents an email verification token
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmailVerification {
    /// Unique identifier for the verification record
    pub id: Uuid,
    
    /// Email address to verify
    pub email: String,
    
    /// Verification token
    pub verification_token: String,
    
    /// When the token expires
    pub expires_at: DateTime<Utc>,
    
    /// When the verification was created
    pub created_at: DateTime<Utc>,
}

impl EmailVerification {
    /// Creates a new email verification with a token that expires in the specified duration
    pub fn new(
        email: String,
        verification_token: String,
        expires_in_hours: i64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email,
            verification_token,
            expires_at: now + chrono::Duration::hours(expires_in_hours),
            created_at: now,
        }
    }
    
    /// Checks if the verification token has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
    
    /// Checks if this verification token matches the provided token
    pub fn matches_token(&self, token: &str) -> bool {
        self.verification_token == token
    }
} 