use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::PaginationResponse;

// =============================================================================
// Member Request DTOs
// =============================================================================

/// DTO for adding a member to an organization
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
    pub role_id: Uuid,
}

/// DTO for updating a member's role or status
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateMemberRequest {
    pub role_id: Option<Uuid>,
    pub status: Option<String>,
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
    pub role_id: Uuid,
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
