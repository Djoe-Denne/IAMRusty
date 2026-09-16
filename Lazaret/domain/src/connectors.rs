//! Named connectors admitted by the operator. Plugins pass a name, never a URL.

use std::collections::HashMap;

use async_trait::async_trait;
use thiserror::Error;
use url::Url;

/// Failure to use a named connector.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConnectorError {
    /// Plugin supplied a raw URL, `fetchInternal`, or unknown name.
    #[error("connector request rejected")]
    Rejected,
    /// Name is not in the operator allow-list.
    #[error("unknown connector")]
    Unknown,
    /// Outbound call failed (fail closed).
    #[error("connector fetch failed")]
    FetchFailed,
    /// Downstream body exceeded [`apparatus_contracts::MAX_PAYLOAD_BYTES`].
    #[error("connector payload too large")]
    PayloadTooLarge,
}

/// Operator-admitted name → base URL.
#[derive(Debug, Clone, Default)]
pub struct ConnectorRegistry {
    by_name: HashMap<String, String>,
}

impl ConnectorRegistry {
    /// Build from config entries. Invalid operator URLs are skipped (fail closed at use).
    #[must_use]
    pub fn from_entries(entries: &[(String, String)]) -> Self {
        let mut by_name = HashMap::new();
        for (name, url) in entries {
            if name.is_empty() || name.contains("://") || name.contains('/') {
                continue;
            }
            if let Ok(parsed) = Url::parse(url) {
                if parsed.scheme() == "http" || parsed.scheme() == "https" {
                    by_name.insert(name.clone(), url.trim_end_matches('/').to_owned());
                }
            }
        }
        Self { by_name }
    }

    /// Resolve a plugin-supplied connector name.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectorError::Rejected`] for URL-shaped input, [`ConnectorError::Unknown`]
    /// when the name is not admitted.
    pub fn lookup(&self, name: &str) -> Result<&str, ConnectorError> {
        if name.is_empty()
            || name.contains("://")
            || name.contains('/')
            || name.eq_ignore_ascii_case("fetchInternal")
            || looks_like_ipv4(name)
        {
            return Err(ConnectorError::Rejected);
        }
        self.by_name
            .get(name)
            .map(String::as_str)
            .ok_or(ConnectorError::Unknown)
    }

    /// Join an admitted base URL with a relative path (`/foo`).
    ///
    /// Operator base path prefix is preserved. `..`, `://`, and host jumps are rejected.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectorError::Rejected`] when `path` is absolute-URL shaped or tries to jump hosts.
    pub fn join_path(&self, name: &str, path: &str) -> Result<String, ConnectorError> {
        let base = self.lookup(name)?;
        if path.contains("://") || path.contains('\\') || path.contains("..") || path.contains("//")
        {
            return Err(ConnectorError::Rejected);
        }
        let path = if path.is_empty() {
            "/"
        } else if path.starts_with('/') {
            path
        } else {
            return Err(ConnectorError::Rejected);
        };
        let base_url = Url::parse(base).map_err(|_| ConnectorError::Rejected)?;
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if segments.iter().any(|s| *s == "." || *s == "..") {
            return Err(ConnectorError::Rejected);
        }
        let mut url = base_url.clone();
        {
            let mut path_segments = url
                .path_segments_mut()
                .map_err(|()| ConnectorError::Rejected)?;
            for segment in segments {
                path_segments.push(segment);
            }
        }
        if url.host_str() != base_url.host_str() || url.scheme() != base_url.scheme() {
            return Err(ConnectorError::Rejected);
        }
        Ok(url.to_string())
    }
}

fn looks_like_ipv4(name: &str) -> bool {
    let parts: Vec<&str> = name.split('.').collect();
    parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok())
}

/// Outbound fetch via an admitted connector name only.
#[async_trait]
pub trait ConnectorProxy: Send + Sync {
    /// GET `name` + relative `path`. Optional bearer is injected by the platform.
    async fn fetch(
        &self,
        name: &str,
        path: &str,
        injected_bearer: Option<&[u8]>,
    ) -> Result<Vec<u8>, ConnectorError>;
}
