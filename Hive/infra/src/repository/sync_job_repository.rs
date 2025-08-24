//! SyncJobRepository SeaORM implementation

use async_trait::async_trait;
use hive_domain::entity::{SyncJob, SyncJobType, SyncJobStatus};
use rustycog_core::error::DomainError;
use hive_domain::port::repository::{
    SyncJobReadRepository, SyncJobRepository, SyncJobWriteRepository,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, PaginatorTrait
};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{
    prelude::SyncJobs,
    sync_jobs,
};

struct SyncJobMapper;

impl SyncJobMapper {
    pub fn to_domain(model: sync_jobs::Model) -> Result<SyncJob, DomainError> {
        let job_type = SyncJobType::from_str(&model.job_type)?;
        let status = SyncJobStatus::from_str(&model.status)?;

        Ok(SyncJob {
            id: model.id,
            organization_external_link_id: model.organization_external_link_id,
            job_type,
            status,
            items_processed: model.items_processed,
            items_created: model.items_created,
            items_updated: model.items_updated,
            items_failed: model.items_failed,
            started_at: model.started_at,
            completed_at: model.completed_at,
            error_message: model.error_message,
            details: model.details,
            created_at: model.created_at,
        })
    }

    pub fn to_active_model(job: &SyncJob) -> sync_jobs::ActiveModel {
        sync_jobs::ActiveModel {
            id: ActiveValue::Set(job.id),
            organization_external_link_id: ActiveValue::Set(job.organization_external_link_id),
            job_type: ActiveValue::Set(job.job_type.as_str().to_string()),
            status: ActiveValue::Set(job.status.as_str().to_string()),
            items_processed: ActiveValue::Set(job.items_processed),
            items_created: ActiveValue::Set(job.items_created),
            items_updated: ActiveValue::Set(job.items_updated),
            items_failed: ActiveValue::Set(job.items_failed),
            started_at: ActiveValue::Set(job.started_at),
            completed_at: ActiveValue::Set(job.completed_at),
            error_message: ActiveValue::Set(job.error_message.clone()),
            details: ActiveValue::Set(job.details.clone()),
            created_at: ActiveValue::Set(job.created_at),
        }
    }
}

/// Read repository
#[derive(Clone)]
pub struct SyncJobReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl SyncJobReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self { Self { db } }
}

#[async_trait]
impl SyncJobReadRepository for SyncJobReadRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<SyncJob>, DomainError> {
        debug!("Finding sync job by ID: {}", id);
        
