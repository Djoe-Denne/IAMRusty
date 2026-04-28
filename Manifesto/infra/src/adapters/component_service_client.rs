use async_trait::async_trait;
use manifesto_domain::port::{ComponentInfo, ComponentServicePort};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use rustycog_core::error::DomainError;
use std::time::Duration;
use tracing::{debug, error};

/// HTTP client for the component register service
pub struct ComponentServiceClient {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl ComponentServiceClient {
    pub fn new(
        base_url: String,
        api_key: Option<String>,
        timeout_seconds: u64,
    ) -> Result<Self, DomainError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .user_agent("Manifesto/1.0")
            .build()
            .map_err(|e| {
                DomainError::external_service_error("component_service", &e.to_string())
            })?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            client,
        })
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Some(api_key) = self.api_key.as_ref() {
            if let Ok(auth_value) = HeaderValue::from_str(&format!("Bearer {api_key}")) {
                headers.insert(AUTHORIZATION, auth_value);
            }
        }

        headers
    }
}

#[async_trait]
impl ComponentServicePort for ComponentServiceClient {
    async fn list_available_components(&self) -> Result<Vec<ComponentInfo>, DomainError> {
        debug!(
            "Fetching available components from {}/api/components",
            self.base_url
        );
        let response = self
            .client
            .get(format!("{}/api/components", self.base_url))
            .headers(self.build_headers())
            .send()
            .await
            .map_err(|e| {
                error!("Failed to connect to component service: {}", e);
                DomainError::external_service_error("component_service", &e.to_string())
            })?;

        let status = response.status();
        if !status.is_success() {
            let response_body = response.text().await.unwrap_or_default();
            error!(
                "Component service returned error status {} with body {}",
                status, response_body
            );
            return Err(DomainError::external_service_error(
                "component_service",
                &format!("HTTP {status}: {response_body}"),
            ));
        }

        let components: Vec<ComponentInfo> = response.json().await.map_err(|e| {
            DomainError::external_service_error(
                "component_service",
                &format!("Failed to parse component list: {e}"),
            )
        })?;

        debug!("Successfully fetched {} components", components.len());
        Ok(components)
    }
}
