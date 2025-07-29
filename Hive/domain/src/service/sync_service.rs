use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{
    entity::*,
    error::DomainError,
    port::*,
};

/// Domain service for sync job management
pub struct SyncServiceImpl<SR, LR, OR>
where
    SR: SyncJobRepository,
    LR: ExternalLinkRepository,
    OR: OrganizationRepository,
{
    sync_job_repo: SR,
    external_link_repo: LR,
    organization_repo: OR,
}

#[async_trait::async_trait]
pub trait SyncService {
    async fn start_sync_job(&self, external_link_id: Uuid, job_type: SyncJobType, requested_by_user_id: Uuid) -> Result<SyncJob, DomainError>;
    async fn update_sync_job_progress(&self, job_id: Uuid, items_processed: i32, items_created: i32, items_updated: i32, items_failed: i32, details: Option<serde_json::Value>) -> Result<SyncJob, DomainError>;
    async fn complete_sync_job(&self, job_id: Uuid, final_stats: Option<(i32, i32, i32, i32)>, details: Option<serde_json::Value>) -> Result<SyncJob, DomainError>;
}

impl<SR, LR, OR> SyncServiceImpl<SR, LR, OR>
where
    SR: SyncJobRepository,
    LR: ExternalLinkRepository,
    OR: OrganizationRepository,
{
    /// Create a new sync service
    pub fn new(
        sync_job_repo: SR,
        external_link_repo: LR,
        organization_repo: OR,
    ) -> Self {
        Self {
            sync_job_repo,
            external_link_repo,
            organization_repo,
        }
    }
    
    /// Check if user has permission to manage sync jobs
    async fn check_sync_permission(
        &self,
        organization_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<(), DomainError> {
        // Find the organization
        let organization = self
            .organization_repo
            .find_by_id(organization_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &organization_id.to_string()))?;

        // Owner always has permission
        if organization.is_owned_by(user_id) {
            return Ok(());
        }

        // For sync operations, we'll allow any active member for now
        // In a more complex system, you might want specific sync permissions
        // TODO: Implement proper permission checking with member repository

        Err(DomainError::business_rule_violation(
            "You do not have permission to manage sync jobs for this organization",
        ))
    }
}

#[async_trait::async_trait]
impl<SR, LR, OR> SyncService for SyncServiceImpl<SR, LR, OR>
where
    SR: SyncJobRepository,
    LR: ExternalLinkRepository,
    OR: OrganizationRepository,
{
    /// Start a new sync job
    async fn start_sync_job(
        &self,
        external_link_id: Uuid,
        job_type: SyncJobType,
        requested_by_user_id: Uuid,
    ) -> Result<SyncJob, DomainError> {
        // Validate external link exists
        let external_link = self
            .external_link_repo
            .find_by_id(&external_link_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ExternalLink", &external_link_id.to_string()))?;

        // Business rule: Check permission to start sync jobs
        self.check_sync_permission(&external_link.organization_id, &requested_by_user_id).await?;

        // Business rule: Sync must be enabled for the external link
        if !external_link.is_sync_enabled() {
            return Err(DomainError::business_rule_violation(
                "Sync is not enabled for this external link",
            ));
        }

        // Business rule: Check if there's already a running job for this external link
        let running_jobs = self
            .sync_job_repo
            .find_running_by_external_link(&external_link_id)
            .await?;

        if !running_jobs.is_empty() {
            return Err(DomainError::business_rule_violation(
                "A sync job is already running for this external link",
            ));
        }

        // Create new sync job
        let sync_job = SyncJob::new(external_link_id, job_type, None);
        let saved_job = self.sync_job_repo.save(&sync_job).await?;

        Ok(saved_job)
    }

    /// Update sync job progress
    async fn update_sync_job_progress(
        &self,
        job_id: Uuid,
        items_processed: i32,
        items_created: i32,
        items_updated: i32,
        items_failed: i32,
        details: Option<serde_json::Value>,
    ) -> Result<SyncJob, DomainError> {
        // Find the sync job
        let mut sync_job = self
            .sync_job_repo
            .find_by_id(&job_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("SyncJob", &job_id.to_string()))?;

        // Business rule: Can only update running jobs
        if !sync_job.is_running() {
            return Err(DomainError::business_rule_violation(
                "Can only update progress for running sync jobs",
            ));
        }

        // Update progress
        sync_job.update_progress(items_processed, items_created, items_updated, items_failed);
        
        if let Some(details) = details {
            sync_job.update_details(details);
        }

        let updated_job = self.sync_job_repo.save(&sync_job).await?;

        Ok(updated_job)
    }

    /// Complete a sync job successfully
    async fn complete_sync_job(
        &self,
        job_id: Uuid,
        final_stats: Option<(i32, i32, i32, i32)>, // (processed, created, updated, failed)
        details: Option<serde_json::Value>,
    ) -> Result<SyncJob, DomainError> {
        // Find the sync job
        let mut sync_job = self
            .sync_job_repo
            .find_by_id(&job_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("SyncJob", &job_id.to_string()))?;

        // Business rule: Can only complete running jobs
        if !sync_job.is_running() {
            return Err(DomainError::business_rule_violation(
                "Can only complete running sync jobs",
            ));
        }

        // Update final stats if provided
        if let Some((processed, created, updated, failed)) = final_stats {
            sync_job.update_progress(processed, created, updated, failed);
        }

        // Complete the job
        sync_job.complete_successfully()?;
        if let Some(details) = details {
            sync_job.update_details(details)?;
        }

        // Update external link last sync info
        let external_link = self
            .external_link_repo
            .find_by_id(&sync_job.organization_external_link_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ExternalLink", &sync_job.organization_external_link_id.to_string()))?;

        let mut updated_link = external_link;
        updated_link.record_sync_success();
        self.external_link_repo.save(&updated_link).await?;

        let updated_job = self.sync_job_repo.save(&sync_job).await?;

        Ok(updated_job)
    }

}

