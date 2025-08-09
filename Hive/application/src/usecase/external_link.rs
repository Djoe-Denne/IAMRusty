use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::{DomainError, ExternalProviderService};
use hive_events::{HiveDomainEvent, ExternalLinkCreatedEvent};
use rustycog_events::EventPublisher;

use crate::{
    dto::{
        ConnectionTestResponse, CreateExternalLinkRequest, ExternalLinkResponse,
    },
    ApplicationError,
};

#[async_trait]
pub trait ExternalLinkUseCase: Send + Sync {
    /**
     * Create a new external link
     * 
     * @param organization_id - The ID of the organization
     * @param request - The request to create the external link
     * @param user_id - The ID of the user creating the external link
     */
    async fn create_link(
        &self,
        organization_id: Uuid,
        request: &CreateExternalLinkRequest,
        user_id: Uuid,
    ) -> Result<ExternalLinkResponse, ApplicationError>;

    /**
     * Delete an external link
     * 
     * @param organization_id - The ID of the organization
     * @param link_id - The ID of the external link
     * @param user_id - The ID of the user deleting the external link
     */
    async fn delete_link(
        &self,
        organization_id: Uuid,
        link_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError>;

    /**
     * Test connection to external provider
     * 
     * @param provider_id - The ID of the external provider
     * @param provider_config - Configuration for the connection
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
}

impl ExternalLinkUseCaseImpl {
    pub fn new(
        external_provider_service: Arc<dyn ExternalProviderService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
    ) -> Self {
        Self {
            external_provider_service,
            event_publisher,
        }
    }

    /// Publish external link created event
    async fn publish_external_link_created_event(
        &self,
        organization_id: Uuid,
        organization_name: String,
        external_link_id: Uuid,
        provider_type: String,
        created_by_user_id: Uuid,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), ApplicationError> {
        let event = HiveDomainEvent::ExternalLinkCreated(ExternalLinkCreatedEvent::new(
            organization_id,
            organization_name.to_string(),
            external_link_id,
            provider_type.to_string(),
            created_by_user_id,
            created_at,
        ));

        self.event_publisher
            .publish(&event.into())
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        Ok(())
    }
}

#[async_trait]
impl ExternalLinkUseCase for ExternalLinkUseCaseImpl {
    async fn create_link(
        &self,
        organization_id: Uuid,
        request: &CreateExternalLinkRequest,
        user_id: Uuid,
    ) -> Result<ExternalLinkResponse, ApplicationError> {
        let external_link = self.external_provider_service.link_organization(organization_id, request.provider_id, &request.provider_config, user_id).await
        .map_err(|e| ApplicationError::Domain(e))?;

        let provider_source = external_link.provider_source.clone().unwrap();

        // Publish external link created event
        self.publish_external_link_created_event(
            organization_id,
            external_link.organization_name.unwrap_or_default(),
            external_link.id,
            provider_source.clone(),
            user_id,
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
            last_sync_status: external_link.last_sync_status.map(|s| s.as_str().to_string()),
            sync_error: external_link.sync_error,
            created_at: external_link.created_at,
            updated_at: external_link.updated_at,
        })
    }

    async fn delete_link(
        &self,
        organization_id: Uuid,
        link_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.external_provider_service.unlink_organization(organization_id, link_id, user_id).await
        .map_err(|e| ApplicationError::Domain(e))?;

        Ok(())
    }

    async fn test_connection(
        &self,
        organization_id: Uuid,
        provider_id: Uuid,
        provider_config: &serde_json::Value,
    ) -> Result<ConnectionTestResponse, ApplicationError> {
        let result = self.external_provider_service.test_connection(provider_id, provider_config).await
        .map_err(|e| ApplicationError::Domain(e))?;
        
        Ok(ConnectionTestResponse {
            success: result,
            message: "Connection test successful".to_string(),
            details: None,
        })
    }
}
