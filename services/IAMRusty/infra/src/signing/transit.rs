//! OpenBao Transit SigningProvider — Sign only, never export private key material.

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use iam_domain::error::DomainError;
use iam_domain::port::{SigningCapabilities, SigningProvider, WorkloadIdentity};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tracing::debug;

pub use iam_domain::entity::signing_key::FORBIDDEN_TRANSIT_KEY_NAME;

/// OpenBao / Vault Transit `POST /v1/transit/sign/{name}` adapter.
///
/// Never uses Cosign/`apparatus-p4-cosign` paths. Credentials come from
/// [`WorkloadIdentity`] (typically [`crate::signing::StaticCredential`]).
pub struct TransitSigningProvider {
    client: Client,
    base_url: String,
    key_name: String,
    token_ref: String,
    workload: Arc<dyn WorkloadIdentity>,
    /// Optional public key PEM published in JWKS (fetched from Transit when absent).
    public_key_pem: Option<String>,
}

impl TransitSigningProvider {
    /// Create a Transit Sign client.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the HTTP client cannot be built or the key name is forbidden.
    pub fn new(
        base_url: impl Into<String>,
        key_name: impl Into<String>,
        token_ref: impl Into<String>,
        workload: Arc<dyn WorkloadIdentity>,
        public_key_pem: Option<String>,
    ) -> Result<Self, DomainError> {
        let key_name = key_name.into();
        iam_domain::entity::signing_key::require_transit_key_name(&key_name)?;
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| {
                DomainError::external_service_error(
                    "openbao_transit",
                    "HTTP client initialization failed",
                )
            })?;
        let base_url = base_url.into();
        let parsed = reqwest::Url::parse(&base_url).map_err(|_| DomainError::InvalidToken)?;
        if !matches!(parsed.scheme(), "http" | "https")
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(DomainError::InvalidToken);
        }
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            key_name,
            token_ref: token_ref.into(),
            workload,
            public_key_pem,
        })
    }

    fn endpoint(&self, operation: &str) -> Result<reqwest::Url, DomainError> {
        let mut url = reqwest::Url::parse(&self.base_url).map_err(|_| DomainError::InvalidToken)?;
        url.path_segments_mut()
            .map_err(|_| DomainError::InvalidToken)?
            .pop_if_empty()
            .push("v1")
            .push("transit")
            .push(operation)
            .push(&self.key_name);
        Ok(url)
    }

    /// `POST /v1/transit/keys/{name}` with `type=rsa-2048`, `exportable=false`.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] on HTTP or auth failure.
    pub async fn create_rsa2048_key(&self) -> Result<(), DomainError> {
        let cred = self.workload.resolve(&self.token_ref).await?;
        let url = self.endpoint("keys")?;
        let body = serde_json::json!({
            "type": "rsa-2048",
            "exportable": false,
        });
        debug!("OpenBao Transit create key");
        let response = self
            .client
            .post(url)
            .header("X-Vault-Token", &cred.secret)
            .json(&body)
            .send()
            .await
            .map_err(|_| {
                DomainError::external_service_error("openbao_transit", "request failed")
            })?;
        if !response.status().is_success() {
            let status = response.status();
            return Err(DomainError::external_service_error(
                "openbao_transit",
                &format!("create key HTTP {status}"),
            ));
        }
        Ok(())
    }

    async fn fetch_public_key_pem(&self) -> Result<String, DomainError> {
        let cred = self.workload.resolve(&self.token_ref).await?;
        let url = self.endpoint("keys")?;
        debug!("OpenBao Transit read key");
        let response = self
            .client
            .get(url)
            .header("X-Vault-Token", &cred.secret)
            .send()
            .await
            .map_err(|_| {
                DomainError::external_service_error("openbao_transit", "request failed")
            })?;
        if !response.status().is_success() {
            let status = response.status();
            return Err(DomainError::external_service_error(
                "openbao_transit",
                &format!("read key HTTP {status}"),
            ));
        }
        let parsed: TransitReadKeyResponse = response.json().await.map_err(|_| {
            DomainError::external_service_error("openbao_transit", "invalid key response")
        })?;
        parsed
            .data
            .keys
            .values()
            .find_map(|v| v.public_key.clone())
            .or(parsed.data.latest_public_key)
            .ok_or_else(|| {
                DomainError::external_service_error(
                    "openbao_transit",
                    "Transit key response missing public_key PEM",
                )
            })
    }
}

