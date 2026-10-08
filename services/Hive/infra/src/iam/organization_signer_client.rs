//! HTTP s2s client for IAM organization-signer RPC (ADR-0306).

use async_trait::async_trait;
use hive_configuration::IamServiceConfig;
use hive_domain::port::service::{
    ConfigureOrganizationSignerRequest, IamOrganizationSignerClient, OrganizationSignerResponse,
    WorkloadIdentity,
};
use reqwest::Client;
use rustycog::core::error::DomainError;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;
use uuid::Uuid;

use super::StaticCredential;

const IAM_INTERNAL_CREDENTIAL_REF: &str = "iam-internal-token";

/// HTTP client using `iam_service` `base_url` + [`WorkloadIdentity`] for `x-iam-internal-token`.
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

    async fn post_json<T, B>(&self, path: &str, body: Option<&B>) -> Result<T, DomainError>
    where
        T: DeserializeOwned + Send,
        B: Serialize + Send + Sync,
    {
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
        if status == reqwest::StatusCode::BAD_REQUEST {
            return Err(DomainError::external_service_error(
                "iam_service",
                "signing_invalid_input",
            ));
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(DomainError::external_service_error(
                "iam_service",
                "signing_admission_throttled",
            ));
        }
        if status == reqwest::StatusCode::CONFLICT {
            return Err(DomainError::external_service_error(
                "iam_service",
                "signing_epoch_conflict",
            ));
        }
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

    #[tokio::test]
    async fn outbound400_is_opaque_invalid_input_while409_429_and_provider500_remain_distinct() {
        use wiremock::{
            matchers::{header, method, path},
            Mock, MockServer, ResponseTemplate,
        };
        for (status, marker) in [
            (400, "signing_invalid_input"),
            (409, "signing_epoch_conflict"),
            (429, "signing_admission_throttled"),
            (500, ""),
        ] {
            for version in [None, Some(0)] {
                let server = MockServer::start().await;
                let org = Uuid::new_v4();
                Mock::given(method("POST"))
                    .and(path(format!(
                        "/iam/internal/organizations/{org}/signer/configure"
                    )))
                    .and(header("x-iam-internal-token", "explicit-test-credential"))
                    .respond_with(
                        ResponseTemplate::new(status)
                            .set_body_string("must-not-expose-invalid-material-details"),
                    )
                    .expect(1)
                    .mount(&server)
                    .await;
                let client = HttpIamOrganizationSignerClient::new(
                    server.uri(),
                    "explicit-test-credential",
                    5,
                )
                .unwrap();
                let err = client
                    .configure_organization_signer(
                        org,
                        &ConfigureOrganizationSignerRequest {
                            provider_type: "openbao_transit".into(),
                            provider_key_ref: format!("org-{org}-key"),
                            provider_key_version: version,
                            credential_ref: Some(format!("org-{org}-credential")),
                            public_key: "public-material".into(),
                            org_slug: "db-org".into(),
                        },
                    )
                    .await
                    .unwrap_err();
                if status == 500 {
                    assert!(
                        matches!(err,DomainError::ExternalServiceError {ref message,..} if message!="signing_invalid_input")
                    );
                } else {
                    assert!(
                        matches!(err,DomainError::ExternalServiceError {ref service,ref message} if service=="iam_service" && message==marker)
                    );
                }
                let requests = server.received_requests().await.unwrap();
                assert_eq!(requests.len(), 1);
                let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
                assert_eq!(
                    body.get("provider_key_version")
                        .and_then(serde_json::Value::as_u64),
                    version.map(u64::from)
                );
                assert_eq!(body["org_slug"], "db-org");
            }
        }
    }
}
