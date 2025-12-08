use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use manifesto_domain::{
    entity::ProjectComponent,
    service::{ComponentService, ProjectService},
    value_objects::ComponentStatus,
};
use manifesto_events::{
    ComponentAddedEvent, ComponentRemovedEvent, ComponentStatusChangedEvent, ManifestoDomainEvent,
};
use rustycog_core::error::DomainError;
use rustycog_events::EventPublisher;

use crate::{
    dto::{AddComponentRequest, ComponentListResponse, ComponentResponse, UpdateComponentRequest},
    ApplicationError,
};

#[async_trait]
pub trait ComponentUseCase: Send + Sync {
    async fn add_component(
        &self,
        project_id: Uuid,
        request: &AddComponentRequest,
        user_id: Uuid,
    ) -> Result<ComponentResponse, ApplicationError>;

    async fn get_component(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<ComponentResponse, ApplicationError>;

    async fn list_components(
        &self,
        project_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<ComponentListResponse, ApplicationError>;

    async fn update_component_status(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        request: &UpdateComponentRequest,
        user_id: Uuid,
    ) -> Result<ComponentResponse, ApplicationError>;

    async fn remove_component(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError>;
}

pub struct ComponentUseCaseImpl {
    component_service: Arc<dyn ComponentService>,
    project_service: Arc<dyn ProjectService>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
}

impl ComponentUseCaseImpl {
    pub fn new(
        component_service: Arc<dyn ComponentService>,
        project_service: Arc<dyn ProjectService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
    ) -> Self {
        Self {
            component_service,
            project_service,
            event_publisher,
        }
    }

    fn component_to_response(&self, component: &ProjectComponent) -> ComponentResponse {
        ComponentResponse {
            id: component.id,
            component_type: component.component_type.clone(),
            status: component.status.as_str().to_string(),
            endpoint: None, // TODO: Get from component service
            access_token: None, // TODO: Generate component-scoped JWT
            added_at: component.added_at,
            configured_at: component.configured_at,
            activated_at: component.activated_at,
            disabled_at: component.disabled_at,
        }
    }
}

#[async_trait]
impl ComponentUseCase for ComponentUseCaseImpl {
    async fn add_component(
        &self,
        project_id: Uuid,
        request: &AddComponentRequest,
        user_id: Uuid,
    ) -> Result<ComponentResponse, ApplicationError> {
        // Ensure project exists
        let _project = self.project_service.get_project(&project_id).await?;

        // Validate component type exists in component service
        self.component_service
            .validate_component_type(&request.component_type)
            .await?;

        // Check uniqueness
        self.component_service
            .validate_unique_component(&project_id, &request.component_type)
            .await?;

        // Create component
        let component = ProjectComponent::new(project_id, request.component_type.clone())?;

        // Save through service (which uses repository)
        let created = self.component_service.add_component(component).await?;

        // Publish ComponentAdded event
        let event = ManifestoDomainEvent::ComponentAdded(ComponentAddedEvent::new(
            project_id,
            created.id,
            created.component_type.clone(),
            user_id,
            created.added_at,
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish ComponentAdded event: {:?}", e);
        }

        Ok(self.component_to_response(&created))
    }

    async fn get_component(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        _user_id: Option<Uuid>,
    ) -> Result<ComponentResponse, ApplicationError> {
        let component = self
            .component_service
            .get_component(&component_id)
            .await?;

        if component.project_id != project_id {
            return Err(ApplicationError::NotFound(format!("ProjectComponent not found for project {}", project_id)));
        }

        Ok(self.component_to_response(&component))
    }

    async fn list_components(
        &self,
        project_id: Uuid,
        _user_id: Option<Uuid>,
    ) -> Result<ComponentListResponse, ApplicationError> {
        let components = self
            .component_service
            .list_components(&project_id)
            .await?;

        let data: Vec<ComponentResponse> = components
            .iter()
            .map(|c| self.component_to_response(c))
            .collect();

        Ok(ComponentListResponse { data })
    }

    async fn update_component_status(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        request: &UpdateComponentRequest,
        user_id: Uuid,
    ) -> Result<ComponentResponse, ApplicationError> {
        let mut component = self
            .component_service
            .get_component(&component_id)
            .await?;

        if component.project_id != project_id {
            return Err(ApplicationError::NotFound(format!("ProjectComponent not found for project {}", project_id)));
        }

        let new_status = ComponentStatus::from_str(&request.status)
            .map_err(ApplicationError::from)?;

        let old_status = component.status;

        // Transition status (validates transition)
        component.transition_status(new_status)
            .map_err(ApplicationError::from)?;

        // Update through service
        let updated = self.component_service.update_component(component).await?;

        // Publish ComponentStatusChanged event
        let event = ManifestoDomainEvent::ComponentStatusChanged(ComponentStatusChangedEvent::new(
            project_id,
            updated.id,
            updated.component_type.clone(),
            old_status.as_str().to_string(),
            updated.status.as_str().to_string(),
            user_id,
            Utc::now(),
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish ComponentStatusChanged event: {:?}", e);
        }

        Ok(self.component_to_response(&updated))
    }

    async fn remove_component(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let component = self
            .component_service
            .get_component(&component_id)
            .await?;

        if component.project_id != project_id {
            return Err(ApplicationError::NotFound(format!("ProjectComponent not found for project {}", project_id)));
        }

        let component_type_str = component.component_type.clone();

        self.component_service.remove_component(&component.id).await?;

        // Publish ComponentRemoved event
        let event = ManifestoDomainEvent::ComponentRemoved(ComponentRemovedEvent::new(
            project_id,
            component_id,
            component_type_str,
            user_id,
            Utc::now(),
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish ComponentRemoved event: {:?}", e);
        }

        Ok(())
    }
}

