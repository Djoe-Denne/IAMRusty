use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;

/// Organization member entity representing a user's membership in an organization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationMember {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role_id: Uuid,
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

impl OrganizationMember {
    /// Create a new organization member (for direct addition)
    pub fn new(
        organization_id: Uuid,
        user_id: Uuid,
        role_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            organization_id,
            user_id,
            role_id,
            status: MemberStatus::Active,
            invited_by_user_id: None,
            invited_at: None,
            joined_at: Some(now),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new pending member from invitation
    pub fn new_from_invitation(
        organization_id: Uuid,
        user_id: Uuid,
        role_id: Uuid,
        invited_by_user_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            organization_id,
            user_id,
            role_id,
            status: MemberStatus::Pending,
            invited_by_user_id: Some(invited_by_user_id),
            invited_at: Some(now),
            joined_at: None,
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
            MemberStatus::Active => {
                Err(DomainError::business_rule_violation(
                    "Member is already active",
                ))
            }
            MemberStatus::Suspended => {
                Err(DomainError::business_rule_violation(
                    "Cannot activate suspended member. Remove suspension first.",
                ))
            }
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
            MemberStatus::Pending => {
                Err(DomainError::business_rule_violation(
                    "Cannot suspend pending member",
                ))
            }
            MemberStatus::Suspended => {
                Err(DomainError::business_rule_violation(
                    "Member is already suspended",
                ))
            }
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
            MemberStatus::Active => {
                Err(DomainError::business_rule_violation(
                    "Member is already active",
                ))
            }
            MemberStatus::Pending => {
                Err(DomainError::business_rule_violation(
                    "Cannot reactivate pending member. Use activate instead.",
                ))
            }
        }
    }

    /// Update member role
    pub fn update_role(&mut self, new_role_id: Uuid) {
        self.role_id = new_role_id;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_new_member() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let member = OrganizationMember::new(org_id, user_id, role_id);

        assert_eq!(member.organization_id, org_id);
        assert_eq!(member.user_id, user_id);
        assert_eq!(member.role_id, role_id);
        assert!(member.is_active());
        assert!(member.joined_at.is_some());
        assert!(member.invited_by_user_id.is_none());
    }

    #[test]
    fn test_create_member_from_invitation() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let inviter_id = Uuid::new_v4();

        let member = OrganizationMember::new_from_invitation(
            org_id, user_id, role_id, inviter_id
        );

        assert_eq!(member.organization_id, org_id);
        assert_eq!(member.user_id, user_id);
        assert_eq!(member.role_id, role_id);
        assert!(member.is_pending());
        assert!(member.joined_at.is_none());
        assert_eq!(member.invited_by_user_id, Some(inviter_id));
        assert!(member.invited_at.is_some());
    }

    #[test]
    fn test_activate_pending_member() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let inviter_id = Uuid::new_v4();

        let mut member = OrganizationMember::new_from_invitation(
            org_id, user_id, role_id, inviter_id
        );

        let result = member.activate();
        assert!(result.is_ok());
        assert!(member.is_active());
        assert!(member.joined_at.is_some());
    }

    #[test]
    fn test_suspend_and_reactivate_member() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let mut member = OrganizationMember::new(org_id, user_id, role_id);

        // Suspend member
        let result = member.suspend();
        assert!(result.is_ok());
        assert!(member.is_suspended());

        // Reactivate member
        let result = member.reactivate();
        assert!(result.is_ok());
        assert!(member.is_active());
    }

    #[test]
    fn test_cannot_activate_already_active_member() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let mut member = OrganizationMember::new(org_id, user_id, role_id);

        let result = member.activate();
        assert!(result.is_err());
    }

    #[test]
    fn test_update_member_role() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let new_role_id = Uuid::new_v4();

        let mut member = OrganizationMember::new(org_id, user_id, role_id);
        let original_updated_at = member.updated_at;

        member.update_role(new_role_id);

        assert_eq!(member.role_id, new_role_id);
        assert!(member.updated_at > original_updated_at);
    }
} 