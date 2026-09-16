//! OpenBao/Vault HTTP KV v2 adapter.

use apparatus_contracts::MAX_PAYLOAD_BYTES;
use async_trait::async_trait;
use lazaret_domain::{parse_secret_reference, SecretError, SecretResolver};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

/// Vault/OpenBao KV v2 resolver (`GET /v1/{mount}/data/{path}`).
#[derive(Clone)]
pub struct VaultHttpSecretResolver {
    client: Client,
    base_url: String,
    token: String,
    mount: String,
}

impl VaultHttpSecretResolver {
    /// `base_url` is scheme+host+port with no trailing slash.
    ///
    /// # Errors
    ///
    /// Returns [`SecretError::ResolveFailed`] if the HTTP client cannot be built.
    pub fn new(
        base_url: impl Into<String>,
        token: impl Into<String>,
        mount: impl Into<String>,
    ) -> Result<Self, SecretError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| SecretError::ResolveFailed)?;
        Ok(Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            token: token.into(),
            mount: mount.into(),
        })
    }
}

#[derive(Deserialize)]
struct VaultReadBody {
    data: VaultDataEnvelope,
}

#[derive(Deserialize)]
struct VaultDataEnvelope {
    data: serde_json::Map<String, serde_json::Value>,
}

#[async_trait]
impl SecretResolver for VaultHttpSecretResolver {
    async fn resolve(&self, reference: &str) -> Result<Vec<u8>, SecretError> {
        let (path, field) = parse_secret_reference(reference)?;
        if self.token.is_empty() {
            return Err(SecretError::ResolveFailed);
        }
        let encoded_path = encode_vault_path(path);
        let url = format!("{}/v1/{}/data/{encoded_path}", self.base_url, self.mount);
        let mut headers = HeaderMap::new();
        let token = HeaderValue::from_str(&self.token).map_err(|_| SecretError::ResolveFailed)?;
        headers.insert("X-Vault-Token", token);
        let response = self
            .client
            .get(url)
            .headers(headers)
            .send()
            .await
            .map_err(|_| SecretError::ResolveFailed)?;
        if !response.status().is_success() {
            return Err(SecretError::ResolveFailed);
        }
        if response
            .content_length()
            .is_some_and(|len| len as usize > MAX_PAYLOAD_BYTES)
        {
            return Err(SecretError::ResolveFailed);
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|_| SecretError::ResolveFailed)?;
        if bytes.len() > MAX_PAYLOAD_BYTES {
            return Err(SecretError::ResolveFailed);
        }
        let body: VaultReadBody =
            serde_json::from_slice(&bytes).map_err(|_| SecretError::ResolveFailed)?;
        let value = body
            .data
            .data
            .get(field)
            .and_then(serde_json::Value::as_str)
            .ok_or(SecretError::ResolveFailed)?;
        Ok(value.as_bytes().to_vec())
    }
}

fn encode_vault_path(path: &str) -> String {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| urlencoding::encode(segment).into_owned())
        .collect::<Vec<_>>()
        .join("/")
}
