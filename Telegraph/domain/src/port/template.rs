//! Template service port interfaces for Telegraph service

use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::DomainError;
use crate::entity::{MessageTemplate, RenderedTemplate, CommunicationMode};

/// Port for template management and rendering
#[async_trait]
pub trait TemplateService: Send + Sync {
    /// Get a template by name and communication mode
    async fn get_template(&self, name: &str, mode: &CommunicationMode) -> Result<MessageTemplate, DomainError>;
    
    /// Find template name for an event type and communication mode
    /// This method uses configuration to determine the correct template naming convention
    async fn find_template(&self, event_type: &str, mode: &CommunicationMode) -> Result<String, DomainError>;
    
    /// Render a template with variables
    async fn render_template(
        &self,
        template_name: &str,
        mode: &CommunicationMode,
        variables: &HashMap<String, String>,
    ) -> Result<RenderedTemplate, DomainError>;
    
    /// Create a new template
    async fn create_template(&self, template: MessageTemplate) -> Result<(), DomainError>;
    
    /// Update an existing template
    async fn update_template(&self, id: Uuid, template: MessageTemplate) -> Result<(), DomainError>;
    
    /// Delete a template
    async fn delete_template(&self, id: Uuid) -> Result<(), DomainError>;
    
    /// List all templates for a communication mode
    async fn list_templates(&self, mode: Option<&CommunicationMode>) -> Result<Vec<MessageTemplate>, DomainError>;
    
    /// Check if a template exists
    async fn template_exists(&self, name: &str, mode: &CommunicationMode) -> Result<bool, DomainError>;
}

/// Port for template repository operations
#[async_trait]
pub trait TemplateRepository: Send + Sync {
    /// Find template by name and mode
    async fn find_by_name_and_mode(&self, name: &str, mode: &CommunicationMode) -> Result<Option<MessageTemplate>, DomainError>;
    
    /// Find template by ID
    async fn find_by_id(&self, id: Uuid) -> Result<Option<MessageTemplate>, DomainError>;
    
    /// Save a template
    async fn save(&self, template: &MessageTemplate) -> Result<(), DomainError>;
    
    /// Update a template
    async fn update(&self, template: &MessageTemplate) -> Result<(), DomainError>;
    
    /// Delete a template
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    
    /// List all templates with optional mode filter
    async fn list(&self, mode: Option<&CommunicationMode>) -> Result<Vec<MessageTemplate>, DomainError>;
    
    /// List active templates only
    async fn list_active(&self, mode: Option<&CommunicationMode>) -> Result<Vec<MessageTemplate>, DomainError>;
} 