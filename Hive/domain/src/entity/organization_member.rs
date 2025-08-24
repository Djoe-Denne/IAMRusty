use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{entity::organization_member_role_permission::OrganizationMemberRolePermission};
use rustycog_core::error::DomainError;

/// Organization member entity representing a user's membership in an organization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationMember {
    pub id: Option<Uuid>,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub roles: Vec<OrganizationMemberRolePermission>,
    pub status: MemberStatus,
    pub invited_by_user_id: Option<Uuid>,
    pub invited_at: Option<DateTime<Utc>>,
    pub joined_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Member status enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MemberStatus {
    Pending,
    Active,
    Suspended,
}

impl From<MemberStatus> for String {
    fn from(status: MemberStatus) -> Self {
        match status {
            MemberStatus::Pending => "pending".to_string(),
            MemberStatus::Active => "active".to_string(),
            MemberStatus::Suspended => "suspended".to_string(),
        }
    }
}

impl OrganizationMember {
    /// Create a new organization member (for direct addition)
    pub fn new(organization_id: Uuid, user_id: Uuid, invited_by_user_id: Option<Uuid>) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            organization_id,
            user_id,
            roles: vec![],
            status: MemberStatus::Active,
            invited_by_user_id,
            invited_at: None,
            joined_at: Some(now),
            created_at: now,
            updated_at: now,
        }
    }

    /// Activate a pending member (when they accept invitation)
    pub fn activate(&mut self) -> Result<(), DomainError> {
        match self.status {
            MemberStatus::Pending => {
                self.status = MemberStatus::Active;
                self.joined_at = Some(Utc::now());
                self.updated_at = Utc::now();
                Ok(())
            }
            MemberStatus::Active => Err(DomainError::business_rule_violation(
                "Member is already active",
            )),
            MemberStatus::Suspended => Err(DomainError::business_rule_violation(
                "Cannot activate suspended member. Remove suspension first.",
            )),
        }
    }

    /// Suspend a member
    pub fn suspend(&mut self) -> Result<(), DomainError> {
        match self.status {
            MemberStatus::Active => {
                self.status = MemberStatus::Suspended;
                self.updated_at = Utc::now();
                Ok(())
            }
            MemberStatus::Pending => Err(DomainError::business_rule_violation(
                "Cannot suspend pending member",
            )),
            MemberStatus::Suspended => Err(DomainError::business_rule_violation(
                "Member is already suspended",
            )),
        }
    }

    /// Reactivate a suspended member
    pub fn reactivate(&mut self) -> Result<(), DomainError> {
        match self.status {
            MemberStatus::Suspended => {
                self.status = MemberStatus::Active;
                self.updated_at = Utc::now();
                Ok(())
            }
            MemberStatus::Active => Err(DomainError::business_rule_violation(
                "Member is already active",
            )),
            MemberStatus::Pending => Err(DomainError::business_rule_violation(
                "Cannot reactivate pending member. Use activate instead.",
            )),
        }
    }

    /// Update member role
    pub fn update_roles(&mut self, new_roles: Vec<OrganizationMemberRolePermission>) {
        self.roles = new_roles;
        self.updated_at = Utc::now();
    }

    /// Check if member is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, MemberStatus::Active)
    }

    /// Check if member is pending
    pub fn is_pending(&self) -> bool {
        matches!(self.status, MemberStatus::Pending)
    }

    /// Check if member is suspended
    pub fn is_suspended(&self) -> bool {
        matches!(self.status, MemberStatus::Suspended)
    }
}

impl Default for MemberStatus {
    fn default() -> Self {
        MemberStatus::Pending
    }
}
