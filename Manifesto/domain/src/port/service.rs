use async_trait::async_trait;
use rustycog_core::error::DomainError;
use serde::{Deserialize, Serialize};

/// Component information from the component register service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    pub component_type: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub endpoint: String,
}

/// Port for interacting with the component service/register
#[async_trait]
pub trait ComponentServicePort: Send + Sync {
    /// List all available component types
    async fn list_available_components(&self) -> Result<Vec<ComponentInfo>, DomainError>;

    /// Check if a component type exists
    async fn component_exists(&self, component_type: &str) -> Result<bool, DomainError> {
        let components = self.list_available_components().await?;
        Ok(components
            .iter()
            .any(|c| c.component_type == component_type))
    }
}
