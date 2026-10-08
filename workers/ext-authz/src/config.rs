//! Runtime knobs for the `ext_authz` Check service.

use std::time::Duration;

/// Mesh `AuthN` configuration (JWKS URL + local cache intervals).
#[derive(Debug, Clone)]
pub struct ExtAuthzConfig {
    /// IAM JWKS URL (`GET` — cached locally, never per known `kid`).
    pub jwks_url: String,
    /// JWT `aud` expected by rustycog `UserIdExtractor` (mandatory — fail-closed).
    pub audience: String,
    /// Background poll interval for last-known-good refresh.
    pub poll_interval: Duration,
    /// TTL for unknown-`kid` negative cache after a successful fetch.
    pub negative_cache_ttl: Duration,
}

impl ExtAuthzConfig {
    /// Build from environment (`EXT_AUTHZ_JWKS_URL`, required `EXT_AUTHZ_AUDIENCE`).
    ///
    /// # Errors
    ///
    /// Returns an error when `EXT_AUTHZ_AUDIENCE` is missing or blank (fail-closed boot).
    pub fn from_env() -> Result<Self, String> {
        let jwks_url = std::env::var("EXT_AUTHZ_JWKS_URL").unwrap_or_default();
        let audience = std::env::var("EXT_AUTHZ_AUDIENCE").unwrap_or_default();
        if audience.trim().is_empty() {
            return Err("EXT_AUTHZ_AUDIENCE is required (fail-closed)".into());
        }
        let poll_secs = std::env::var("EXT_AUTHZ_JWKS_POLL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);
        let neg_secs = std::env::var("EXT_AUTHZ_JWKS_NEGATIVE_TTL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        Ok(Self {
            jwks_url,
            audience,
            poll_interval: Duration::from_secs(poll_secs),
            negative_cache_ttl: Duration::from_secs(neg_secs),
        })
    }
}
