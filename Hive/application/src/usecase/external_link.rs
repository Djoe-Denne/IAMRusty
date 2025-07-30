use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::{port::repository::ExternalLinkRepository, DomainError};
use hive_events::{event_types, ExternalLinkCreatedEvent};
use rustycog_events::{DomainEvent, MultiQueueEventPublisher};

use crate::{
    dto::{
        ConnectionTestResponse, CreateExternalLinkRequest, ExternalLinkListResponse,
        ExternalLinkResponse, ToggleSyncRequest, UpdateExternalLinkRequest,
    },
    ApplicationError,
};

#[async_trait]
pub trait ExternalLinkUseCase: Send + Sync {
    async fn create_link(
        &self,
        organization_id: Uuid,
        request: CreateExternalLinkRequest,
        user_id: Uuid,
    ) -> Result<ExternalLinkResponse, ApplicationError>;
}

pub struct ExternalLinkUseCaseImpl {
    link_repository: Arc<dyn ExternalLinkRepository>,
    event_publisher: Arc<MultiQueueEventPublisher<DomainError>>,
}

impl ExternalLinkUseCaseImpl {
    pub fn new(
        link_repository: Arc<dyn ExternalLinkRepository>,
        event_publisher: Arc<MultiQueueEventPublisher<DomainError>>,
    ) -> Self {
        Self {
            link_repository,
            event_publisher,
        }
    }
}

#[async_trait]
impl ExternalLinkUseCase for ExternalLinkUseCaseImpl {
    async fn create_link(
        &self,
        organization_id: Uuid,
        request: CreateExternalLinkRequest,
        user_id: Uuid,
    ) -> Result<ExternalLinkResponse, ApplicationError> {
        // TODO: Implement external link creation

        let link_id = Uuid::new_v4();

        // Publish event
        let event = ExternalLinkCreatedEvent {
            organization_id,
            organization_name: "Test Org".to_string(),
            external_link_id: link_id,
            provider_type: "github".to_string(),
            created_by_user_id: user_id,
            created_at: chrono::Utc::now(),
        };

        let domain_event: Box<dyn DomainEvent> = Box::new(rustycog_events::event::Event::new(
            event_types::EXTERNAL_LINK_CREATED,
            serde_json::to_value(event).map_err(|e| {
                ApplicationError::internal_error(&format!("Failed to serialize event: {}", e))
            })?,
            organization_id,
        ));

        self.event_publisher
            .publish(&domain_event)
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        Ok(ExternalLinkResponse {
            id: link_id,
            organization_id,
            provider_id: request.provider_id,
            sync_enabled: false,
        })
    }
}
