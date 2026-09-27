//! GCP STS token-exchange WIF adapter (ADR-0307).

use super::read_subject_token;
use async_trait::async_trait;
use hive_configuration::GcpWorkloadConfig;
use hive_domain::port::service::{WorkloadCredential, WorkloadIdentity};
use reqwest::Client;
use rustycog::core::error::DomainError;
use serde::Deserialize;
use std::time::Duration;
use tracing::debug;

/// Exchanges a file-backed OIDC JWT for a GCP `access_token`.
pub struct GcpWif {
    client: Client,
    token_url: String,
    subject_token_file: String,
    audience: String,
}

#[derive(Debug, Deserialize)]
struct GcpTokenResponse {
    access_token: String,
}

impl GcpWif {
    /// Build a GCP WIF adapter from validated config.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the HTTP client cannot be built.
    pub fn new(config: &GcpWorkloadConfig) -> Result<Self, DomainError> {
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
            audience: config.audience.clone(),
        })
    }
}

#[async_trait]
impl WorkloadIdentity for GcpWif {
    /// # Errors
    ///
    /// Returns [`DomainError`] when the subject token cannot be read or STS exchange fails.
    async fn resolve(&self, _credential_ref: &str) -> Result<WorkloadCredential, DomainError> {
        let subject_token = read_subject_token(&self.subject_token_file)?;
        let form = [
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:token-exchange",
            ),
            ("audience", self.audience.as_str()),
            (
                "requested_token_type",
                "urn:ietf:params:oauth:token-type:access_token",
            ),
            ("subject_token", subject_token.as_str()),
            ("subject_token_type", "urn:ietf:params:oauth:token-type:jwt"),
            ("scope", "https://www.googleapis.com/auth/cloud-platform"),
        ];
        debug!(url = %self.token_url, "GCP WIF token exchange");
        let response = self
            .client
            .post(&self.token_url)
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
                &format!("GCP STS HTTP {status}"),
            ));
        }
        let parsed: GcpTokenResponse = response.json().await.map_err(|e| {
            DomainError::external_service_error(
                "workload_identity",
                &format!("GCP STS invalid JSON: {e}"),
            )
        })?;
        if parsed.access_token.trim().is_empty() {
            return Err(DomainError::external_service_error(
                "workload_identity",
                "GCP STS response missing access_token",
            ));
        }
        Ok(WorkloadCredential {
            secret: parsed.access_token,
        })
    }
}