#[derive(Debug, Deserialize)]
struct TransitSignResponse {
    data: TransitSignData,
}

#[derive(Debug, Deserialize)]
struct TransitSignData {
    signature: String,
}

#[derive(Debug, Deserialize)]
struct TransitReadKeyResponse {
    data: TransitReadKeyData,
}

#[derive(Debug, Deserialize)]
struct TransitReadKeyData {
    #[serde(default)]
    keys: std::collections::HashMap<String, TransitKeyVersion>,
    #[serde(default)]
    latest_public_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TransitKeyVersion {
    #[serde(default)]
    public_key: Option<String>,
}

#[async_trait]
impl SigningProvider for TransitSigningProvider {
    async fn sign_digest(&self, digest: &[u8]) -> Result<Vec<u8>, DomainError> {
        let cred = self.workload.resolve(&self.token_ref).await?;
        let url = self.endpoint("sign")?;
        let body = serde_json::json!({
            "input": STANDARD.encode(digest),
            "prehashed": true,
            "hash_algorithm": "sha2-256",
            "signature_algorithm": "pkcs1v15",
        });

        debug!("OpenBao Transit sign");
        let response = self
            .client
            .post(url)
            .header("X-Vault-Token", &cred.secret)
            .json(&body)
            .send()
            .await
            .map_err(|_| {
                DomainError::external_service_error("openbao_transit", "request failed")
            })?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(DomainError::external_service_error(
                "openbao_transit",
                &format!("HTTP {status}"),
            ));
        }

        let parsed: TransitSignResponse = response.json().await.map_err(|_| {
            DomainError::external_service_error("openbao_transit", "invalid sign response")
        })?;

        // vault:v1:<base64>
        let raw = parsed.data.signature.rsplit(':').next().ok_or_else(|| {
            DomainError::external_service_error("openbao_transit", "empty signature")
        })?;
        STANDARD.decode(raw).map_err(|e| {
            DomainError::external_service_error(
                "openbao_transit",
                &format!("invalid signature base64: {e}"),
            )
        })
    }

    async fn public_key(&self) -> Result<String, DomainError> {
        if let Some(pem) = &self.public_key_pem {
            return Ok(pem.clone());
        }
        self.fetch_public_key_pem().await
    }

    fn capabilities(&self) -> SigningCapabilities {
        SigningCapabilities {
            sign_digest: true,
            public_key_available: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::StaticCredential;
    use iam_domain::error::DomainError;
    use std::sync::Arc;

    #[test]
    fn new_rejects_forbidden_cosign_transit_key_name() {
        let workload = Arc::new(StaticCredential::default());
        let result = TransitSigningProvider::new(
            "http://127.0.0.1:8200",
            "apparatus-p4-cosign",
            "openbao-token",
            workload,
            None,
        );
        assert!(
            matches!(
                result,
                Err(DomainError::AuthorizationError(ref msg)) if msg.contains("apparatus-p4-cosign")
            ),
            "TransitSigningProvider::new must refuse apparatus-p4-cosign"
        );
    }

    #[tokio::test]
    async fn challenge_requests_only_exact_sign_endpoint_and_does_not_follow_redirects() {
        use iam_domain::port::{WorkloadCredential, WorkloadIdentity};
        use wiremock::{
            matchers::{method, path},
            Mock, MockServer, ResponseTemplate,
        };
        struct Credential;
        #[async_trait]
        impl WorkloadIdentity for Credential {
            async fn resolve(&self, _: &str) -> Result<WorkloadCredential, DomainError> {
                Ok(WorkloadCredential {
                    secret: "unit-credential".into(),
                })
            }
        }
        let server = MockServer::start().await;
        let destination = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/sign/org-key"))
            .respond_with(ResponseTemplate::new(302).insert_header("Location", destination.uri()))
            .expect(1)
            .mount(&server)
            .await;
        let provider = TransitSigningProvider::new(
            server.uri(),
            "org-key",
            "org-credential",
            Arc::new(Credential),
            Some("public-key".into()),
        )
        .unwrap();
        assert!(provider.sign_digest(b"digest").await.is_err());
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url.path(), "/v1/transit/sign/org-key");
        assert!(destination.received_requests().await.unwrap().is_empty());
    }
}
