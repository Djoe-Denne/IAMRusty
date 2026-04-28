//! Template service port interfaces for Telegraph service

use async_trait::async_trait;
use std::collections::HashMap;

use crate::entity::{CommunicationMode, RenderedTemplate};
use crate::error::DomainError;

/// Port for template management and rendering
#[async_trait]
pub trait TemplateService: Send + Sync {
    /// Find template name for an event type and communication mode
    /// This method uses configuration to determine the correct template naming convention
    async fn find_template(
        &self,
        event_type: &str,
        mode: &CommunicationMode,
    ) -> Result<String, DomainError>;

    /// Render a template with variables
    async fn render_template(
        &self,
        template_name: &str,
        mode: &CommunicationMode,
        variables: &HashMap<String, String>,
    ) -> Result<RenderedTemplate, DomainError>;

    /// Check if a template exists
    async fn template_exists(
        &self,
        name: &str,
        mode: &CommunicationMode,
    ) -> Result<bool, DomainError>;
}
