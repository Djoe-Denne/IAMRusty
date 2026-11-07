//! Additive mixed-profile startup seam, independent of the OAuth-only builder.

use std::{fmt, sync::Arc};

use idp_connect_contract::{FederatedOAuthClient, HmacKey};
use oauth_connect_configuration::{validate_config, AppConfig, ProviderProfile};
use oauth_connect_infra::{OAuthClient, OidcClient};
use rustycog::core::error::DomainError;

use crate::{build_http_client, resolve_secrets};

/// One fully initialized instance, with its own client and resolved HMAC key.
///
/// Debug hides the whole bundle; no serialization or mutable registry is added.
pub struct ConfiguredClient {
    /// Canonical logical instance slug, not its profile kind or runtime address.
    pub id: String,
    /// Existing vendor-neutral v2 port, implemented by the selected profile.
    pub client: Arc<dyn FederatedOAuthClient>,
    /// Existing key wrapper containing exactly the once-normalized secret bytes.
    pub hmac_key: HmacKey,
}

impl fmt::Debug for ConfiguredClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ConfiguredClient([REDACTED])")
    }
}

/// Construct the complete mixed-profile catalogue without exposing partial output.
///
/// # Errors
/// Refuses invalid declarations, secret acquisition, transport or profile startup
/// failures without exposing remote data or returning an incomplete catalogue.
pub async fn build_clients(config: &AppConfig) -> Result<Vec<ConfiguredClient>, DomainError> {
    validate_config(config).map_err(|_| invalid())?;
    let bundles = resolve_secrets(config).await.map_err(|_| invalid())?;
    if bundles.len() != config.providers.len() {
        return Err(invalid());
    }
    let mut clients = Vec::with_capacity(bundles.len());
    for (provider, credentials) in config.providers.iter().zip(bundles) {
        if credentials.id != provider.id.to_ascii_lowercase() {
            return Err(invalid());
        }
        let http = build_http_client(provider).map_err(|_| invalid())?;
        let client: Arc<dyn FederatedOAuthClient> = match &provider.profile {
            ProviderProfile::Oauth(_) => Arc::new(
                OAuthClient::new(provider, credentials.client_secret, http)
                    .map_err(|_| invalid())?,
            ),
            ProviderProfile::Oidc(_) => Arc::new(
                Box::pin(OidcClient::new(provider, credentials.client_secret, http))
                    .await
                    .map_err(|_| invalid())?,
            ),
        };
        clients.push(ConfiguredClient {
            id: credentials.id,
            client,
            // The resolver already trimmed and validated these exact bytes.
            hmac_key: HmacKey::new(credentials.hmac_secret.into_inner().into_bytes()),
        });
    }
    Ok(clients)
}

fn invalid() -> DomainError {
    DomainError::InvalidInput {
        message: "OAuthConnect mixed client construction failed".to_owned(),
    }
}
