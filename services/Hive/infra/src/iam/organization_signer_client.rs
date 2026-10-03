//! HTTP s2s client for IAM organization-signer RPC (ADR-0306).

use async_trait::async_trait;
use hive_configuration::IamServiceConfig;
use hive_domain::port::service::{
    ConfigureOrganizationSignerRequest, IamOrganizationSignerClient, OrganizationSignerResponse,
    WorkloadIdentity,
};
use reqwest::Client;
use rustycog::core::error::DomainError;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;
use uuid::Uuid;

use super::StaticCredential;

const IAM_INTERNAL_CREDENTIAL_REF: &str = "iam-internal-token";

/// HTTP client using `[iam_service]` base_url + [`WorkloadIdentity`] for `x-iam-internal-token`.
#[derive(Clone)]
pub struct HttpIamOrganizationSignerClient {
    base_url: String,
    credential_ref: String,
    workload: Arc<dyn WorkloadIdentity>,
    client: Client,
}

impl HttpIamOrganizationSignerClient {
    /// # Errors
    ///
    /// Returns [`DomainError`] if `api_key` is empty after trim or the HTTP client cannot be built.
    pub fn new(
        base_url: impl AsRef<str>,
        api_key: impl Into<String>,
        timeout_seconds: u64,
    ) -> Result<Self, DomainError> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(DomainError::external_service_error(
                "iam_service",
                "iam_service.api_key must not be empty",
            ));
        }
        let workload = Arc::new(StaticCredential::from_pair(
            IAM_INTERNAL_CREDENTIAL_REF,
            api_key,
        )?) as Arc<dyn WorkloadIdentity>;
        Self::with_workload(
            base_url,
            IAM_INTERNAL_CREDENTIAL_REF,
            workload,
            timeout_seconds,
        )
    }

    /// # Errors
    ///
    /// Returns [`DomainError`] if the HTTP client cannot be built.
    pub fn with_workload(
        base_url: impl AsRef<str>,
        credential_ref: impl Into<String>,
        workload: Arc<dyn WorkloadIdentity>,
        timeout_seconds: u64,
    ) -> Result<Self, DomainError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .user_agent("Hive/iam-signer/1.0")
            .build()
            .map_err(|e| DomainError::external_service_error("iam_service", &e.to_string()))?;
        Ok(Self {
            base_url: base_url.as_ref().trim_end_matches('/').to_string(),
            credential_ref: credential_ref.into(),
            workload,
            client,
        })
    }

    /// # Errors
    ///
    /// Returns [`DomainError`] if `api_key` is empty or the HTTP client cannot be built.
    pub fn from_config(config: &IamServiceConfig) -> Result<Self, DomainError> {
        Self::new(
            &config.base_url,
            config.api_key.clone(),
            config.timeout_seconds,
        )
    }

    async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: Option<&impl Serialize>,
    ) -> Result<T, DomainError> {
        let cred = self.workload.resolve(&self.credential_ref).await?;
        let url = format!("{}{path}", self.base_url);
        debug!(%url, "IAM organization signer RPC");
        let mut req = self
            .client
            .post(&url)
            .header("x-iam-internal-token", &cred.secret)
            .header(reqwest::header::CONTENT_TYPE, "application/json");
        if let Some(b) = body {
            req = req.json(b);
        }
        let response = req
            .send()
            .await
            .map_err(|e| DomainError::external_service_error("iam_service", &e.to_string()))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| DomainError::external_service_error("iam_service", &e.to_string()))?;
        if !status.is_success() {
            return Err(DomainError::external_service_error(
                "iam_service",
                &format!("HTTP {status}: {text}"),
            ));
        }
        serde_json::from_str(&text).map_err(|e| {
            DomainError::external_service_error("iam_service", &format!("invalid JSON: {e}"))
        })
    }
}

use serde::Serialize;

#[async_trait]
impl IamOrganizationSignerClient for HttpIamOrganizationSignerClient {
    async fn configure_organization_signer(
        &self,
        org_id: Uuid,
        request: &ConfigureOrganizationSignerRequest,
    ) -> Result<OrganizationSignerResponse, DomainError> {
        self.post_json(
            &format!("/iam/internal/organizations/{org_id}/signer/configure"),
            Some(request),
        )
        .await
    }

    async fn test_organization_signer(
        &self,
        org_id: Uuid,
    ) -> Result<OrganizationSignerResponse, DomainError> {
        self.post_json(
            &format!("/iam/internal/organizations/{org_id}/signer/test"),
            None::<&()>,
        )
        .await
    }

    async fn rotate_organization_signer(
        &self,
        org_id: Uuid,
    ) -> Result<OrganizationSignerResponse, DomainError> {
        self.post_json(
            &format!("/iam/internal/organizations/{org_id}/signer/rotate"),
            None::<&()>,
        )
        .await
    }

    async fn disable_organization_signer(
        &self,
        org_id: Uuid,
    ) -> Result<OrganizationSignerResponse, DomainError> {
        self.post_json(
            &format!("/iam/internal/organizations/{org_id}/signer/disable"),
            None::<&()>,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_config_rejects_empty_api_key() {
        let config = IamServiceConfig {
            base_url: "http://127.0.0.1:8080".into(),
            api_key: "  ".into(),
            timeout_seconds: 10,
            workload: None,
        };
        assert!(HttpIamOrganizationSignerClient::from_config(&config).is_err());
    }

    #[test]
    fn from_config_accepts_non_empty_api_key() {
        let config = IamServiceConfig {
            base_url: "http://127.0.0.1:8080".into(),
            api_key: "iam-internal-test-token".into(),
            timeout_seconds: 10,
            workload: None,
        };
        assert!(HttpIamOrganizationSignerClient::from_config(&config).is_ok());
    }
}
