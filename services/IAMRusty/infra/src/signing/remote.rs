//! Remote HTTP SigningProvider — Sign(digest) / GetPublicKey only (ADR-0309).

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use iam_domain::error::DomainError;
use iam_domain::port::{SigningCapabilities, SigningProvider, WorkloadIdentity};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

/// HTTP adapter: `POST {url}/sign` and `GET {url}/keys/{key_id}`.
///
/// Never sends JWT claims. Never holds or returns a private key.
pub struct RemoteSigningProvider {
    client: Client,
    base_url: String,
    key_id: String,
    algorithm: String,
    token_ref: String,
    workload: Arc<dyn WorkloadIdentity>,
}

#[derive(Debug, Serialize)]
struct SignRequest<'a> {
    key_id: &'a str,
    algorithm: &'a str,
    digest: String,
}

#[derive(Debug, Deserialize)]
struct SignResponse {
    signature: String,
}

#[derive(Debug, Deserialize)]
struct PublicKeyJson {
    public_key: String,
}

impl RemoteSigningProvider {
    /// Build a remote signer client.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `url` / `key_id` is empty or the HTTP client cannot be built.
    pub fn new(
        base_url: impl Into<String>,
        key_id: impl Into<String>,
        algorithm: impl Into<String>,
        token_ref: impl Into<String>,
        workload: Arc<dyn WorkloadIdentity>,
    ) -> Result<Self, DomainError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let key_id = key_id.into();
        if base_url.is_empty() {
            return Err(DomainError::external_service_error(
                "remote_signer",
                "remote signer url absent — fail-closed",
            ));
        }
        if key_id.trim().is_empty() {
            return Err(DomainError::external_service_error(
                "remote_signer",
                "remote signer key_id must not be empty",
            ));
        }
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|_| {
                DomainError::external_service_error(
                    "remote_signer",
                    "HTTP client initialization failed",
                )
            })?;
        Ok(Self {
            client,
            base_url,
            key_id,
            algorithm: algorithm.into(),
            token_ref: token_ref.into(),
            workload,
        })
    }

    async fn authorization(&self) -> Result<String, DomainError> {
        let cred = self.workload.resolve(&self.token_ref).await?;
        let secret = cred.secret;
        if secret.len() >= 7 && secret[..7].eq_ignore_ascii_case("Bearer ") {
            Ok(secret)
        } else {
            Ok(format!("Bearer {secret}"))
        }
    }
}

#[async_trait]
impl SigningProvider for RemoteSigningProvider {
    async fn sign_digest(&self, digest: &[u8]) -> Result<Vec<u8>, DomainError> {
        let auth = self.authorization().await?;
        let url = format!("{}/sign", self.base_url);
        let body = SignRequest {
            key_id: &self.key_id,
            algorithm: &self.algorithm,
            digest: STANDARD.encode(digest),
        };
        debug!("remote signer sign_digest");
        let response = self
            .client
            .post(&url)
            .header("Authorization", auth)
            .json(&body)
            .send()
            .await
            .map_err(|_| {
                DomainError::external_service_error("remote_signer", "sign request failed")
            })?;
        if !response.status().is_success() {
            let status = response.status();
            return Err(DomainError::external_service_error(
                "remote_signer",
                &format!("sign HTTP {status}"),
            ));
        }
        let parsed: SignResponse = response.json().await.map_err(|_| {
            DomainError::external_service_error("remote_signer", "invalid sign response")
        })?;
        STANDARD.decode(parsed.signature.trim()).map_err(|e| {
            DomainError::external_service_error(
                "remote_signer",
                &format!("invalid signature encoding: {e}"),
            )
        })
    }

    async fn public_key(&self) -> Result<String, DomainError> {
        let auth = self.authorization().await?;
        let url = format!("{}/keys/{}", self.base_url, self.key_id);
        debug!("remote signer get public key");
        let response = self
            .client
            .get(&url)
            .header("Authorization", auth)
            .send()
            .await
            .map_err(|_| {
                DomainError::external_service_error("remote_signer", "public key request failed")
            })?;
        if !response.status().is_success() {
            let status = response.status();
            return Err(DomainError::external_service_error(
                "remote_signer",
                &format!("keys HTTP {status}"),
            ));
        }
        let text = response.text().await.map_err(|_| {
            DomainError::external_service_error("remote_signer", "public key response failed")
        })?;
        let pem = if let Ok(json) = serde_json::from_str::<PublicKeyJson>(&text) {
            json.public_key
        } else {
            text
        };
        let pem = pem.trim().to_string();
        if pem.contains("PRIVATE KEY") {
            return Err(DomainError::external_service_error(
                "remote_signer",
                "remote signer must not return a private key",
            ));
        }
        if !pem.contains("PUBLIC KEY") {
            return Err(DomainError::external_service_error(
                "remote_signer",
                "remote signer public key response is not a public PEM",
            ));
        }
        Ok(pem)
    }

    fn capabilities(&self) -> SigningCapabilities {
        SigningCapabilities {
            sign_digest: true,
            public_key_available: true,
        }
    }
}
