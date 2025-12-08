use async_trait::async_trait;
use rustycog_core::error::DomainError;
use std::sync::Arc;
use uuid::Uuid;

use crate::entity::ProjectComponent;
use crate::port::{ComponentRepository, ComponentServicePort};
use crate::service::PermissionService;
use crate::value_objects::ComponentStatus;

#[async_trait]
pub trait ComponentService: Send + Sync {
    async fn get_component(&self, id: &Uuid) -> Result<ProjectComponent, DomainError>;
    
    async fn get_component_by_type(
        &self,
        project_id: &Uuid,
        component_type: &str,
    ) -> Result<ProjectComponent, DomainError>;
    
    async fn add_component(&self, component: ProjectComponent) -> Result<ProjectComponent, DomainError>;
    
    async fn update_component(&self, component: ProjectComponent) -> Result<ProjectComponent, DomainError>;
    
    async fn remove_component(&self, id: &Uuid) -> Result<(), DomainError>;
    
    async fn list_components(&self, project_id: &Uuid) -> Result<Vec<ProjectComponent>, DomainError>;
    
    async fn validate_component_type(&self, component_type: &str) -> Result<(), DomainError>;
    
    async fn validate_unique_component(
        &self,
        project_id: &Uuid,
        component_type: &str,
    ) -> Result<(), DomainError>;
}

pub struct ComponentServiceImpl<CR, CSP>
where
    CR: ComponentRepository,
    CSP: ComponentServicePort,
{
    component_repo: Arc<CR>,
    component_service_port: Arc<CSP>,
    permission_service: Arc<dyn PermissionService>,
}

impl<CR, CSP> ComponentServiceImpl<CR, CSP>
where
    CR: ComponentRepository,
    CSP: ComponentServicePort,
{
    pub fn new(
        component_repo: Arc<CR>,
        component_service_port: Arc<CSP>,
        permission_service: Arc<dyn PermissionService>,
    ) -> Self {
        Self {
            component_repo,
            component_service_port,
            permission_service,
        }
    }
}

#[async_trait]
impl<CR, CSP> ComponentService for ComponentServiceImpl<CR, CSP>
where
    CR: ComponentRepository,
    CSP: ComponentServicePort,
{
    async fn get_component(&self, id: &Uuid) -> Result<ProjectComponent, DomainError> {
        self.component_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ProjectComponent", &id.to_string()))
    }

    async fn get_component_by_type(
        &self,
        project_id: &Uuid,
        component_type: &str,
    ) -> Result<ProjectComponent, DomainError> {
        self.component_repo
            .find_by_project_and_type(project_id, component_type)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found(
                    "ProjectComponent",
                    &format!("{}/{}", project_id, component_type),
                )
            })
    }

    async fn add_component(&self, component: ProjectComponent) -> Result<ProjectComponent, DomainError> {
        // Validate component
        component.validate()?;
        
        // Check uniqueness
        self.validate_unique_component(&component.project_id, &component.component_type).await?;
        
        // Create resource for this component type
        self.permission_service
            .create_component_resource(&component.component_type)
            .await?;
        
        // Save to repository
        self.component_repo.save(&component).await
    }

    async fn update_component(&self, component: ProjectComponent) -> Result<ProjectComponent, DomainError> {
        // Validate component
        component.validate()?;
        
        // Save to repository
        self.component_repo.save(&component).await
    }

    async fn remove_component(&self, id: &Uuid) -> Result<(), DomainError> {
        // Get component to retrieve its type
        let component = self.get_component(id).await?;
        
        // Delete the component
        self.component_repo.delete(id).await?;
        
        // Delete resource for this component type (cascade deletes role_permissions)
        self.permission_service
            .delete_resource(&component.component_type)
            .await?;
        
        Ok(())
    }

    async fn list_components(&self, project_id: &Uuid) -> Result<Vec<ProjectComponent>, DomainError> {
        self.component_repo.find_by_project(project_id).await
    }

    async fn validate_component_type(&self, component_type: &str) -> Result<(), DomainError> {
        let exists = self
            .component_service_port
            .component_exists(component_type)
            .await?;

        if !exists {
            return Err(DomainError::invalid_input(&format!(
                "Component type '{}' does not exist in the component register",
                component_type
            )));
        }

        Ok(())
    }

    async fn validate_unique_component(
        &self,
        project_id: &Uuid,
        component_type: &str,
    ) -> Result<(), DomainError> {
        let exists = self
            .component_repo
            .exists_by_project_and_type(project_id, component_type)
            .await?;

        if exists {
            return Err(DomainError::resource_already_exists(
                "Component",
                component_type,
            ));
        }

        Ok(())
    }
}

