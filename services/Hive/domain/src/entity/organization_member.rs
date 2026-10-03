use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::organization_member_role_permission::OrganizationMemberRolePermission;
use rustycog::core::error::DomainError;

/// Organization member entity representing a user's membership in an organization
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationMember {
    pub id: Option<Uuid>,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    /// Trust-domain issuer of the member principal (ADR-0305). Historical default: `iamrusty`.
    pub issuer: String,
    pub roles: Vec<OrganizationMemberRolePermission>,
    pub status: MemberStatus,
    pub invited_by_user_id: Option<Uuid>,
    pub invited_at: Option<DateTime<Utc>>,
    pub joined_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Member status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MemberStatus {
    #[default]
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
    /// Create a new organization member (for direct addition).
    ///
    /// `issuer` is the trust-domain issuer from the authenticated JWT principal.
    /// Historical HS256 IT tokens use `iamrusty`.
    #[must_use]
    pub fn new(
        organization_id: Uuid,
        user_id: Uuid,
        issuer: impl Into<String>,
        invited_by_user_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            organization_id,
            user_id,
            issuer: issuer.into(),
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
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the member is not pending.
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
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the member is not active.
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
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the member is not suspended.
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
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.status, MemberStatus::Active)
    }

    /// Check if member is pending
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self.status, MemberStatus::Pending)
    }

    /// Check if member is suspended
    #[must_use]
    pub const fn is_suspended(&self) -> bool {
        matches!(self.status, MemberStatus::Suspended)
    }
}

/// Historical platform issuer used by SQL backfill and HS256 tests (ADR-0305).
pub const HISTORICAL_PLATFORM_ISSUER: &str = "iamrusty";

/// Platform IAM issuer URLs end with `/iam` and are not org-scoped (`/iam/orgs/`).
#[must_use]
pub fn is_platform_url_issuer(issuer: &str) -> bool {
    issuer.ends_with("/iam") && !issuer.contains("/iam/orgs/")
}

/// Issuers to try for membership lookup / duplicate detection.
///
/// Org-managed issuers (`/iam/orgs/`) are never aliased. Platform URL
/// issuers also match historical `iamrusty` rows.
#[must_use]
pub fn platform_issuer_aliases(issuer: &str) -> Vec<String> {
    if issuer.contains("/iam/orgs/") {
        return vec![issuer.to_string()];
    }
    if issuer == HISTORICAL_PLATFORM_ISSUER || is_platform_url_issuer(issuer) {
        let mut aliases = vec![issuer.to_string()];
        if issuer != HISTORICAL_PLATFORM_ISSUER {
            aliases.push(HISTORICAL_PLATFORM_ISSUER.to_string());
        }
        return aliases;
    }
    vec![issuer.to_string()]
}

/// Issuers to try for membership lookup / duplicate detection.
///
/// Exact `principal.iss` first; if it is a platform URL, also try `iamrusty`.
#[must_use]
pub fn membership_lookup_issuers(principal_iss: &str) -> Vec<String> {
    platform_issuer_aliases(principal_iss)
}
