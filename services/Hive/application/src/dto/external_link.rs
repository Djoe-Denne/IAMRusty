use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::PaginationResponse;

// =============================================================================
// External Link Request DTOs
// =============================================================================

/// DTO for creating an external provider link
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateExternalLinkRequest {
    pub provider_id: Uuid,
    pub provider_config: serde_json::Value,
    pub sync_enabled: Option<bool>,
    pub sync_settings: Option<serde_json::Value>,
}

/// DTO for updating an external provider link
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateExternalLinkRequest {
    pub provider_config: Option<serde_json::Value>,
    pub sync_enabled: Option<bool>,
    pub sync_settings: Option<serde_json::Value>,
}

/// DTO for toggling sync on/off
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ToggleSyncRequest {
    pub enabled: bool,
}

// =============================================================================
// External Link Response DTOs
// =============================================================================

/// DTO for external link response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalLinkResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub provider_id: Uuid,
    pub provider_name: String,
    pub provider_config: serde_json::Value,
    pub sync_enabled: bool,
    pub sync_settings: serde_json::Value,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sync_status: Option<String>,
    pub sync_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for paginated list of external links
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalLinkListResponse {
    pub external_links: Vec<ExternalLinkResponse>,
    pub pagination: PaginationResponse,
}

/// DTO for connection test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResponse {
    pub success: bool,
    pub message: String,
    pub details: Option<serde_json::Value>,
}