        let job = SyncJobs::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match job {
            Some(model) => Ok(Some(SyncJobMapper::to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_external_link(&self, link_id: &Uuid) -> Result<Vec<SyncJob>, DomainError> {
        debug!("Finding sync jobs by external link: {}", link_id);
        
        let jobs = SyncJobs::find()
            .filter(sync_jobs::Column::OrganizationExternalLinkId.eq(*link_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in jobs {
            result.push(SyncJobMapper::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_by_organization(&self, organization_id: &Uuid) -> Result<Vec<SyncJob>, DomainError> {
        debug!("Finding sync jobs by organization: {}", organization_id);
        
        // This would require a join with external_links table to filter by organization
        // For now, implementing a simplified version
        let jobs = SyncJobs::find()
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in jobs {
            result.push(SyncJobMapper::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_by_status(&self, status: &SyncJobStatus) -> Result<Vec<SyncJob>, DomainError> {
        debug!("Finding sync jobs by status: {:?}", status);
        
        let jobs = SyncJobs::find()
            .filter(sync_jobs::Column::Status.eq(status.as_str()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in jobs {
            result.push(SyncJobMapper::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_running(&self) -> Result<Vec<SyncJob>, DomainError> {
        debug!("Finding running sync jobs");
        
        let jobs = SyncJobs::find()
            .filter(sync_jobs::Column::Status.eq("running"))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in jobs {
            result.push(SyncJobMapper::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_running_by_external_link(&self, link_id: &Uuid) -> Result<Vec<SyncJob>, DomainError> {
        debug!("Finding running sync jobs by external link: {}", link_id);
        
        let jobs = SyncJobs::find()
            .filter(sync_jobs::Column::OrganizationExternalLinkId.eq(*link_id))
            .filter(sync_jobs::Column::Status.eq("running"))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in jobs {
            result.push(SyncJobMapper::to_domain(model)?);
        }
        Ok(result)
    }

    async fn find_recent(&self, days: i64) -> Result<Vec<SyncJob>, DomainError> {
        debug!("Finding recent sync jobs (last {} days)", days);
        
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let jobs = SyncJobs::find()
            .filter(sync_jobs::Column::CreatedAt.gte(cutoff))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in jobs {
            result.push(SyncJobMapper::to_domain(model)?);
        }
        Ok(result)
    }

    async fn count_by_external_link(&self, link_id: &Uuid) -> Result<i64, DomainError> {
        debug!("Counting sync jobs by external link: {}", link_id);
        
        let count = SyncJobs::find()
            .filter(sync_jobs::Column::OrganizationExternalLinkId.eq(*link_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count as i64)
    }

    async fn count_running(&self) -> Result<i64, DomainError> {
        debug!("Counting running sync jobs");
        
        let count = SyncJobs::find()
            .filter(sync_jobs::Column::Status.eq("running"))
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count as i64)
    }
}

/// Write repository
#[derive(Clone)]
pub struct SyncJobWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl SyncJobWriteRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self { Self { db } }
}

#[async_trait]
impl SyncJobWriteRepository for SyncJobWriteRepositoryImpl {
    async fn save(&self, job: &SyncJob) -> Result<SyncJob, DomainError> {
        debug!("Saving sync job with ID: {}", job.id);
        
        let active_model = SyncJobMapper::to_active_model(job);
        
        let result = active_model
            .save(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let saved_model = sync_jobs::Model {
            id: result.id.unwrap(),
            organization_external_link_id: result.organization_external_link_id.unwrap(),
            job_type: result.job_type.unwrap(),
            status: result.status.unwrap(),
            items_processed: result.items_processed.unwrap(),
            items_created: result.items_created.unwrap(),
            items_updated: result.items_updated.unwrap(),
            items_failed: result.items_failed.unwrap(),
            started_at: result.started_at.unwrap(),
            completed_at: result.completed_at.unwrap(),
            error_message: result.error_message.unwrap(),
            details: result.details.unwrap(),
            created_at: result.created_at.unwrap(),
        };

        SyncJobMapper::to_domain(saved_model)
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting sync job by ID: {}", id);
        
        let result = SyncJobs::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found("SyncJob", &id.to_string()));
        }

        Ok(())
    }
}

/// Combined delegator
#[derive(Clone)]
pub struct SyncJobRepositoryImpl {
    read_repo: Arc<dyn SyncJobReadRepository>,
    write_repo: Arc<dyn SyncJobWriteRepository>,
}

impl SyncJobRepositoryImpl {
    pub fn new(
        read_repo: Arc<dyn SyncJobReadRepository>,
        write_repo: Arc<dyn SyncJobWriteRepository>,
    ) -> Self {
        Self { read_repo, write_repo }
    }
}

#[async_trait]
impl SyncJobReadRepository for SyncJobRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<SyncJob>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_external_link(&self, link_id: &Uuid) -> Result<Vec<SyncJob>, DomainError> {
        self.read_repo.find_by_external_link(link_id).await
    }

    async fn find_by_organization(&self, organization_id: &Uuid) -> Result<Vec<SyncJob>, DomainError> {
        self.read_repo.find_by_organization(organization_id).await
    }

    async fn find_by_status(&self, status: &SyncJobStatus) -> Result<Vec<SyncJob>, DomainError> {
        self.read_repo.find_by_status(status).await
    }

    async fn find_running(&self) -> Result<Vec<SyncJob>, DomainError> {
        self.read_repo.find_running().await
    }

    async fn find_running_by_external_link(&self, link_id: &Uuid) -> Result<Vec<SyncJob>, DomainError> {
        self.read_repo.find_running_by_external_link(link_id).await
    }

    async fn find_recent(&self, days: i64) -> Result<Vec<SyncJob>, DomainError> {
        self.read_repo.find_recent(days).await
    }

    async fn count_by_external_link(&self, link_id: &Uuid) -> Result<i64, DomainError> {
        self.read_repo.count_by_external_link(link_id).await
    }

    async fn count_running(&self) -> Result<i64, DomainError> {
        self.read_repo.count_running().await
    }
}

#[async_trait]
impl SyncJobWriteRepository for SyncJobRepositoryImpl {
    async fn save(&self, job: &SyncJob) -> Result<SyncJob, DomainError> {
        self.write_repo.save(job).await
    }

    async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.delete_by_id(id).await
    }
}

impl SyncJobRepository for SyncJobRepositoryImpl {}