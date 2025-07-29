use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;

/// Organization member role permission entity representing the assignment of a role permission group
/// to a specific member in an organization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationMemberRolePermission {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role_permission_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl OrganizationMemberRolePermission {
    /// Create a new organization member role permission
    pub fn new(
        organization_id: Uuid,
        user_id: Uuid,
        role_permission_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            organization_id,
            user_id,
            role_permission_id,
            created_at: Utc::now(),
        }
    }
} 