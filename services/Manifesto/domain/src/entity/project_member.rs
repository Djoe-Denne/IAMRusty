use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::entity::project_member_role_permission::ProjectMemberRolePermission;
use crate::value_objects::{MemberSource, PermissionLevel};
use rustycog::core::error::DomainError;

#[derive(Debug, Clone)]
pub struct ProjectMember {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub source: MemberSource,
    pub added_by: Option<Uuid>,
    pub added_at: DateTime<Utc>,
    pub removed_at: Option<DateTime<Utc>>,
    pub removal_reason: Option<String>,
    pub grace_period_ends_at: Option<DateTime<Utc>>,
    pub last_access_at: Option<DateTime<Utc>>,
    pub is_owner: bool,
    pub role_permissions: Vec<ProjectMemberRolePermission>,
}

impl ProjectMember {
    /// Create a new project member
    #[must_use]
    pub fn new(
        project_id: Uuid,
        user_id: Uuid,
        source: MemberSource,
        added_by: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            user_id,
            source,
            added_by,
            added_at: Utc::now(),
            removed_at: None,
            removal_reason: None,
            grace_period_ends_at: None,
            last_access_at: None,
            is_owner: false,
            role_permissions: Vec::new(),
        }
    }

    /// Check if the member is active (not removed)
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.removed_at.is_none()
    }

    /// Update the member's role permissions.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the member has been removed.
    pub fn update_role_permissions(
        &mut self,
        role_permissions: Vec<ProjectMemberRolePermission>,
    ) -> Result<(), DomainError> {
        if !self.is_active() {
            return Err(DomainError::business_rule_violation(
                "Cannot update role permissions of removed member",
            ));
        }

        self.role_permissions = role_permissions;
        Ok(())
    }

    /// Check if member has specific permission on a resource
    #[must_use]
    pub fn has_permission(
        &self,
        resource_name: &str,
        required_permission: &PermissionLevel,
    ) -> bool {
        if !self.is_active() {
            return false;
        }

        // Use case-insensitive comparison since resource names in DB may be capitalized
        self.role_permissions.iter().any(|rp| {
            rp.role_permission
                .resource
                .name
                .eq_ignore_ascii_case(resource_name)
                && rp
                    .role_permission
                    .permission
                    .level
                    .has_permission(required_permission)
        })
    }

    /// Get permission level for a specific resource
    #[must_use]
    pub fn get_permission_for_resource(&self, resource_name: &str) -> Option<PermissionLevel> {
        if !self.is_active() {
            return None;
        }

        // Use case-insensitive comparison since resource names in DB may be capitalized
        self.role_permissions
            .iter()
            .find(|rp| {
                rp.role_permission
                    .resource
                    .name
                    .eq_ignore_ascii_case(resource_name)
            })
            .map(|rp| rp.role_permission.permission.level)
    }

    /// Check if member can manage another member (based on project resource permission)
    #[must_use]
    pub fn can_manage_member(&self, other: &Self) -> bool {
        if !self.is_active() || !other.is_active() {
            return false;
        }
        if other.is_project_owner() {
            return false;
        }
        let self_level = self
            .get_permission_for_resource("project")
            .or_else(|| self.get_permission_for_resource("member"));
        let other_level = other
            .get_permission_for_resource("project")
            .unwrap_or(PermissionLevel::Read);
        self_level.is_some_and(|level| level.can_manage(&other_level))
    }

    /// Whether this member is the unique project owner.
    #[must_use]
    pub fn is_project_owner(&self) -> bool {
        self.is_active()
            && (self.is_owner || self.has_permission("project", &PermissionLevel::Owner))
    }

    /// Whether a removed membership can still be restored.
    #[must_use]
    pub fn is_within_grace(&self, now: DateTime<Utc>) -> bool {
        match (self.removed_at, self.grace_period_ends_at) {
            (Some(_), Some(ends_at)) => now < ends_at,
            _ => false,
        }
    }

    /// Restore a soft-deleted membership during the grace period.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the member is still active or the grace period has expired.
    pub fn restore(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.is_active() {
            return Err(DomainError::business_rule_violation(
                "Cannot restore an active member",
            ));
        }
        if !self.is_within_grace(now) {
            return Err(DomainError::business_rule_violation(
                "Membership grace period has expired",
            ));
        }
        self.removed_at = None;
        self.removal_reason = None;
        self.grace_period_ends_at = None;
        self.is_owner = false;
        Ok(())
    }

    /// Remove the member from the project
    pub fn remove(&mut self, reason: Option<String>, grace_period_days: Option<i64>) {
        self.removed_at = Some(Utc::now());
        self.removal_reason = reason;
        self.is_owner = false;

        if let Some(days) = grace_period_days {
            self.grace_period_ends_at = Some(Utc::now() + chrono::Duration::days(days));
        }
    }

    /// Update last access time
    pub fn update_last_access(&mut self) {
        self.last_access_at = Some(Utc::now());
    }

    /// Validate the member.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `removal_reason` exceeds 100 characters.
    pub fn validate(&self) -> Result<(), DomainError> {
        if let Some(reason) = &self.removal_reason {
            if reason.len() > 100 {
                return Err(DomainError::invalid_input(
                    "Removal reason cannot exceed 100 characters",
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_keeps_id_during_grace() {
        let mut member =
            ProjectMember::new(Uuid::new_v4(), Uuid::new_v4(), MemberSource::Direct, None);
        let original_id = member.id;
        member.remove(Some("left".into()), Some(7));
        member.restore(Utc::now()).expect("within grace");
        assert_eq!(member.id, original_id);
        assert!(member.is_active());
        assert!(!member.is_owner);
    }

    #[test]
    fn restore_fails_after_grace() {
        let mut member =
            ProjectMember::new(Uuid::new_v4(), Uuid::new_v4(), MemberSource::Direct, None);
        member.remove(Some("left".into()), Some(0));
        member.grace_period_ends_at = Some(Utc::now() - chrono::Duration::seconds(1));
        assert!(member.restore(Utc::now()).is_err());
    }
}
