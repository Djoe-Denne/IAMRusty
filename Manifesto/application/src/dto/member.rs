use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Resource-permission combination
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ResourcePermissionRequest {
    pub resource: String,  // e.g., "project", "member", or component type
    pub permission: String,  // read, write, admin, owner
}

/// Resource-permission response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePermissionResponse {
    pub resource: String,
    pub permission: String,
}

/// Request to add a member to a project (starts with project permission only)
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
    #[validate(length(min = 1, message = "Resource cannot be empty"))]
    pub resource: Option<String>,  // Defaults to "project" if not provided
    #[validate(length(min = 1, message = "Permission cannot be empty"))]
    pub permission: String,  // read, write, admin, owner
}

/// Request to update a member's permissions
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateMemberPermissionsRequest {
    pub permissions: Vec<ResourcePermissionRequest>,
}

/// Request to grant a permission on a resource
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GrantPermissionRequest {
    #[validate(length(min = 1, message = "Resource cannot be empty"))]
    pub resource: String,
    #[validate(length(min = 1, message = "Permission cannot be empty"))]
    pub permission: String,
}

/// Member response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub permissions: Vec<ResourcePermissionResponse>,
    pub source: String,
    pub added_by: Option<Uuid>,
    pub added_at: DateTime<Utc>,
    pub removed_at: Option<DateTime<Utc>>,
    pub removal_reason: Option<String>,
    pub grace_period_ends_at: Option<DateTime<Utc>>,
    pub last_access_at: Option<DateTime<Utc>>,
}

/// List members response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberListResponse {
    pub data: Vec<MemberResponse>,
    pub pagination: crate::dto::PaginationResponse,
}

