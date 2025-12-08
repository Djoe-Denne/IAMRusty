use async_trait::async_trait;
use manifesto_domain::port::{ComponentInfo, ComponentServicePort};
use rustycog_core::error::DomainError;
use std::time::Duration;
use tracing::{debug, warn};

/// HTTP client for the component register service
pub struct ComponentServiceClient {
    base_url: String,
    client: reqwest::Client,
}

impl ComponentServiceClient {
    pub fn new(base_url: String, timeout_seconds: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { base_url, client }
    }
}

#[async_trait]
impl ComponentServicePort for ComponentServiceClient {
    async fn list_available_components(&self) -> Result<Vec<ComponentInfo>, DomainError> {
        debug!("Fetching available components from {}/api/components", self.base_url);

        // For MVP, return a mock list if the service is not available
        match self
            .client
            .get(format!("{}/api/components", self.base_url))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let components: Vec<ComponentInfo> = response
                        .json()
                        .await
                        .map_err(|e| DomainError::internal_error(&format!(
                            "Failed to parse component list: {}",
                            e
                        )))?;
                    
                    debug!("Successfully fetched {} components", components.len());
                    Ok(components)
                } else {
                    warn!("Component service returned error status: {}", response.status());
                    // Return mock data for MVP
                    Ok(Self::get_mock_components())
                }
            }
            Err(e) => {
                warn!("Failed to connect to component service: {}", e);
                // Return mock data for MVP when service is unavailable
                Ok(Self::get_mock_components())
            }
        }
    }
}

impl ComponentServiceClient {
    /// Mock component list for development/testing
    fn get_mock_components() -> Vec<ComponentInfo> {
        vec![
            ComponentInfo {
                component_type: "taskboard".to_string(),
                name: "Task Board".to_string(),
                description: Some("Kanban-style task management".to_string()),
                version: "1.0.0".to_string(),
                endpoint: "http://localhost:9001".to_string(),
            },
            ComponentInfo {
                component_type: "custom_forms".to_string(),
                name: "Custom Forms".to_string(),
                description: Some("Customizable form builder".to_string()),
                version: "1.0.0".to_string(),
                endpoint: "http://localhost:9002".to_string(),
            },
            ComponentInfo {
                component_type: "analytics".to_string(),
                name: "Analytics Dashboard".to_string(),
                description: Some("Project analytics and reporting".to_string()),
                version: "1.0.0".to_string(),
                endpoint: "http://localhost:9003".to_string(),
            },
        ]
    }
}

