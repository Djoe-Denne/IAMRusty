use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;

/// Organization invitation entity for inviting users to join an organization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationInvitation {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub role_id: Uuid,
    pub invited_by_user_id: Uuid,
    pub token: String,
    pub status: InvitationStatus,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Invitation status enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Expired,
    Cancelled,
}

impl OrganizationInvitation {
    /// Default invitation expiry duration (7 days)
    pub const DEFAULT_EXPIRY_DAYS: i64 = 7;

    /// Create a new organization invitation
    pub fn new(
        organization_id: Uuid,
        email: String,
        role_id: Uuid,
        invited_by_user_id: Uuid,
        message: Option<String>,
    ) -> Result<Self, DomainError> {
        Self::validate_email(&email)?;
        
        let now = Utc::now();
        let expires_at = now + Duration::days(Self::DEFAULT_EXPIRY_DAYS);
        let token = Self::generate_token();

        Ok(Self {
            id: Uuid::new_v4(),
            organization_id,
            email: email.to_lowercase(),
            role_id,
            invited_by_user_id,
            token,
            status: InvitationStatus::Pending,
            expires_at,
            accepted_at: None,
            message,
            created_at: now,
        })
    }

    /// Create invitation with custom expiry
    pub fn new_with_expiry(
        organization_id: Uuid,
        email: String,
        role_id: Uuid,
        invited_by_user_id: Uuid,
        expires_at: DateTime<Utc>,
        message: Option<String>,
    ) -> Result<Self, DomainError> {
        Self::validate_email(&email)?;
        
        if expires_at <= Utc::now() {
            return Err(DomainError::invalid_input(
                "Expiry date must be in the future"
            ));
        }

        let token = Self::generate_token();

        Ok(Self {
            id: Uuid::new_v4(),
            organization_id,
            email: email.to_lowercase(),
            role_id,
            invited_by_user_id,
            token,
            status: InvitationStatus::Pending,
            expires_at,
            accepted_at: None,
            message,
            created_at: Utc::now(),
        })
    }

    /// Accept the invitation
    pub fn accept(&mut self) -> Result<(), DomainError> {
        match self.status {
            InvitationStatus::Pending => {
                if self.is_expired() {
                    self.status = InvitationStatus::Expired;
                    return Err(DomainError::business_rule_violation(
                        "Cannot accept expired invitation",
                    ));
                }
                
                self.status = InvitationStatus::Accepted;
                self.accepted_at = Some(Utc::now());
                Ok(())
            }
            InvitationStatus::Accepted => {
                Err(DomainError::business_rule_violation(
                    "Invitation has already been accepted",
                ))
            }
            InvitationStatus::Expired => {
                Err(DomainError::business_rule_violation(
                    "Cannot accept expired invitation",
                ))
            }
            InvitationStatus::Cancelled => {
                Err(DomainError::business_rule_violation(
                    "Cannot accept cancelled invitation",
                ))
            }
        }
    }

    /// Cancel the invitation
    pub fn cancel(&mut self) -> Result<(), DomainError> {
        match self.status {
            InvitationStatus::Pending => {
                self.status = InvitationStatus::Cancelled;
                Ok(())
            }
            InvitationStatus::Accepted => {
                Err(DomainError::business_rule_violation(
                    "Cannot cancel accepted invitation",
                ))
            }
            InvitationStatus::Expired => {
                Err(DomainError::business_rule_violation(
                    "Cannot cancel expired invitation",
                ))
            }
            InvitationStatus::Cancelled => {
                Err(DomainError::business_rule_violation(
                    "Invitation is already cancelled",
                ))
            }
        }
    }

    /// Check if invitation is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if invitation is pending and not expired
    pub fn is_valid(&self) -> bool {
        matches!(self.status, InvitationStatus::Pending) && !self.is_expired()
    }

    /// Check if invitation can be accepted
    pub fn can_be_accepted(&self) -> bool {
        self.is_valid()
    }

    /// Mark invitation as expired (called by background job)
    pub fn mark_expired(&mut self) {
        if matches!(self.status, InvitationStatus::Pending) && self.is_expired() {
            self.status = InvitationStatus::Expired;
        }
    }

