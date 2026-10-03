use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::role_permission::RolePermission;

/// Project member role permission entity representing the assignment of a role permission
/// to a specific member in a project
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectMemberRolePermission {
    pub id: Option<Uuid>,
    pub member_id: Uuid,
    pub role_permission: RolePermission,
    pub created_at: DateTime<Utc>,
}

impl ProjectMemberRolePermission {
    /// Create a new project member role permission
    #[must_use]
    pub const fn new(
        id: Option<Uuid>,
        member_id: Uuid,
        role_permission: RolePermission,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            member_id,
            role_permission,
            created_at,
        }
    }
}
