//! Explicit security policy. Build profile and environment names are not exemptions.

use crate::IdpConfig;
use serde::{Deserialize, Serialize};

/// Local plaintext transport is opt-in, never inferred from a hostname.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityMode {
    #[default]
    Verified,
    LocalInsecure,
    IsolatedTest,
}

/// `[security]` boot policy; missing configuration is production-safe.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default)]
    pub mode: SecurityMode,
    #[serde(default)]
    pub rate_limit: AuthRateLimitConfig,
}

/// Proxy metadata is accepted only from an allowlisted transport IP AND verified TLS SAN.
/// The proxy must append its actual downstream peer to XFF (never preserve it verbatim).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedAuthProxy {
    /// Exact peer address, or canonical network address when `prefix_len` is set.
    pub ip: std::net::IpAddr,
    /// Omitted means an exact IP. Zero/universal and noncanonical networks are rejected.
    #[serde(default)]
    pub prefix_len: Option<u8>,
    pub dns_san: String,
}

impl TrustedAuthProxy {
    fn network_mask(&self) -> Option<u128> {
        let bits = if self.ip.is_ipv4() { 32 } else { 128 };
        let prefix = self.prefix_len.unwrap_or(bits);
        if prefix == 0 || prefix > bits {
            return None;
        }
        Some(u128::MAX << (bits - prefix))
    }

    fn address_bits(ip: std::net::IpAddr) -> u128 {
        match ip {
            std::net::IpAddr::V4(ip) => u128::from(u32::from(ip)),
            std::net::IpAddr::V6(ip) => u128::from(ip),
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.dns_san.is_empty() && self.dns_san.len() <= 253
            && self.network_mask().is_some_and(|mask| Self::address_bits(self.ip) & !mask == 0)
    }

    /// Match only the configured transport network; TLS SAN verification is still mandatory.
    #[must_use]
    pub fn matches_peer(&self, peer: std::net::IpAddr) -> bool {
        self.is_valid() && self.ip.is_ipv4() == peer.is_ipv4()
            && self.network_mask().is_some_and(|mask| Self::address_bits(peer) & mask == Self::address_bits(self.ip))
    }
}

/// Per-instance bounded IAM abuse budget. Disable is legal only in an isolated test.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthRateLimitConfig {
    pub source_limit: u32,
    pub account_limit: u32,
    pub window_seconds: u64,
    pub max_buckets: usize,
    pub max_concurrent: usize,
    pub trusted_proxies: Vec<TrustedAuthProxy>,
    pub disabled: bool,
}

impl Default for AuthRateLimitConfig {
    fn default() -> Self {
        // Mesh deployments declare their proxy network and TLS SAN in configuration.
        // Never implicitly trust all peers (or even all pods) in generic defaults.
        Self { source_limit: 30, account_limit: 10, window_seconds: 60,
            max_buckets: 10_000, max_concurrent: 8, trusted_proxies: Vec::new(), disabled: false }
    }
}

/// A boot-validated OAuth key. Deliberately has no `Debug`/`Serialize` implementation.
#[derive(Clone)]
pub struct OAuthStateSecret(Vec<u8>);

impl OAuthStateSecret {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

// Committed development credential: expressly forbidden in verified mode.
pub const LOCAL_OAUTH_STATE_SECRET: &str = "b9a3475cd01f826ee684f2703acb15d96c02e85147bd6a3905f8a63e12d9c407";

impl SecurityConfig {
    /// Validate before constructing any app state, tasks, or outbound clients.
    ///
    /// # Errors
    /// Returns a non-secret diagnostic for an unsafe key or connector URL.
    pub fn validate(&self, key: &str, idp: &IdpConfig) -> Result<OAuthStateSecret, String> {
        let key = self.validate_oauth_state_secret(key)?;
        idp.validate()?;
        for connector in &idp.connectors {
            validate_connector_url(&connector.base_url, self.mode).map_err(str::to_owned)?;
        }
        Ok(key)
    }

