use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::{PaginationResponse, role::MemberRole};

// =============================================================================
// Member Request DTOs
// =============================================================================

/// DTO for adding a member to an organization
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
    pub roles: Vec<MemberRole>,
}

/// DTO for updating a member's role or status
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateMemberRolesRequest {
    pub roles: Vec<MemberRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemberStatus {
    Pending,
    Active,
    Suspended,
}

// =============================================================================
// Member Response DTOs
// =============================================================================

/// DTO for member response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub invited_by_user_id: Option<Uuid>,
    pub invited_at: Option<DateTime<Utc>>,
    pub joined_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for paginated list of members
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberListResponse {
    pub members: Vec<MemberResponse>,
    pub pagination: PaginationResponse,
}
