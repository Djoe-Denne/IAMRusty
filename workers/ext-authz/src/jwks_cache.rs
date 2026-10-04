//! Local JWKS cache with a bounded, monotonic authorization snapshot.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::warn;

const MAX_SNAPSHOT_AGE: Duration = Duration::from_secs(60);
const FETCH_TIMEOUT: Duration = Duration::from_millis(500);
const MIN_REFRESH_INTERVAL: Duration = Duration::from_millis(250);
const MAX_NEGATIVE_KIDS: usize = 1024;
const MAX_KID_BYTES: usize = 128;
const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;

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
            "active" => Self::Active,
            _ => Self::Revoked,
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
    acquired_at: Instant,
}

struct Inner {
    last_good: Option<CachedDoc>,
    last_fetch: Option<Instant>,
    negative: HashMap<String, Instant>,
}

/// Process-local JWKS cache. Known keys are usable only within 60 seconds.
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
        if kid.is_empty() || kid.len() > MAX_KID_BYTES {
            return Err("invalid kid length".into());
        }
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
        self.cached_if_known_at(kid, Instant::now())
    }

    fn cached_if_known_at(&self, kid: &str, now: Instant) -> Option<String> {
        let guard = self
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let doc = guard.last_good.as_ref()?;
        if now.saturating_duration_since(doc.acquired_at) >= MAX_SNAPSHOT_AGE {
            return None;
        }
        if doc.kids.contains_key(kid) {
            Some(doc.raw.clone())
        } else {
            None
        }
    }

    fn is_negative(&self, kid: &str) -> bool {
        self.is_negative_at(kid, Instant::now())
    }

    fn is_negative_at(&self, kid: &str, now: Instant) -> bool {
        let guard = self
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard
            .negative
            .get(kid)
            .is_some_and(|at| now.saturating_duration_since(*at) < self.negative_ttl)
    }

    fn mark_negative(&self, kid: &str) {
        let mut guard = self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = Instant::now();
        guard
            .negative
            .retain(|_, at| now.saturating_duration_since(*at) < self.negative_ttl);
        if guard.negative.len() < MAX_NEGATIVE_KIDS {
            guard.negative.insert(kid.to_string(), now);
        }
    }

    /// Canonical publisher status on a fresh cached JWK.
    #[must_use]
    pub fn status_for_kid(&self, kid: &str) -> Option<KeyStatus> {
        let guard = self
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let doc = guard.last_good.as_ref()?;
        if doc.acquired_at.elapsed() >= MAX_SNAPSHOT_AGE {
            return None;
        }
        doc.kids.get(kid).copied()
    }

    /// Recheck the exact authorization snapshot after signature verification.
    #[must_use]
    pub fn still_authorizes(&self, kid: &str, raw: &str) -> bool {
        let guard = self
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.last_good.as_ref().is_some_and(|doc| {
            doc.acquired_at.elapsed() < MAX_SNAPSHOT_AGE
                && doc.raw == raw
                && doc.kids.get(kid).is_some_and(|status| !status.is_denied())
        })
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
        // Do not accumulate unbounded request waiters behind a slow upstream.
        let Ok(_permit) = self.refresh.try_lock() else {
            return Ok(());
        };
        let acquired_at = Instant::now();
        {
            let mut guard = self
                .inner
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(at) = guard.last_fetch {
                if at.elapsed() < MIN_REFRESH_INTERVAL {
                    return Ok(());
                }
            }
            // Attempt time throttles retries, but never renews authorization age.
            guard.last_fetch = Some(acquired_at);
        }
        match self.fetch_once().await {
            Ok(mut doc) => {
                doc.acquired_at = acquired_at;
                let mut guard = self
                    .inner
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                guard.last_good = Some(doc);
                guard.negative.clear();
                Ok(())
            }
            Err(e) => {
                warn!(error = %e, "JWKS refresh failed; retaining snapshot without renewing freshness");
                let has_good = {
                    let guard = self
                        .inner
                        .read()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    guard
                        .last_good
                        .as_ref()
                        .is_some_and(|doc| doc.acquired_at.elapsed() < MAX_SNAPSHOT_AGE)
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
        let mut response = self
            .http
            .get(&self.url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("JWKS HTTP {}", response.status()));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            if bytes.len().saturating_add(chunk.len()) > MAX_DOCUMENT_BYTES {
                return Err("JWKS document too large".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let raw = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        let kids = parse_kid_status(&raw)?;
        // A valid empty set is an authoritative revocation, not an outage.
        Ok(CachedDoc {
            raw,
            kids,
            acquired_at: Instant::now(),
        })
    }
}

fn build_http_client() -> Result<reqwest::Client, String> {
    let ca = non_empty_env("EXT_AUTHZ_TLS_CA");
    let cert = non_empty_env("EXT_AUTHZ_TLS_CERT");
    let key = non_empty_env("EXT_AUTHZ_TLS_KEY");
    let mut builder = reqwest::Client::builder()
        .use_rustls_tls()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(FETCH_TIMEOUT)
        .timeout(FETCH_TIMEOUT);
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
    let _: jsonwebtoken::jwk::JwkSet = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let keys = value
        .get("keys")
        .and_then(Value::as_array)
        .ok_or_else(|| "JWKS missing keys".to_string())?;
    let mut map = HashMap::new();
    for key in keys {
        let kid = key
            .get("kid")
            .and_then(Value::as_str)
            .filter(|kid| !kid.is_empty() && kid.len() <= MAX_KID_BYTES)
            .ok_or_else(|| "JWKS key missing valid kid".to_string())?;
        let status_raw = key
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| "JWKS missing status".to_string())?;
        if !matches!(status_raw, "pending" | "active" | "retiring" | "revoked") {
            return Err("JWKS unknown status".into());
        }
        let status = KeyStatus::parse(status_raw);
        if key
            .get("iss")
            .and_then(Value::as_str)
            .is_none_or(|iss| iss.trim().is_empty())
        {
            return Err("JWKS missing issuer".into());
        }
        match key.get("trust_scope").and_then(Value::as_str) {
            Some("platform") if key.get("organization_id").is_some_and(Value::is_null) => {}
            Some("organization") => {
                let raw = key
                    .get("organization_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "JWKS missing organization owner".to_string())?;
                let owner = uuid::Uuid::parse_str(raw)
                    .map_err(|_| "JWKS invalid organization owner".to_string())?;
                if owner.to_string() != raw {
                    return Err("JWKS noncanonical organization owner".into());
                }
            }
            _ => return Err("JWKS invalid trust scope".into()),
        }
        if key.get("kty").and_then(Value::as_str) != Some("RSA")
            || key
                .get("alg")
                .and_then(Value::as_str)
                .is_some_and(|alg| alg != "RS256")
            || key
                .get("use")
                .and_then(Value::as_str)
                .is_some_and(|usage| usage != "sig")
        {
            return Err("JWKS unsupported algorithm".into());
        }
        let jwk: jsonwebtoken::jwk::Jwk =
            serde_json::from_value(key.clone()).map_err(|e| e.to_string())?;
        jsonwebtoken::DecodingKey::from_jwk(&jwk).map_err(|e| e.to_string())?;
        if map.insert(kid.to_string(), status).is_some() {
            return Err("duplicate JWKS kid".into());
        }
    }
    Ok(map)
}

#[cfg(test)]
mod freshness_tests {
    use super::*;

    #[test]
    fn canonical_publisher_metadata_is_required_and_pending_is_not_trusted() {
        let mut doc: Value =
            serde_json::from_str(&rustycog::testing::http::jwt::test_rs256_jwks_json()).unwrap();
        doc["keys"][0]["status"] = serde_json::json!("pending");
        doc["keys"][0]["trust_scope"] = serde_json::json!("platform");
        doc["keys"][0]["organization_id"] = Value::Null;
        assert!(parse_kid_status(&doc.to_string())
            .unwrap()
            .values()
            .all(|status| status.is_denied()));
        for field in ["status", "trust_scope", "organization_id", "iss"] {
            let mut invalid = doc.clone();
            invalid["keys"][0].as_object_mut().unwrap().remove(field);
            assert!(parse_kid_status(&invalid.to_string()).is_err());
        }
        let mut duplicate = doc.clone();
        duplicate["keys"]
            .as_array_mut()
            .unwrap()
            .push(doc["keys"][0].clone());
        assert!(parse_kid_status(&duplicate.to_string()).is_err());
        doc["keys"][0]["status"] = serde_json::json!("unknown");
        assert!(parse_kid_status(&doc.to_string()).is_err());
    }

    #[test]
    fn known_key_expires_and_failed_attempt_does_not_extend_epoch() {
        let cache = JwksCache::new(
            "http://unused.invalid",
            Duration::from_secs(60),
            Duration::from_secs(30),
        )
        .unwrap();
        let start = Instant::now();
        {
            let mut inner = cache.inner.write().unwrap();
            inner.last_good = Some(CachedDoc {
                raw: "snapshot".into(),
                kids: HashMap::from([("kid".into(), KeyStatus::Active)]),
                acquired_at: start,
            });
            inner.last_fetch = Some(start + Duration::from_secs(59));
        }
        assert!(cache
            .cached_if_known_at("kid", start + Duration::from_secs(59))
            .is_some());
        assert!(cache
            .cached_if_known_at("kid", start + Duration::from_secs(60))
            .is_none());
        assert!(cache
            .cached_if_known_at("kid", start + Duration::from_secs(3600))
            .is_none());
    }

    #[test]
    fn empty_snapshot_is_authoritative_and_negative_cache_is_bounded() {
        assert!(parse_kid_status(r#"{"keys":[]}"#).unwrap().is_empty());
        assert!(parse_kid_status(r#"{"keys":null}"#).is_err());
        let cache = JwksCache::new(
            "http://unused.invalid",
            Duration::ZERO,
            Duration::from_secs(30),
        )
        .unwrap();
        for index in 0..(MAX_NEGATIVE_KIDS * 2) {
            cache.mark_negative(&index.to_string());
        }
        assert_eq!(
            cache.inner.read().unwrap().negative.len(),
            MAX_NEGATIVE_KIDS
        );
        assert!(KeyStatus::parse("unexpected").is_denied());
    }

    #[test]
    fn negative_expiry_and_authoritative_replacement_recovery() {
        let cache = JwksCache::new(
            "http://unused.invalid",
            Duration::from_secs(60),
            Duration::from_secs(30),
        )
        .unwrap();
        cache.mark_negative("missing");
        let at = *cache.inner.read().unwrap().negative.get("missing").unwrap();
        assert!(cache.is_negative_at("missing", at + Duration::from_secs(29)));
        assert!(!cache.is_negative_at("missing", at + Duration::from_secs(30)));
        let now = Instant::now();
        {
            let mut inner = cache.inner.write().unwrap();
            inner.last_good = Some(CachedDoc {
                raw: "old".into(),
                kids: HashMap::from([("kid".into(), KeyStatus::Active)]),
                acquired_at: now,
            });
        }
        assert!(cache.still_authorizes("kid", "old"));
        {
            let mut inner = cache.inner.write().unwrap();
            inner.last_good = Some(CachedDoc {
                raw: r#"{"keys":[]}"#.into(),
                kids: HashMap::new(),
                acquired_at: now,
            });
        }
        assert!(!cache.still_authorizes("kid", "old"));
        assert!(cache.cached_if_known_at("kid", now).is_none());
        {
            let mut inner = cache.inner.write().unwrap();
            inner.last_good = Some(CachedDoc {
                raw: "new".into(),
                kids: HashMap::from([("kid".into(), KeyStatus::Active)]),
                acquired_at: now,
            });
        }
        assert!(cache.still_authorizes("kid", "new"));
        assert!(!cache.still_authorizes("kid", "old"));
    }
}
