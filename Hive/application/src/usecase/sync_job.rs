use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::service::SyncService;
use rustycog_core::error::DomainError;
use hive_events::{HiveDomainEvent, SyncJobStartedEvent};
use rustycog_events::EventPublisher;

use crate::{
    dto::{
        StartSyncJobRequest, SyncJobResponse,
    },
    ApplicationError,
};

#[async_trait]
pub trait SyncJobUseCase: Send + Sync {
    async fn start_sync_job(
        &self,
        organization_id: Uuid,
        request: StartSyncJobRequest,
        requested_by_user_id: Uuid,
    ) -> Result<SyncJobResponse, ApplicationError>;
}

pub struct SyncJobUseCaseImpl {
    sync_service: Arc<dyn SyncService>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
}

impl SyncJobUseCaseImpl {
    pub fn new(
        sync_service: Arc<dyn SyncService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
    ) -> Self {
        Self {
            sync_service,
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
        requested_by_user_id: Uuid,
    ) -> Result<SyncJobResponse, ApplicationError> {
        let job = self.sync_service.start_sync_job(request.external_link_id, hive_domain::SyncJobType::from_str(&request.job_type).map_err(|e| ApplicationError::Domain(e))?, requested_by_user_id).await?;
        let started_at = chrono::Utc::now();

        // Publish started event
        self.publish_sync_job_started_event(
            organization_id,
            request.external_link_id,
            job.id,
            request.job_type.clone(),
            started_at,
        )
        .await?;

        Ok(SyncJobResponse {
            id: job.id,
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
