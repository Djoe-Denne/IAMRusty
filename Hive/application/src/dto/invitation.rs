use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::{PaginationResponse, MemberRole};

// =============================================================================
// Invitation Request DTOs
// =============================================================================

/// DTO for creating an invitation
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateInvitationRequest {
    #[validate(email)]
    pub email: String,
    pub roles: Vec<MemberRole>,
    pub message: Option<String>,
}

// =============================================================================
// Invitation Response DTOs
// =============================================================================

/// DTO for invitation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub email: String,
    pub roles: Vec<MemberRole>,
    pub status: String,
    pub invited_by_user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// DTO for paginated list of invitations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationListResponse {
    pub invitations: Vec<InvitationResponse>,
    pub pagination: PaginationResponse,
}

/// DTO for invitation details (public view with limited info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationDetailsResponse {
    pub organization_name: String,
    pub role_name: String,
    pub invited_by_name: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub message: Option<String>,
}
