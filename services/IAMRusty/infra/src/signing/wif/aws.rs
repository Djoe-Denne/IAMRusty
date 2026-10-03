//! AWS STS AssumeRoleWithWebIdentity WIF adapter (ADR-0307).

use super::{extract_xml_tag, read_subject_token};
use async_trait::async_trait;
use iam_configuration::AwsWorkloadConfig;
use iam_domain::error::DomainError;
use iam_domain::port::{WorkloadCredential, WorkloadIdentity};
use reqwest::Client;
use std::time::Duration;
use tracing::debug;

/// Exchanges a file-backed OIDC JWT for an AWS STS `SessionToken`.
pub struct AwsWif {
    client: Client,
    token_url: String,
    subject_token_file: String,
    role_arn: String,
    role_session_name: String,
    /// Retained for config completeness / future token audience checks (not STS form field).
    #[allow(dead_code)]
    audience: String,
}

impl AwsWif {
    /// Build an AWS WIF adapter from validated config.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the HTTP client cannot be built.
    pub fn new(config: &AwsWorkloadConfig) -> Result<Self, DomainError> {
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
            role_arn: config.role_arn.clone(),
            role_session_name: config.role_session_name.clone(),
            audience: config.audience.clone(),
        })
    }
}

#[async_trait]
impl WorkloadIdentity for AwsWif {
    /// # Errors
    ///
    /// Returns [`DomainError`] when the subject token cannot be read or STS exchange fails.
    async fn resolve(&self, _credential_ref: &str) -> Result<WorkloadCredential, DomainError> {
        let subject_token = read_subject_token(&self.subject_token_file)?;
        let form = [
            ("Action", "AssumeRoleWithWebIdentity"),
            ("Version", "2011-06-15"),
            ("RoleArn", self.role_arn.as_str()),
            ("RoleSessionName", self.role_session_name.as_str()),
            ("WebIdentityToken", subject_token.as_str()),
        ];
        debug!(url = %self.token_url, "AWS WIF AssumeRoleWithWebIdentity");
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
        let body = response.text().await.map_err(|e| {
            DomainError::external_service_error("workload_identity", &e.to_string())
        })?;
        if !status.is_success() {
            return Err(DomainError::external_service_error(
                "workload_identity",
                &format!("AWS STS HTTP {status}"),
            ));
        }
        let session_token = extract_xml_tag(&body, "SessionToken").ok_or_else(|| {
            DomainError::external_service_error(
                "workload_identity",
                "AWS STS response missing SessionToken",
            )
        })?;
        Ok(WorkloadCredential {
            secret: session_token,
        })
    }
}
