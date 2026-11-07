//! Later integration seam between secret acquisition and the OAuth-only adapter.

use std::fmt;

use idp_connect_contract::HmacKey;
use oauth_connect_configuration::AppConfig;
use oauth_connect_infra::OAuthClient;
use rustycog::core::error::DomainError;

/// One complete OAuth construction result for a later HTTP composition consumer.
/// This is not a registry and does not publish routes or an operational service.
pub struct ConfiguredOAuthClient {
    /// Canonical logical instance slug associated with both client and HMAC key.
    pub id: String,
    /// Concrete v2 adapter for this instance only.
    pub client: OAuthClient,
    /// Existing contract key wrapper for this instance, never a global fallback.
    pub hmac_key: HmacKey,
}

impl fmt::Debug for ConfiguredOAuthClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ConfiguredOAuthClient([REDACTED])")
    }
}

/// Construct all requested OAuth instances atomically for a later composition.
/// A declaration needing the unsupported OIDC client must fail before any
/// acquisition/publication, never disappear from a partially built catalogue.
///
/// # Errors
/// Always refuses in T0 without I/O. No registry, listener or OIDC client exists.
pub async fn build_oauth_clients(
    _config: &AppConfig,
) -> Result<Vec<ConfiguredOAuthClient>, DomainError> {
    Err(DomainError::InvalidInput {
        message: "OAuthConnect OAuth composition not implemented".to_owned(),
    })
}
