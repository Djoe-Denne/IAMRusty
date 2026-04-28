use async_trait::async_trait;
use reqwest::{Client, Response};
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use hive_configuration::ExternalProviderServiceConfig;
use hive_domain::{
    port::service::{
        ExternalMember, ExternalOrganizationInfo, ExternalProviderClient, ExternalProviderInfo,
    },
    RolePermission,
};
use rustycog_core::error::DomainError;

/// HTTP client implementation for External Provider Service
#[derive(Debug, Clone)]
pub struct HttpExternalProviderClient {
    base_url: String,
    api_key: Option<String>,
    client: Client,
}

impl HttpExternalProviderClient {
    /// Create a new HTTP client for external provider service
    pub fn new(
        base_url: String,
        api_key: Option<String>,
        timeout_seconds: u64,
        max_retries: u32,
    ) -> Result<Self, DomainError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .user_agent("Hive/1.0")
            .build()
            .map_err(|e| {
                DomainError::external_service_error("external_provider_service", &e.to_string())
            })?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            client,
        })
    }

    /// Create a new HTTP client from configuration
    pub fn from_config(config: &ExternalProviderServiceConfig) -> Result<Self, DomainError> {
        Self::new(
            config.base_url.clone(),
            config.api_key.clone(),
            config.timeout_seconds,
            config.max_retries,
        )
    }

    /// Build headers for HTTP requests
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        if let Some(ref api_key) = self.api_key {
            if let Ok(auth_value) =
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))
            {
                headers.insert(reqwest::header::AUTHORIZATION, auth_value);
            }
        }

        headers
    }

    /// Handle HTTP response and extract JSON
    async fn handle_response<T>(&self, response: Response) -> Result<T, DomainError>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();
        let response_text = response.text().await.map_err(|e| {
            DomainError::external_service_error("external_provider_service", &e.to_string())
        })?;

        debug!(
            "External provider service response: status={}, body={}",
            status, response_text
        );

        if !status.is_success() {
            error!(
                "External provider service error: status={}, body={}",
                status, response_text
            );
            return Err(DomainError::external_service_error(
                "external_provider_service",
                &format!("HTTP {}: {}", status, response_text),
            ));
        }

        serde_json::from_str(&response_text).map_err(|e| {
            error!("Failed to parse response: {}, body: {}", e, response_text);
            DomainError::external_service_error(
                "external_provider_service",
                &format!("Invalid JSON response: {}", e),
            )
        })
    }

    /// Make GET request to external provider service
    async fn get<T>(&self, endpoint: &str) -> Result<T, DomainError>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'));
        debug!("Making GET request to: {}", url);

        let response = self
            .client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await
            .map_err(|e| {
                error!("Failed to make GET request: {}", e);
                DomainError::external_service_error("external_provider_service", &e.to_string())
            })?;

        self.handle_response(response).await
    }

    /// Make POST request to external provider service
    async fn post<T>(&self, endpoint: &str, body: &Value) -> Result<T, DomainError>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'));
        debug!("Making POST request to: {} with body: {}", url, body);

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(body)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to make POST request: {}", e);
                DomainError::external_service_error("external_provider_service", &e.to_string())
            })?;

        self.handle_response(response).await
    }
}

#[async_trait]
impl ExternalProviderClient for HttpExternalProviderClient {
    async fn validate_config(
        &self,
        provider_source: &String,
        config: &Value,
    ) -> Result<(), DomainError> {
        info!("Validating config for provider: {:?}", provider_source);

        let request_body = serde_json::json!({
            "provider_source": provider_source.as_str(),
            "config": config
        });

        let _: Value = self.post("/config/validate", &request_body).await?;
        Ok(())
    }

    async fn test_connection(
        &self,
        provider_source: &String,
        config: &Value,
    ) -> Result<bool, DomainError> {
        info!("Testing connection for provider: {:?}", provider_source);

        let request_body = serde_json::json!({
            "provider_source": provider_source.as_str(),
            "config": config
        });

        let response: Value = self.post("/connection/test", &request_body).await?;

        response
            .get("connected")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| {
                DomainError::external_service_error(
                    "external_provider_service",
                    "Invalid response format for connection test",
                )
            })
    }

    async fn sync_members(
        &self,
        provider_source: &String,
        config: &Value,
    ) -> Result<Vec<ExternalMember>, DomainError> {
        info!("Syncing members for provider: {:?}", provider_source);
        self.get_members(provider_source, config).await
    }

    async fn get_organization_info(
        &self,
        provider_source: &String,
        config: &Value,
    ) -> Result<ExternalOrganizationInfo, DomainError> {
        info!(
            "Getting organization info for provider: {:?}",
            provider_source
        );

        let request_body = serde_json::json!({
            "provider_source": provider_source.as_str(),
            "config": config
        });

        self.post("/organization/info", &request_body).await
    }

    async fn get_members(
        &self,
        provider_source: &String,
        config: &Value,
    ) -> Result<Vec<ExternalMember>, DomainError> {
        info!("Getting members for provider: {:?}", provider_source);

        let request_body = serde_json::json!({
            "provider_source": provider_source.as_str(),
            "config": config
        });

        let response: Value = self.post("/members", &request_body).await?;

        response
            .get("members")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                DomainError::external_service_error(
                    "external_provider_service",
                    "Invalid response format for members list",
                )
            })?
            .iter()
            .map(|member_value| {
                serde_json::from_value(member_value.clone()).map_err(|e| {
                    error!("Failed to deserialize member: {}", e);
                    DomainError::external_service_error(
                        "external_provider_service",
                        &format!("Invalid member format: {}", e),
                    )
                })
            })
            .collect()
    }

    async fn is_member(
        &self,
        provider_source: &String,
        config: &Value,
        username: &str,
    ) -> Result<bool, DomainError> {
        info!(
            "Checking membership for user '{}' with provider: {:?}",
            username, provider_source
        );

        let request_body = serde_json::json!({
            "provider_source": provider_source.as_str(),
            "config": config,
            "username": username
        });

        let response: Value = self.post("/members/check", &request_body).await?;

        response
            .get("is_member")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| {
                DomainError::external_service_error(
                    "external_provider_service",
                    "Invalid response format for membership check",
                )
            })
    }
}
