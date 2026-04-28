use chrono::{DateTime, Utc};
use rustycog_core::error::DomainError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Sync job entity for tracking synchronization operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncJob {
    pub id: Uuid,
    pub organization_external_link_id: Uuid,
    pub job_type: SyncJobType,
    pub status: SyncJobStatus,
    pub items_processed: i32,
    pub items_created: i32,
    pub items_updated: i32,
    pub items_failed: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub details: Value,
    pub created_at: DateTime<Utc>,
}

/// Sync job type enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncJobType {
    FullSync,
    IncrementalSync,
    MembersOnly,
}

/// Sync job status enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncJobStatus {
    Running,
    Completed,
    Failed,
}

impl SyncJob {
    /// Create a new sync job
    pub fn new(
        organization_external_link_id: Uuid,
        job_type: SyncJobType,
        details: Option<Value>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            organization_external_link_id,
            job_type,
            status: SyncJobStatus::Running,
            items_processed: 0,
            items_created: 0,
            items_updated: 0,
            items_failed: 0,
            started_at: now,
            completed_at: None,
            error_message: None,
            details: details.unwrap_or_else(|| serde_json::json!({})),
            created_at: now,
        }
    }

    /// Mark job as completed successfully
    pub fn complete_successfully(&mut self) -> Result<(), DomainError> {
        match self.status {
            SyncJobStatus::Running => {
                self.status = SyncJobStatus::Completed;
                self.completed_at = Some(Utc::now());
                self.error_message = None;
                Ok(())
            }
            SyncJobStatus::Completed => Err(DomainError::business_rule_violation(
                "Job is already completed",
            )),
            SyncJobStatus::Failed => Err(DomainError::business_rule_violation(
                "Cannot complete a failed job",
            )),
        }
    }

    /// Mark job as failed
    pub fn fail(&mut self, error_message: String) -> Result<(), DomainError> {
        match self.status {
            SyncJobStatus::Running => {
                self.status = SyncJobStatus::Failed;
                self.completed_at = Some(Utc::now());
                self.error_message = Some(error_message);
                Ok(())
            }
            SyncJobStatus::Completed => Err(DomainError::business_rule_violation(
                "Cannot fail a completed job",
            )),
            SyncJobStatus::Failed => Err(DomainError::business_rule_violation(
                "Job is already failed",
            )),
        }
    }

    /// Update job progress
    pub fn update_progress(
        &mut self,
        items_processed: i32,
        items_created: i32,
        items_updated: i32,
        items_failed: i32,
    ) -> Result<(), DomainError> {
        if !self.is_running() {
            return Err(DomainError::business_rule_violation(
                "Cannot update progress of non-running job",
            ));
        }

        self.items_processed = items_processed;
        self.items_created = items_created;
        self.items_updated = items_updated;
        self.items_failed = items_failed;
        Ok(())
    }

    /// Add to item counts
    pub fn add_progress(
        &mut self,
        processed: i32,
        created: i32,
        updated: i32,
        failed: i32,
    ) -> Result<(), DomainError> {
        if !self.is_running() {
            return Err(DomainError::business_rule_violation(
                "Cannot update progress of non-running job",
            ));
        }

        self.items_processed += processed;
        self.items_created += created;
        self.items_updated += updated;
        self.items_failed += failed;
        Ok(())
    }

    /// Update job details
    pub fn update_details(&mut self, new_details: Value) -> Result<(), DomainError> {
        if !self.is_running() {
            return Err(DomainError::business_rule_violation(
                "Cannot update details of non-running job",
            ));
        }

        self.details = new_details;
        Ok(())
    }

    /// Check if job is running
    pub fn is_running(&self) -> bool {
        matches!(self.status, SyncJobStatus::Running)
    }

    /// Check if job is completed
    pub fn is_completed(&self) -> bool {
        matches!(self.status, SyncJobStatus::Completed)
    }

    /// Check if job failed
    pub fn is_failed(&self) -> bool {
        matches!(self.status, SyncJobStatus::Failed)
    }

    /// Check if job is finished (completed or failed)
    pub fn is_finished(&self) -> bool {
        self.is_completed() || self.is_failed()
    }

    /// Get job duration
    pub fn get_duration(&self) -> Option<chrono::Duration> {
        self.completed_at.map(|end| end - self.started_at)
    }

    /// Calculate success rate
    pub fn get_success_rate(&self) -> f64 {
        if self.items_processed == 0 {
            return 1.0;
        }

        let successful = self.items_created + self.items_updated;
        successful as f64 / self.items_processed as f64
    }

    /// Get summary statistics
    pub fn get_summary(&self) -> SyncJobSummary {
        SyncJobSummary {
            total_processed: self.items_processed,
            successful: self.items_created + self.items_updated,
            failed: self.items_failed,
            success_rate: self.get_success_rate(),
            duration: self.get_duration(),
        }
    }
}

/// Sync job summary statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncJobSummary {
    pub total_processed: i32,
    pub successful: i32,
    pub failed: i32,
    pub success_rate: f64,
    pub duration: Option<chrono::Duration>,
}

