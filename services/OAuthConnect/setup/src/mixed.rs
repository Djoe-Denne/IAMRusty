//! Additive mixed-profile startup seam, independent of the OAuth-only builder.

use std::{fmt, sync::Arc};

use idp_connect_contract::{FederatedOAuthClient, HmacKey};
use oauth_connect_configuration::AppConfig;
use rustycog::core::error::DomainError;

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
/// Always refuses in T0, before secret acquisition, discovery or client creation.
/// The separately published OAuth-only builder keeps its existing behavior.
pub async fn build_clients(_config: &AppConfig) -> Result<Vec<ConfiguredClient>, DomainError> {
    Err(DomainError::InvalidInput {
        message: "OAuthConnect mixed client construction not implemented".to_owned(),
    })
}
