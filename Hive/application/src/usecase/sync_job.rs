use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::{port::repository::SyncJobRepository, DomainError};
use hive_events::{HiveDomainEvent, SyncJobStartedEvent, SyncJobCompletedEvent};
use rustycog_events::EventPublisher;

use crate::{
    dto::{
        StartSyncJobRequest, SyncJobListResponse, SyncJobLogsResponse, SyncJobResponse,
        SyncJobStatusResponse,
    },
    ApplicationError,
};

#[async_trait]
pub trait SyncJobUseCase: Send + Sync {
    async fn start_sync_job(
        &self,
        organization_id: Uuid,
        request: StartSyncJobRequest,
    ) -> Result<SyncJobResponse, ApplicationError>;
}

pub struct SyncJobUseCaseImpl {
    sync_job_repository: Arc<dyn SyncJobRepository>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
}

impl SyncJobUseCaseImpl {
    pub fn new(
        sync_job_repository: Arc<dyn SyncJobRepository>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
    ) -> Self {
        Self {
            sync_job_repository,
            event_publisher,
        }
    }

    /// Publish sync job started event
    async fn publish_sync_job_started_event(
        &self,
        organization_id: Uuid,
        external_link_id: Uuid,
        sync_job_id: Uuid,
        job_type: String,
        started_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), ApplicationError> {
        let event = HiveDomainEvent::SyncJobStarted(SyncJobStartedEvent::new(
            organization_id,
            external_link_id,
            sync_job_id,
            job_type,
            started_at,
        ));

        self.event_publisher
            .publish(&event.into())
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        Ok(())
    }
}

#[async_trait]
impl SyncJobUseCase for SyncJobUseCaseImpl {
    async fn start_sync_job(
        &self,
        organization_id: Uuid,
        request: StartSyncJobRequest,
    ) -> Result<SyncJobResponse, ApplicationError> {
        // TODO: Implement sync job creation

        let job_id = Uuid::new_v4();
        let started_at = chrono::Utc::now();

        // Publish started event
        self.publish_sync_job_started_event(
            organization_id,
            request.external_link_id,
            job_id,
            request.job_type.clone(),
            started_at,
        )
        .await?;

        Ok(SyncJobResponse {
            id: job_id,
            organization_id: organization_id,
            external_link_id: request.external_link_id,
            job_type: request.job_type,
            status: "running".to_string(),
            items_processed: 0,
            items_created: 0,
            items_updated: 0,
            items_failed: 0,
            started_at,
            completed_at: None,
            error_message: None,
            details: None,
        })
    }
}
