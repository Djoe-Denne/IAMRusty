//! Local JWKS cache: known kid → no fetch; singleflight; negative; last-known-good.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::warn;

/// JWKS / test-registry key lifecycle (deny pending + revoked).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStatus {
    Pending,
    Active,
    Retiring,
    Revoked,
}

impl KeyStatus {
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "pending" => Self::Pending,
            "revoked" => Self::Revoked,
            "retiring" => Self::Retiring,
            _ => Self::Active,
        }
    }

    #[must_use]
    pub const fn is_denied(self) -> bool {
        matches!(self, Self::Pending | Self::Revoked)
    }
}

#[derive(Clone)]
struct CachedDoc {
    raw: String,
    kids: HashMap<String, KeyStatus>,
}

struct Inner {
    last_good: Option<CachedDoc>,
    last_fetch: Option<Instant>,
    negative: HashMap<String, Instant>,
}

/// Process-local JWKS cache (not rustycog's). Do not fetch when `kid` is already known.
pub struct JwksCache {
    url: String,
    poll_interval: Duration,
    negative_ttl: Duration,
    http: reqwest::Client,
    inner: std::sync::RwLock<Inner>,
    refresh: Mutex<()>,
}

impl JwksCache {
    /// # Errors
    ///
    /// Returns an error when `url` is empty or the HTTP client cannot be built.
    pub fn new(
        url: impl Into<String>,
        poll_interval: Duration,
        negative_ttl: Duration,
    ) -> Result<Arc<Self>, String> {
        let url = url.into().trim().to_string();
        if url.is_empty() {
            return Err("JWKS URL is empty".into());
        }
        let http = build_http_client()?;
        Ok(Arc::new(Self {
            url,
            poll_interval,
            negative_ttl,
            http,
            inner: std::sync::RwLock::new(Inner {
                last_good: None,
                last_fetch: None,
                negative: HashMap::new(),
            }),
            refresh: Mutex::new(()),
        }))
    }

    /// Return the last-known JWKS JSON for `kid`, fetching only if the kid is unknown.
    ///
    /// # Errors
    ///
    /// Returns an error when the kid is negatively cached or no document can be obtained.
    pub async fn document_for_kid(&self, kid: &str) -> Result<String, String> {
        if let Some(doc) = self.cached_if_known(kid) {
            return Ok(doc);
        }
        if self.is_negative(kid) {
            return Err(format!("unknown kid (negative cache): {kid}"));
        }
        self.refresh_coalesced().await?;
        if let Some(doc) = self.cached_if_known(kid) {
            return Ok(doc);
        }
        self.mark_negative(kid);
        Err(format!("kid {kid} not in JWKS"))
    }

    fn cached_if_known(&self, kid: &str) -> Option<String> {
        let guard = self
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let doc = guard.last_good.as_ref()?;
        if doc.kids.contains_key(kid) {
            Some(doc.raw.clone())
        } else {
            None
        }
    }

    fn is_negative(&self, kid: &str) -> bool {
        let guard = self
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard
            .negative
            .get(kid)
            .is_some_and(|at| at.elapsed() < self.negative_ttl)
    }

    fn mark_negative(&self, kid: &str) {
        let mut guard = self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.negative.insert(kid.to_string(), Instant::now());
    }

    /// Status advertised on the cached JWK (`status` field) or `Active` if omitted.
    #[must_use]
    pub fn status_for_kid(&self, kid: &str) -> Option<KeyStatus> {
        let guard = self
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.last_good.as_ref()?.kids.get(kid).copied()
    }

    /// Periodic refresh (bin poll). Not invoked on Check when `kid` is already known.
    pub async fn poll_if_due(&self) {
        let due = {
            let guard = self
                .inner
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            match guard.last_fetch {
                None => true,
                Some(at) => at.elapsed() >= self.poll_interval,
            }
        };
        if due {
            let _ = self.refresh_coalesced().await;
        }
    }

    async fn refresh_coalesced(&self) -> Result<(), String> {
        let _permit = self.refresh.lock().await;
        {
            let guard = self
                .inner
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(at) = guard.last_fetch {
                if at.elapsed() < Duration::from_millis(50) && guard.last_good.is_some() {
                    return Ok(());
                }
            }
        }
        match self.fetch_once().await {
            Ok(doc) => {
                let mut guard = self
                    .inner
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                guard.last_good = Some(doc);
                guard.last_fetch = Some(Instant::now());
                guard.negative.clear();
                Ok(())
            }
            Err(e) => {
                warn!(error = %e, "JWKS refresh failed; keeping last-known-good");
                let has_good = {
                    let guard = self
                        .inner
                        .read()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    guard.last_good.is_some()
                };
                if has_good {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn fetch_once(&self) -> Result<CachedDoc, String> {
        let response = self
            .http
            .get(&self.url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("JWKS HTTP {}", response.status()));
        }
        let raw = response.text().await.map_err(|e| e.to_string())?;
        let kids = parse_kid_status(&raw)?;
        if kids.is_empty() {
            return Err("JWKS document contained no keys".into());
        }
        Ok(CachedDoc { raw, kids })
    }
}

fn build_http_client() -> Result<reqwest::Client, String> {
    let ca = non_empty_env("EXT_AUTHZ_TLS_CA");
    let cert = non_empty_env("EXT_AUTHZ_TLS_CERT");
    let key = non_empty_env("EXT_AUTHZ_TLS_KEY");
    let mut builder = reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(10));
    match (ca, cert, key) {
        (None, None, None) => {}
        (Some(ca_path), Some(cert_path), Some(key_path)) => {
            let ca_pem = std::fs::read(&ca_path).map_err(|err| format!("read {ca_path}: {err}"))?;
            let mut identity =
                std::fs::read(&cert_path).map_err(|err| format!("read {cert_path}: {err}"))?;
            identity.push(b'\n');
            identity
                .extend(std::fs::read(&key_path).map_err(|err| format!("read {key_path}: {err}"))?);
            let root = reqwest::Certificate::from_pem(&ca_pem)
                .map_err(|err| format!("parse CA: {err}"))?;
            let id = reqwest::Identity::from_pem(&identity)
                .map_err(|err| format!("parse client identity: {err}"))?;
            builder = builder.add_root_certificate(root).identity(id);
        }
        _ => {
            return Err(
                "EXT_AUTHZ_TLS_CA, EXT_AUTHZ_TLS_CERT and EXT_AUTHZ_TLS_KEY must be set together"
                    .into(),
            );
        }
    }
    builder.build().map_err(|err| err.to_string())
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn parse_kid_status(raw: &str) -> Result<HashMap<String, KeyStatus>, String> {
    let value: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let keys = value
        .get("keys")
        .and_then(Value::as_array)
        .ok_or_else(|| "JWKS missing keys".to_string())?;
    let mut map = HashMap::new();
    for key in keys {
        let Some(kid) = key.get("kid").and_then(Value::as_str) else {
            continue;
        };
        let status = key
            .get("status")
            .and_then(Value::as_str)
            .map(KeyStatus::parse)
            .unwrap_or(KeyStatus::Active);
        map.insert(kid.to_string(), status);
    }
    Ok(map)
}
