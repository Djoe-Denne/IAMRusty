//! All-or-nothing startup acquisition through the existing SDK secret facilities.

use std::fmt;

use oauth_connect_configuration::AppConfig;
use rustycog::config::secrets::SecretValue;
use rustycog::core::error::DomainError;

/// Startup-resolved credentials for exactly one logical provider instance.
/// SDK values are not clonable or serializable; Debug hides the complete bundle.
pub struct ResolvedProviderSecrets {
    /// Instance identifier associated with these credentials, never a vendor kind.
    pub id: String,
    /// Resolved OAuth client credential, transferred to that instance's adapter.
    pub client_secret: SecretValue,
    /// Resolved IAM HMAC credential; must contain at least sixteen UTF-8 bytes.
    pub hmac_secret: SecretValue,
}

impl fmt::Debug for ResolvedProviderSecrets {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ResolvedProviderSecrets([REDACTED])")
    }
}

/// Resolve credentials for every declaration, OAuth or OIDC, or return no result.
/// Future acquisition uses SDK direct values or authenticated KV v2 only, after
/// validating the complete declaration; there is no ambient or direct fallback.
///
/// # Errors
/// Always refuses in T0 without reading files or contacting the secret service.
/// Later errors must remain sanitized and must not expose a partial result list.
pub async fn resolve_secrets(
    _config: &AppConfig,
) -> Result<Vec<ResolvedProviderSecrets>, DomainError> {
    Err(DomainError::InvalidInput {
        message: "OAuthConnect startup secret resolution not implemented".to_owned(),
    })
}
