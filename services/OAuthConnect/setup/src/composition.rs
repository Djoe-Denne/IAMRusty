//! Startup composition of resolved credentials and trusted OAuth HTTP clients.

use std::fmt;

use idp_connect_contract::HmacKey;
use oauth_connect_configuration::{validate_config, AppConfig, ProviderProfile};
use oauth_connect_infra::OAuthClient;
use rustycog::core::error::DomainError;

use crate::{build_http_client, resolve_secrets};

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
///
/// A declaration needing the unsupported OIDC client must fail before any
/// acquisition/publication, never disappear from a partially built catalogue.
///
/// # Errors
/// Refuses invalid catalogues and unsupported profiles before acquisition, then
/// fails closed on secret resolution, HTTP construction or OAuth mapping errors.
/// Errors contain no configuration or remote data; no partial collection escapes.
pub async fn build_oauth_clients(
    config: &AppConfig,
) -> Result<Vec<ConfiguredOAuthClient>, DomainError> {
    validate_config(config).map_err(|_| invalid())?;
    if config
        .providers
        .iter()
        .any(|provider| !matches!(&provider.profile, ProviderProfile::Oauth(_)))
    {
        return Err(DomainError::InvalidInput {
            message: "Unsupported OAuthConnect client profile".to_owned(),
        });
    }

    let bundles = resolve_secrets(config).await.map_err(|_| invalid())?;
    if bundles.len() != config.providers.len() {
        return Err(invalid());
    }
    let mut clients = Vec::with_capacity(config.providers.len());
    for (provider, credentials) in config.providers.iter().zip(bundles) {
        if credentials.id != provider.id.to_ascii_lowercase() {
            return Err(invalid());
        }
        let http = build_http_client(provider).map_err(|_| invalid())?;
        let client =
            OAuthClient::new(provider, credentials.client_secret, http).map_err(|_| invalid())?;
        // resolve_secrets already normalized and validated these exact HMAC bytes.
        // HmacKey is only the receiving wrapper: no second trim or fallback here.
        let hmac_key = HmacKey::new(credentials.hmac_secret.into_inner().into_bytes());
        clients.push(ConfiguredOAuthClient {
            id: credentials.id,
            client,
            hmac_key,
        });
    }
    Ok(clients)
}

fn invalid() -> DomainError {
    DomainError::InvalidInput {
        message: "OAuthConnect OAuth composition failed".to_owned(),
    }
}