    /// Generate a secure random token for the invitation
    fn generate_token() -> String {
        // In a real implementation, this would use a cryptographically secure random generator
        // For now, we'll use a UUID-based approach
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        format!("{}{}", 
            uuid1.simple().to_string(),
            uuid2.simple().to_string()
        )[..64].to_string()
    }

    /// Validate email format
    fn validate_email(email: &str) -> Result<(), DomainError> {
        if email.trim().is_empty() {
            return Err(DomainError::invalid_input("Email cannot be empty"));
        }

        // Basic email validation (in a real app, use a proper email validation library)
        if !email.contains('@') || !email.contains('.') {
            return Err(DomainError::invalid_input("Invalid email format"));
        }

        if email.len() > 255 {
            return Err(DomainError::invalid_input(
                "Email cannot be longer than 255 characters",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_invitation() {
        let org_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let inviter_id = Uuid::new_v4();

        let invitation = OrganizationInvitation::new(
            org_id,
            "test@example.com".to_string(),
            role_id,
            inviter_id,
            Some("Welcome to our organization!".to_string()),
        );

        assert!(invitation.is_ok());
        let invitation = invitation.unwrap();
        assert_eq!(invitation.email, "test@example.com");
        assert_eq!(invitation.organization_id, org_id);
        assert_eq!(invitation.role_id, role_id);
        assert_eq!(invitation.invited_by_user_id, inviter_id);
        assert!(invitation.is_valid());
        assert!(invitation.can_be_accepted());
        assert!(!invitation.token.is_empty());
    }

    #[test]
    fn test_email_validation() {
        let org_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let inviter_id = Uuid::new_v4();

        // Empty email
        let result = OrganizationInvitation::new(
            org_id, "".to_string(), role_id, inviter_id, None
        );
        assert!(result.is_err());

        // Invalid email format
        let result = OrganizationInvitation::new(
            org_id, "invalid-email".to_string(), role_id, inviter_id, None
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_accept_invitation() {
        let org_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let inviter_id = Uuid::new_v4();

        let mut invitation = OrganizationInvitation::new(
            org_id,
            "test@example.com".to_string(),
            role_id,
            inviter_id,
            None,
        ).unwrap();

        let result = invitation.accept();
        assert!(result.is_ok());
        assert!(matches!(invitation.status, InvitationStatus::Accepted));
        assert!(invitation.accepted_at.is_some());

        // Try to accept again
        let result = invitation.accept();
        assert!(result.is_err());
    }

    #[test]
    fn test_cancel_invitation() {
        let org_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let inviter_id = Uuid::new_v4();

        let mut invitation = OrganizationInvitation::new(
            org_id,
            "test@example.com".to_string(),
            role_id,
            inviter_id,
            None,
        ).unwrap();

        let result = invitation.cancel();
        assert!(result.is_ok());
        assert!(matches!(invitation.status, InvitationStatus::Cancelled));

        // Try to cancel again
        let result = invitation.cancel();
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_invitation() {
        let org_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let inviter_id = Uuid::new_v4();

        // Create invitation that expires in the past
        let past_expiry = Utc::now() - Duration::days(1);
        let mut invitation = OrganizationInvitation::new_with_expiry(
            org_id,
            "test@example.com".to_string(),
            role_id,
            inviter_id,
            past_expiry,
            None,
        ).unwrap();

        assert!(invitation.is_expired());
        assert!(!invitation.is_valid());
        assert!(!invitation.can_be_accepted());

        // Try to accept expired invitation
        let result = invitation.accept();
        assert!(result.is_err());
        assert!(matches!(invitation.status, InvitationStatus::Expired));
    }

    #[test]
    fn test_mark_expired() {
        let org_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let inviter_id = Uuid::new_v4();

        let past_expiry = Utc::now() - Duration::days(1);
        let mut invitation = OrganizationInvitation::new_with_expiry(
            org_id,
            "test@example.com".to_string(),
            role_id,
            inviter_id,
            past_expiry,
            None,
        ).unwrap();

        invitation.mark_expired();
        assert!(matches!(invitation.status, InvitationStatus::Expired));
    }
} 