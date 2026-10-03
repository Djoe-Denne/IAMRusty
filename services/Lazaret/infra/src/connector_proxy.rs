//! Outbound HTTP only through operator-admitted named connectors.

use apparatus_contracts::MAX_PAYLOAD_BYTES;
use async_trait::async_trait;
use futures::StreamExt;
use lazaret_domain::{ConnectorError, ConnectorProxy, ConnectorRegistry};
use reqwest::Client;
use std::time::Duration;

/// Proxy: name in, bytes out. No plugin-chosen URL.
#[derive(Clone)]
pub struct NamedConnectorProxy {
    registry: ConnectorRegistry,
    client: Client,
}

impl NamedConnectorProxy {
    /// Wire an admitted registry.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectorError::FetchFailed`] if the HTTP client cannot be built.
    pub fn new(registry: ConnectorRegistry) -> Result<Self, ConnectorError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| ConnectorError::FetchFailed)?;
        Ok(Self { registry, client })
    }

    /// GET an admitted connector + relative path. Optional secret is injected as Bearer.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectorError`] on name/path rejection or transport failure.
    pub async fn fetch(
        &self,
        name: &str,
        path: &str,
        injected_bearer: Option<&[u8]>,
    ) -> Result<Vec<u8>, ConnectorError> {
        let url = self.registry.join_path(name, path)?;
        let mut request = self.client.get(&url);
        if let Some(secret) = injected_bearer {
            let token = String::from_utf8_lossy(secret);
            request = request.bearer_auth(token.as_ref());
        }
        let response = request
            .send()
            .await
            .map_err(|_| ConnectorError::FetchFailed)?;
        if !response.status().is_success() {
            return Err(ConnectorError::FetchFailed);
        }
        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| ConnectorError::FetchFailed)?;
            body.extend_from_slice(&chunk);
            if body.len() > MAX_PAYLOAD_BYTES {
                return Err(ConnectorError::PayloadTooLarge);
            }
        }
        Ok(body)
    }

    /// Expose registry for tests.
    #[must_use]
    pub const fn registry(&self) -> &ConnectorRegistry {
        &self.registry
    }
}

#[async_trait]
impl ConnectorProxy for NamedConnectorProxy {
    async fn fetch(
        &self,
        name: &str,
        path: &str,
        injected_bearer: Option<&[u8]>,
    ) -> Result<Vec<u8>, ConnectorError> {
        NamedConnectorProxy::fetch(self, name, path, injected_bearer).await
    }
}
