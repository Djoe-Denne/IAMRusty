//! Startup-only HTTP construction; no permissive test/production fallback.

use oauth_connect_configuration::ProviderConfig;
use reqwest::Client;
use rustycog::core::error::DomainError;

/// Build one injected HTTP client using the provider's explicit transport/CA mode.
/// Future construction must preserve verification, refuse redirects and apply
/// infra's fixed timeouts; the adapter must additionally bound decompressed reads.
///
/// # Errors
/// Always refuses in T0, without reading CA files or constructing a transport.
pub fn build_http_client(_provider: &ProviderConfig) -> Result<Client, DomainError> {
    Err(DomainError::InvalidInput {
        message: "OAuthConnect HTTP client construction not implemented".to_owned(),
    })
}
