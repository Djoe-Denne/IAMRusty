use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::role_permission::RolePermission;
use rustycog_core::error::DomainError;

/// Organization invitation entity for inviting users to join an organization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationInvitation {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub organization_name: Option<String>,
    pub aggregate_id: String,
    pub role_permissions: Vec<RolePermission>,
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
        aggregate_id: String,
        role_permissions: Vec<RolePermission>,
        invited_by_user_id: Uuid,
        message: Option<String>,
    ) -> Result<Self, DomainError> {
        let now = Utc::now();
        let expires_at = now + Duration::days(Self::DEFAULT_EXPIRY_DAYS);
        let token = Self::generate_token();

        Ok(Self {
            id: Uuid::new_v4(),
            organization_id,
            organization_name: None,
            aggregate_id,
            role_permissions,
            invited_by_user_id,
            token,
            status: InvitationStatus::Pending,
            expires_at,
            accepted_at: None,
            message,
            created_at: now,
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
            InvitationStatus::Accepted => Err(DomainError::business_rule_violation(
                "Invitation has already been accepted",
            )),
            InvitationStatus::Expired => Err(DomainError::business_rule_violation(
                "Cannot accept expired invitation",
            )),
            InvitationStatus::Cancelled => Err(DomainError::business_rule_violation(
                "Cannot accept cancelled invitation",
            )),
        }
    }

    /// Cancel the invitation
    pub fn cancel(&mut self) -> Result<(), DomainError> {
        match self.status {
            InvitationStatus::Pending => {
                self.status = InvitationStatus::Cancelled;
                Ok(())
            }
            InvitationStatus::Accepted => Err(DomainError::business_rule_violation(
                "Cannot cancel accepted invitation",
            )),
            InvitationStatus::Expired => Err(DomainError::business_rule_violation(
                "Cannot cancel expired invitation",
            )),
            InvitationStatus::Cancelled => Err(DomainError::business_rule_violation(
                "Invitation is already cancelled",
            )),
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
        // TODO: In a real implementation, this would use a cryptographically secure random generator
        // For now, we'll use a UUID-based approach
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        format!(
            "{}{}",
            uuid1.simple().to_string(),
            uuid2.simple().to_string()
        )[..64]
            .to_string()
    }

    pub fn update_organization_name(&mut self, organization_name: &str) {
        self.organization_name = Some(organization_name.to_string());
    }
}

impl InvitationStatus {
    pub fn as_str(&self) -> &str {
        match self {
            InvitationStatus::Pending => "pending",
            InvitationStatus::Accepted => "accepted",
            InvitationStatus::Expired => "expired",
            InvitationStatus::Cancelled => "cancelled",
        }
    }
}