impl SyncJobType {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncJobType::FullSync => "full_sync",
            SyncJobType::IncrementalSync => "incremental_sync",
            SyncJobType::MembersOnly => "members_only",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "full_sync" => Ok(SyncJobType::FullSync),
            "incremental_sync" => Ok(SyncJobType::IncrementalSync),
            "members_only" => Ok(SyncJobType::MembersOnly),
            _ => Err(DomainError::invalid_input(&format!(
                "Unknown sync job type: {}",
                s
            ))),
        }
    }
}

impl SyncJobStatus {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncJobStatus::Running => "running",
            SyncJobStatus::Completed => "completed",
            SyncJobStatus::Failed => "failed",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "running" => Ok(SyncJobStatus::Running),
            "completed" => Ok(SyncJobStatus::Completed),
            "failed" => Ok(SyncJobStatus::Failed),
            _ => Err(DomainError::invalid_input(&format!(
                "Unknown sync job status: {}",
                s
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sync_job() {
        let link_id = Uuid::new_v4();
        let details = serde_json::json!({"dry_run": true});

        let job = SyncJob::new(link_id, SyncJobType::FullSync, Some(details.clone()));

        assert_eq!(job.organization_external_link_id, link_id);
        assert!(matches!(job.job_type, SyncJobType::FullSync));
        assert!(job.is_running());
        assert_eq!(job.details, details);
        assert_eq!(job.items_processed, 0);
    }

    #[test]
    fn test_complete_job_successfully() {
        let link_id = Uuid::new_v4();
        let mut job = SyncJob::new(link_id, SyncJobType::IncrementalSync, None);

        let result = job.complete_successfully();
        assert!(result.is_ok());
        assert!(job.is_completed());
        assert!(job.is_finished());
        assert!(job.completed_at.is_some());
        assert!(job.error_message.is_none());

        // Try to complete again
        let result = job.complete_successfully();
        assert!(result.is_err());
    }

    #[test]
    fn test_fail_job() {
        let link_id = Uuid::new_v4();
        let mut job = SyncJob::new(link_id, SyncJobType::MembersOnly, None);

        let error_msg = "Connection timeout".to_string();
        let result = job.fail(error_msg.clone());

        assert!(result.is_ok());
        assert!(job.is_failed());
        assert!(job.is_finished());
        assert!(job.completed_at.is_some());
        assert_eq!(job.error_message, Some(error_msg));

        // Try to fail again
        let result = job.fail("Another error".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_update_progress() {
        let link_id = Uuid::new_v4();
        let mut job = SyncJob::new(link_id, SyncJobType::FullSync, None);

        let result = job.update_progress(100, 20, 30, 5);
        assert!(result.is_ok());
        assert_eq!(job.items_processed, 100);
        assert_eq!(job.items_created, 20);
        assert_eq!(job.items_updated, 30);
        assert_eq!(job.items_failed, 5);

        // Add more progress
        let result = job.add_progress(50, 10, 15, 2);
        assert!(result.is_ok());
        assert_eq!(job.items_processed, 150);
        assert_eq!(job.items_created, 30);
        assert_eq!(job.items_updated, 45);
        assert_eq!(job.items_failed, 7);
    }

    #[test]
    fn test_cannot_update_finished_job() {
        let link_id = Uuid::new_v4();
        let mut job = SyncJob::new(link_id, SyncJobType::FullSync, None);

        job.complete_successfully().unwrap();

        let result = job.update_progress(100, 20, 30, 5);
        assert!(result.is_err());

        let result = job.update_details(serde_json::json!({"new": "data"}));
        assert!(result.is_err());
    }

    #[test]
    fn test_job_summary() {
        let link_id = Uuid::new_v4();
        let mut job = SyncJob::new(link_id, SyncJobType::FullSync, None);

        job.update_progress(100, 30, 40, 30).unwrap();
        job.complete_successfully().unwrap();

        let summary = job.get_summary();
        assert_eq!(summary.total_processed, 100);
        assert_eq!(summary.successful, 70); // created + updated
        assert_eq!(summary.failed, 30);
        assert_eq!(summary.success_rate, 0.7);
        assert!(summary.duration.is_some());
    }

    #[test]
    fn test_sync_job_type_conversion() {
        assert_eq!(SyncJobType::FullSync.as_str(), "full_sync");
        assert_eq!(SyncJobType::IncrementalSync.as_str(), "incremental_sync");
        assert_eq!(SyncJobType::MembersOnly.as_str(), "members_only");

        assert!(matches!(
            SyncJobType::from_str("full_sync").unwrap(),
            SyncJobType::FullSync
        ));
        assert!(SyncJobType::from_str("invalid").is_err());
    }

    #[test]
    fn test_sync_job_status_conversion() {
        assert_eq!(SyncJobStatus::Running.as_str(), "running");
        assert_eq!(SyncJobStatus::Completed.as_str(), "completed");
        assert_eq!(SyncJobStatus::Failed.as_str(), "failed");

        assert!(matches!(
            SyncJobStatus::from_str("running").unwrap(),
            SyncJobStatus::Running
        ));
        assert!(SyncJobStatus::from_str("invalid").is_err());
    }
}
