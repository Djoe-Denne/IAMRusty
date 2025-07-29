use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog_events::BaseEvent;

// =============================================================================
// Sync Events
// =============================================================================

/// Event published when a sync job starts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobStartedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub external_link_id: Uuid,
    pub sync_job_id: Uuid,
    pub job_type: String,
    pub started_at: DateTime<Utc>,
}

/// Event published when a sync job completes (success or failure)
/// Failed jobs trigger error notifications via Telegraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub organization_id: Uuid,
    pub external_link_id: Uuid,
    pub sync_job_id: Uuid,
    pub job_type: String,
    pub status: String, // "completed", "failed"
    pub items_processed: i32,
    pub items_created: i32,
    pub items_updated: i32,
    pub items_failed: i32,
    pub error_message: Option<String>,
    pub completed_at: DateTime<Utc>,
}

// =============================================================================
// Implementations
// =============================================================================

impl SyncJobStartedEvent {
    pub fn new(organization_id: Uuid, external_link_id: Uuid, sync_job_id: Uuid, job_type: String, started_at: DateTime<Utc>) -> Self {
        Self {
            base: BaseEvent::new("sync_job_started".to_string(), organization_id),
            organization_id,
            external_link_id,
            sync_job_id,
            job_type,
            started_at,
        }
    }
}

impl SyncJobCompletedEvent {
    pub fn new(organization_id: Uuid, external_link_id: Uuid, sync_job_id: Uuid, job_type: String, status: String, items_processed: i32, items_created: i32, items_updated: i32, items_failed: i32, error_message: Option<String>, completed_at: DateTime<Utc>) -> Self {
        Self {
            base: BaseEvent::new("sync_job_completed".to_string(), organization_id),
            organization_id,
            external_link_id,
            sync_job_id,
            job_type,
            status,
            items_processed,
            items_created,
            items_updated,
            items_failed,
            error_message,
            completed_at,
        }
    }
}