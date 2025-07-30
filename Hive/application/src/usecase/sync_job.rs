use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::{port::repository::SyncJobRepository, DomainError};
use hive_events::{event_types, SyncJobCompletedEvent, SyncJobStartedEvent};
use rustycog_events::{DomainEvent, MultiQueueEventPublisher};

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
    event_publisher: Arc<MultiQueueEventPublisher<DomainError>>,
}

impl SyncJobUseCaseImpl {
    pub fn new(
        sync_job_repository: Arc<dyn SyncJobRepository>,
        event_publisher: Arc<MultiQueueEventPublisher<DomainError>>,
    ) -> Self {
        Self {
            sync_job_repository,
            event_publisher,
        }
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

        // Publish started event
        let event = SyncJobStartedEvent {
            organization_id,
            external_link_id: request.external_link_id,
            sync_job_id: job_id,
            job_type: request.job_type.clone(),
            started_at: chrono::Utc::now(),
        };

        let domain_event: Box<dyn DomainEvent> = Box::new(rustycog_events::event::Event::new(
            event_types::SYNC_JOB_STARTED,
            serde_json::to_value(event).map_err(|e| {
                ApplicationError::internal_error(&format!("Failed to serialize event: {}", e))
            })?,
            organization_id,
        ));

        self.event_publisher
            .publish(&domain_event)
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

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
            started_at: chrono::Utc::now(),
            completed_at: None,
            error_message: None,
            details: None,
        })
    }
}
