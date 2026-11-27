//! Runtime knobs for the `ext_authz` Check service.

use rustycog::config::SessionLivenessConfig;
use std::time::Duration;

/// Mesh `AuthN` configuration (JWKS URL + local cache intervals).
#[derive(Debug, Clone, Default)]
pub struct ExtAuthzConfig {
    /// IAM JWKS URL (`GET` — cached locally, never per known `kid`).
    pub jwks_url: String,
    /// JWT `aud` expected by rustycog `UserIdExtractor` (mandatory — fail-closed).
    pub audience: String,
    /// Exact platform issuer. Required when session liveness is enabled.
    pub platform_issuer: String,
    /// Optional authoritative session-liveness reader.
    pub session_liveness: SessionLivenessConfig,
    /// Background poll interval for last-known-good refresh.
    pub poll_interval: Duration,
    /// TTL for unknown-`kid` negative cache after a successful fetch.
    pub negative_cache_ttl: Duration,
}

impl ExtAuthzConfig {
    /// Build from environment (`EXT_AUTHZ_JWKS_URL`, required `EXT_AUTHZ_AUDIENCE`).
    ///
    /// Session liveness is enabled only when `EXT_AUTHZ_LIVENESS_URL` is set;
    /// `EXT_AUTHZ_PLATFORM_ISSUER` and `EXT_AUTHZ_LIVENESS_TOKEN_ENV` then become
    /// mandatory at state construction.
    ///
    /// # Errors
    ///
    /// Returns an error when `EXT_AUTHZ_AUDIENCE` is missing or blank, or when
    /// `EXT_AUTHZ_LIVENESS_TIMEOUT_MS` is not an integer.
    pub fn from_env() -> Result<Self, String> {
        let jwks_url = std::env::var("EXT_AUTHZ_JWKS_URL").unwrap_or_default();
        let audience = std::env::var("EXT_AUTHZ_AUDIENCE").unwrap_or_default();
        if audience.trim().is_empty() {
            return Err("EXT_AUTHZ_AUDIENCE is required (fail-closed)".into());
        }
        let platform_issuer = std::env::var("EXT_AUTHZ_PLATFORM_ISSUER").unwrap_or_default();
        let session_liveness = SessionLivenessConfig {
            endpoint: std::env::var("EXT_AUTHZ_LIVENESS_URL").ok(),
            token_env: std::env::var("EXT_AUTHZ_LIVENESS_TOKEN_ENV").ok(),
            token_header: std::env::var("EXT_AUTHZ_LIVENESS_TOKEN_HEADER").ok(),
            timeout_ms: std::env::var("EXT_AUTHZ_LIVENESS_TIMEOUT_MS")
                .ok()
                .map(|value| {
                    value
                        .parse::<u64>()
                        .map_err(|_| "EXT_AUTHZ_LIVENESS_TIMEOUT_MS must be an integer".to_string())
                })
                .transpose()?,
        };
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
            platform_issuer,
            session_liveness,
            poll_interval: Duration::from_secs(poll_secs),
            negative_cache_ttl: Duration::from_secs(neg_secs),
        })
    }
}
