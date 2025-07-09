//! Template domain service for Telegraph

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::DomainError;
use crate::entity::{MessageTemplate, RenderedTemplate, CommunicationMode};
use crate::port::{TemplateService, TemplateRepository};

/// Template service implementation
pub struct TemplateServiceImpl {
    repository: Arc<dyn TemplateRepository>,
}

impl TemplateServiceImpl {
    /// Create a new template service
    pub fn new(repository: Arc<dyn TemplateRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl TemplateService for TemplateServiceImpl {
    async fn get_template(&self, name: &str, mode: &CommunicationMode) -> Result<MessageTemplate, DomainError> {
        self.repository
            .find_by_name_and_mode(name, mode)
            .await?
            .ok_or_else(|| DomainError::template_not_found(format!("Template '{}' for mode '{}' not found", name, mode)))
    }
    
    async fn render_template(
        &self,
        template_name: &str,
        mode: &CommunicationMode,
        variables: &HashMap<String, String>,
    ) -> Result<RenderedTemplate, DomainError> {
        let template = self.get_template(template_name, mode).await?;
        template.render(variables)
    }
    
    async fn create_template(&self, template: MessageTemplate) -> Result<(), DomainError> {
        // Check if template already exists
        if self.repository
            .find_by_name_and_mode(&template.name, &template.mode)
            .await?
            .is_some()
        {
            return Err(DomainError::configuration_error(
                format!("Template '{}' for mode '{}' already exists", template.name, template.mode)
            ));
        }
        
        self.repository.save(&template).await
    }
    
    async fn update_template(&self, id: Uuid, template: MessageTemplate) -> Result<(), DomainError> {
        // Verify template exists
        if self.repository.find_by_id(id).await?.is_none() {
            return Err(DomainError::template_not_found(format!("Template with ID '{}' not found", id)));
        }
        
        self.repository.update(&template).await
    }
    
    async fn delete_template(&self, id: Uuid) -> Result<(), DomainError> {
        self.repository.delete(id).await
    }
    
    async fn list_templates(&self, mode: Option<&CommunicationMode>) -> Result<Vec<MessageTemplate>, DomainError> {
        self.repository.list_active(mode).await
    }
    
    async fn template_exists(&self, name: &str, mode: &CommunicationMode) -> Result<bool, DomainError> {
        Ok(self.repository
            .find_by_name_and_mode(name, mode)
            .await?
            .is_some())
    }
} 