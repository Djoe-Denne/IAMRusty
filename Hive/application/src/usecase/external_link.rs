use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::ExternalProviderService;
use hive_events::{ExternalLinkCreatedEvent, HiveDomainEvent};
use rustycog_core::error::DomainError;
use rustycog_events::{DomainEvent, EventPublisher};

use crate::{
    dto::{ConnectionTestResponse, CreateExternalLinkRequest, ExternalLinkResponse},
    ApplicationError, HiveOutboxUnitOfWork,
};

#[async_trait]
pub trait ExternalLinkUseCase: Send + Sync {
    /**
     * Create a new external link
     *
     * @param `organization_id` - The ID of the organization
     * @param request - The request to create the external link
     */
    async fn create_link(
        &self,
        organization_id: Uuid,
        request: &CreateExternalLinkRequest,
    ) -> Result<ExternalLinkResponse, ApplicationError>;

    /**
     * Delete an external link
     *
     * @param `organization_id` - The ID of the organization
     * @param `link_id` - The ID of the external link
     */
    async fn delete_link(
        &self,
        organization_id: Uuid,
        link_id: Uuid,
    ) -> Result<(), ApplicationError>;

    /**
     * Test connection to external provider
     *
     * @param `provider_id` - The ID of the external provider
     * @param `provider_config` - Configuration for the connection
     */
    async fn test_connection(
        &self,
        organization_id: Uuid,
        provider_id: Uuid,
        provider_config: &serde_json::Value,
    ) -> Result<ConnectionTestResponse, ApplicationError>;
}

pub struct ExternalLinkUseCaseImpl {
    external_provider_service: Arc<dyn ExternalProviderService>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
    outbox_unit_of_work: Option<Arc<dyn HiveOutboxUnitOfWork>>,
}

impl ExternalLinkUseCaseImpl {
    pub fn new(
        external_provider_service: Arc<dyn ExternalProviderService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
    ) -> Self {
        Self {
            external_provider_service,
            event_publisher,
            outbox_unit_of_work: None,
        }
    }

    pub fn new_with_outbox_unit_of_work(
        external_provider_service: Arc<dyn ExternalProviderService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
        outbox_unit_of_work: Arc<dyn HiveOutboxUnitOfWork>,
    ) -> Self {
        Self {
            external_provider_service,
            event_publisher,
            outbox_unit_of_work: Some(outbox_unit_of_work),
        }
    }

    async fn record_or_publish_event(
        &self,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<(), ApplicationError> {
        if let Some(outbox_unit_of_work) = &self.outbox_unit_of_work {
            outbox_unit_of_work.record_event(event).await
        } else {
            self.event_publisher
                .publish(&event)
                .await
                .map_err(ApplicationError::Domain)
        }
    }

    /// Publish external link created event
    async fn publish_external_link_created_event(
        &self,
        organization_id: Uuid,
        organization_name: String,
        external_link_id: Uuid,
        provider_type: String,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), ApplicationError> {
        let event = HiveDomainEvent::ExternalLinkCreated(ExternalLinkCreatedEvent::new(
            organization_id,
            organization_name.clone(),
            external_link_id,
            provider_type.clone(),
            created_at,
        ));

        self.record_or_publish_event(event.into()).await
    }
}

#[async_trait]
impl ExternalLinkUseCase for ExternalLinkUseCaseImpl {
    async fn create_link(
        &self,
        organization_id: Uuid,
        request: &CreateExternalLinkRequest,
    ) -> Result<ExternalLinkResponse, ApplicationError> {
        let external_link = self
            .external_provider_service
            .link_organization(
                organization_id,
                request.provider_id,
                &request.provider_config,
            )
            .await
            .map_err(ApplicationError::Domain)?;

        let provider_source = external_link.provider_source.clone().unwrap();

        // Publish external link created event
        self.publish_external_link_created_event(
            organization_id,
            external_link.organization_name.unwrap_or_default(),
            external_link.id,
            provider_source.clone(),
            external_link.created_at,
        )
        .await?;

        Ok(ExternalLinkResponse {
            id: external_link.id,
            organization_id,
            provider_id: request.provider_id,
            sync_enabled: false,
            provider_name: provider_source,
            provider_config: external_link.provider_config,
            sync_settings: external_link.sync_settings,
            last_sync_at: external_link.last_sync_at,
            last_sync_status: external_link
                .last_sync_status
                .map(|s| s.as_str().to_string()),
            sync_error: external_link.sync_error,
            created_at: external_link.created_at,
            updated_at: external_link.updated_at,
        })
    }

    async fn delete_link(
        &self,
        organization_id: Uuid,
        link_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.external_provider_service
            .unlink_organization(organization_id, link_id)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(())
    }

    async fn test_connection(
        &self,
        _organization_id: Uuid,
        provider_id: Uuid,
        provider_config: &serde_json::Value,
    ) -> Result<ConnectionTestResponse, ApplicationError> {
        let result = self
            .external_provider_service
            .test_connection(provider_id, provider_config)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(ConnectionTestResponse {
            success: result,
            message: "Connection test successful".to_string(),
            details: None,
        })
    }
}
