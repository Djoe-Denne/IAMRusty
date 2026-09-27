//! Azure AD client-assertion WIF adapter (ADR-0307).

use super::read_subject_token;
use async_trait::async_trait;
use hive_configuration::AzureWorkloadConfig;
use hive_domain::port::service::{WorkloadCredential, WorkloadIdentity};
use reqwest::Client;
use rustycog::core::error::DomainError;
use serde::Deserialize;
use std::time::Duration;
use tracing::debug;

/// Exchanges a file-backed client assertion JWT for an Azure `access_token`.
pub struct AzureWif {
    client: Client,
    token_url: String,
    subject_token_file: String,
    tenant_id: String,
    client_id: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct AzureTokenResponse {
    access_token: String,
}

impl AzureWif {
    /// Build an Azure WIF adapter from validated config.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the HTTP client cannot be built.
    pub fn new(config: &AzureWorkloadConfig) -> Result<Self, DomainError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| {
                DomainError::external_service_error("workload_identity", &e.to_string())
            })?;
        Ok(Self {
            client,
            token_url: config.token_url.clone(),
            subject_token_file: config.subject_token_file.clone(),
            tenant_id: config.tenant_id.clone(),
            client_id: config.client_id.clone(),
            scope: config.scope.clone(),
        })
    }

    fn endpoint(&self) -> String {
        format!(
            "{}/{}/oauth2/v2.0/token",
            self.token_url.trim_end_matches('/'),
            self.tenant_id.trim_matches('/')
        )
    }
}

#[async_trait]
impl WorkloadIdentity for AzureWif {
    /// # Errors
    ///
    /// Returns [`DomainError`] when the assertion cannot be read or token exchange fails.
    async fn resolve(&self, _credential_ref: &str) -> Result<WorkloadCredential, DomainError> {
        let assertion = read_subject_token(&self.subject_token_file)?;
        let url = self.endpoint();
        let form = [
            ("client_id", self.client_id.as_str()),
            (
                "client_assertion_type",
                "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
            ),
            ("client_assertion", assertion.as_str()),
            ("grant_type", "client_credentials"),
            ("scope", self.scope.as_str()),
        ];
        debug!(%url, "Azure WIF client_assertion token");
        let response = self
            .client
            .post(&url)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .form(&form)
            .send()
            .await
            .map_err(|e| {
                DomainError::external_service_error("workload_identity", &e.to_string())
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(DomainError::external_service_error(
                "workload_identity",
                &format!("Azure token HTTP {status}"),
            ));
        }
        let parsed: AzureTokenResponse = response.json().await.map_err(|e| {
            DomainError::external_service_error(
                "workload_identity",
                &format!("Azure token invalid JSON: {e}"),
            )
        })?;
        if parsed.access_token.trim().is_empty() {
            return Err(DomainError::external_service_error(
                "workload_identity",
                "Azure token response missing access_token",
            ));
        }
        Ok(WorkloadCredential {
            secret: parsed.access_token,
        })
    }
}
