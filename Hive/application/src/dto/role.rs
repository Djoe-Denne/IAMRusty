use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::PaginationResponse;

// =============================================================================
// Role Request DTOs
// =============================================================================

/// DTO for creating a new role
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateRoleRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_system_default: Option<bool>,
}

/// DTO for updating a role
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateRoleRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}

// =============================================================================
// Role Response DTOs
// =============================================================================

/// DTO for role response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_system_default: bool,
    pub member_count: Option<i64>, // Number of members with this role
    pub created_at: DateTime<Utc>,
}

/// DTO for paginated list of roles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleListResponse {
    pub roles: Vec<RoleResponse>,
    pub pagination: PaginationResponse,
}