    /// Tests must select the isolated policy explicitly; empty keys are never valid.
    ///
    /// # Errors
    /// Rejects empty, padded, weak, placeholder or committed production keys.
    pub fn validate_oauth_state_secret(&self, key: &str) -> Result<OAuthStateSecret, String> {
        if key.trim() != key || key.len() > 1024 || key.len() < 16 {
            return Err("OAuth state secret must be nonempty, unpadded and sufficiently long".into());
        }
        if self.mode != SecurityMode::IsolatedTest {
            let lower = key.to_ascii_lowercase();
            let placeholder = ["change-me", "changeme", "placeholder", "example", "test", "secret"]
                .iter().any(|marker| lower.contains(marker));
            let distinct = key.bytes().collect::<std::collections::HashSet<_>>().len();
            let repeated = (1..=key.len() / 2).any(|period| {
                key.len() % period == 0 && key.as_bytes().chunks(period).all(|chunk| chunk == &key.as_bytes()[..period])
            });
            if key.len() < 32 || distinct < 8 || placeholder || repeated
                || (self.mode == SecurityMode::Verified && key == LOCAL_OAUTH_STATE_SECRET)
            {
                return Err("OAuth state secret must be provisioned (at least 32 bytes, not a placeholder)".into());
            }
        }
        Ok(OAuthStateSecret(key.as_bytes().to_vec()))
    }
}

/// Validate a connector base URL without reflecting its contents into errors.
///
/// # Errors
/// Rejects malformed URLs, credentials, queries/fragments, or plaintext in verified mode.
pub fn validate_connector_url(raw: &str, mode: SecurityMode) -> Result<url::Url, &'static str> {
    let url = url::Url::parse(raw).map_err(|_| "invalid IdP connector URL")?;
    if raw.trim() != raw || url.host_str().is_none() || !url.username().is_empty()
        || url.password().is_some() || url.query().is_some() || url.fragment().is_some()
        || !matches!(url.scheme(), "https" | "http")
    {
        return Err("invalid IdP connector URL");
    }
    if mode == SecurityMode::Verified && url.scheme() != "https" {
        return Err("IdP connectors require verified HTTPS; local plaintext needs explicit policy");
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_defaults_require_explicit_mesh_declaration() {
        let config: AuthRateLimitConfig = serde_json::from_str("{}").unwrap();
        assert!(config.trusted_proxies.is_empty());
        assert!(!config.disabled);
        assert_eq!(config.source_limit, 30);
        let proxy: TrustedAuthProxy = serde_json::from_str(
            r#"{"ip":"10.244.0.0","prefix_len":16,"dns_san":"envoy-mesh"}"#,
        ).unwrap();
        assert!(proxy.is_valid());
        assert!(proxy.matches_peer("10.244.2.19".parse().unwrap()));
        assert!(!proxy.matches_peer("10.245.2.19".parse().unwrap()));
        assert!(!proxy.matches_peer("::ffff:10.244.2.19".parse().unwrap()));
        let exact: TrustedAuthProxy = serde_json::from_str(
            r#"{"ip":"10.244.2.19","dns_san":"envoy-mesh"}"#,
        ).unwrap();
        assert!(exact.matches_peer("10.244.2.19".parse().unwrap()));
        assert!(!exact.matches_peer("10.244.2.20".parse().unwrap()));
    }

    #[test]
    fn proxy_networks_reject_universal_invalid_and_noncanonical_prefixes() {
        for (ip, prefix) in [("0.0.0.0", 0), ("10.244.0.0", 33), ("10.244.0.1", 16),
            ("::", 0), ("2001:db8::", 129), ("2001:db8::1", 64)] {
            let proxy = TrustedAuthProxy { ip: ip.parse().unwrap(), prefix_len: Some(prefix), dns_san: "envoy-mesh".into() };
            assert!(!proxy.is_valid());
            assert!(!proxy.matches_peer(proxy.ip));
        }
        let proxy = TrustedAuthProxy { ip: "2001:db8::".parse().unwrap(), prefix_len: Some(64), dns_san: "envoy-mesh".into() };
        assert!(proxy.matches_peer("2001:db8::19".parse().unwrap()));
        assert!(!proxy.matches_peer("2001:db8:0:1::19".parse().unwrap()));
    }

    #[test]
    fn verified_keys_reject_empty_weak_placeholders_and_local_key() {
        let config = SecurityConfig::default();
        for key in ["", " ", "iam-oauth-state-hmac-change-me", "iam-oauth-state-hmac-test", LOCAL_OAUTH_STATE_SECRET,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "0123456789abcdef0123456789abcdef", "placeholder-key-0123456789abcdef0123456789"] {
            assert!(config.validate_oauth_state_secret(key).is_err());
        }
        assert!(config.validate_oauth_state_secret("730bef2964a182f60bd87491e36dc5209ac472ee85fa61903d62b7084f19ca35").is_ok());
    }

    #[test]
    fn exceptions_are_explicit_and_still_never_allow_empty_key() {
        let test = SecurityConfig { mode: SecurityMode::IsolatedTest, ..SecurityConfig::default() };
        assert!(test.validate_oauth_state_secret("iam-oauth-state-hmac-test").is_ok());
        assert!(test.validate_oauth_state_secret("").is_err());
        let local = SecurityConfig { mode: SecurityMode::LocalInsecure, ..SecurityConfig::default() };
        assert!(local.validate_oauth_state_secret(LOCAL_OAUTH_STATE_SECRET).is_ok());
        assert!(local.validate_oauth_state_secret("iam-oauth-state-hmac-change-me").is_err());
    }

    #[test]
    fn connector_transport_is_not_inferred_from_debug_or_local_hostname() {
        for url in ["http://localhost:3000/idp", "http://idp:8080/idp"] {
            assert!(validate_connector_url(url, SecurityMode::Verified).is_err());
            assert!(validate_connector_url(url, SecurityMode::LocalInsecure).is_ok());
        }
        for url in ["https://user:password@idp/idp", "https://idp/idp?credential=x", "file:///idp", "https://idp/idp#fragment"] {
            assert!(validate_connector_url(url, SecurityMode::LocalInsecure).is_err());
        }
        assert!(validate_connector_url("https://idp/idp", SecurityMode::Verified).is_ok());
    }
}
