use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::PaginationResponse;

// =============================================================================
// Sync Job Request DTOs
// =============================================================================

/// DTO for starting a sync job
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct StartSyncJobRequest {
    pub external_link_id: Uuid,
    pub job_type: String, // "full_sync", "incremental_sync", "members_only"
    pub options: Option<serde_json::Value>,
}

// =============================================================================
// Sync Job Response DTOs
// =============================================================================

/// DTO for sync job response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub external_link_id: Uuid,
    pub job_type: String,
    pub status: String, // "running", "completed", "failed"
    pub items_processed: i32,
    pub items_created: i32,
    pub items_updated: i32,
    pub items_failed: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub details: Option<serde_json::Value>,
}

/// DTO for paginated list of sync jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobListResponse {
    pub sync_jobs: Vec<SyncJobResponse>,
    pub pagination: PaginationResponse,
}

/// DTO for sync job status (lightweight)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobStatusResponse {
    pub id: Uuid,
    pub status: String,
    pub progress_percentage: Option<f32>,
    pub current_operation: Option<String>,
    pub items_processed: i32,
    pub estimated_completion: Option<DateTime<Utc>>,
}

/// DTO for sync job logs/details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobLogsResponse {
    pub id: Uuid,
    pub logs: Vec<SyncJobLogEntry>,
    pub summary: Option<serde_json::Value>,
}

/// DTO for individual sync job log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String, // "info", "warn", "error"
    pub message: String,
    pub details: Option<serde_json::Value>,
}
