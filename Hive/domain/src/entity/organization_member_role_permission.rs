use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::role_permission::RolePermission;

/// Organization member role permission entity representing the assignment of a role permission group
/// to a specific member in an organization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationMemberRolePermission {
    pub id: Option<Uuid>,
    pub organization_id: Uuid,
    pub member_id: Uuid,
    pub role_permission: RolePermission,
    pub created_at: DateTime<Utc>,
}

impl OrganizationMemberRolePermission {
    /// Create a new organization member role permission
    #[must_use]
    pub fn new(
        id: Option<Uuid>,
        organization_id: &Uuid,
        member_id: &Uuid,
        role_permission: &RolePermission,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            organization_id: *organization_id,
            member_id: *member_id,
            role_permission: role_permission.clone(),
            created_at,
        }
    }
}
