use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::entity::project_member_role_permission::ProjectMemberRolePermission;
use crate::value_objects::{MemberSource, PermissionLevel};
use rustycog_core::error::DomainError;

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
            role_permissions: Vec::new(),
        }
    }

    /// Check if the member is active (not removed)
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.removed_at.is_none()
    }

    /// Update the member's role permissions
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

        // Check if this member has admin or owner permission on "member" resource
        self.has_permission("member", &PermissionLevel::Admin)
    }

    /// Remove the member from the project
    pub fn remove(&mut self, reason: Option<String>, grace_period_days: Option<i64>) {
        self.removed_at = Some(Utc::now());
        self.removal_reason = reason;

        if let Some(days) = grace_period_days {
            self.grace_period_ends_at = Some(Utc::now() + chrono::Duration::days(days));
        }
    }

    /// Update last access time
    pub fn update_last_access(&mut self) {
        self.last_access_at = Some(Utc::now());
    }

    /// Validate the member
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
