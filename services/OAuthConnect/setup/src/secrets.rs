//! All-or-nothing startup acquisition through the existing SDK secret facilities.

use std::fmt;

use oauth_connect_configuration::{validate_config, AppConfig, ProviderConfig, SecretsMode};
use rustycog::config::secrets::{OpenBaoKv2Resolver, SecretResolver, SecretValue};
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
///
/// The complete declaration and direct credentials are checked before any I/O.
/// KV v2 uses the SDK bootstrap once per batch, only if a vault instance exists.
/// No ambient credentials, fallback mode or partial result is exposed.
///
/// # Errors
/// Returns a sanitized domain error for invalid declarations, SDK acquisition
/// failures or HMAC keys shorter than sixteen UTF-8 bytes after normalization.
pub async fn resolve_secrets(
    config: &AppConfig,
) -> Result<Vec<ResolvedProviderSecrets>, DomainError> {
    validate_config(config).map_err(|_| invalid())?;
    // Prepare every direct value first: an invalid later direct instance must
    // not cause even the first vault instance to bootstrap unnecessarily.
    let prepared = config
        .providers
        .iter()
        .map(|provider| match provider.secrets_mode {
            SecretsMode::Direct => direct_secrets(provider).map(Some),
            SecretsMode::Openbao => Ok(None),
        })
        .collect::<Result<Vec<_>, DomainError>>()?;

    let mut resolver = None;
    let mut bundles = Vec::with_capacity(config.providers.len());
    for (provider, direct) in config.providers.iter().zip(prepared) {
        if let Some(direct) = direct {
            bundles.push(direct);
            continue;
        }
        if resolver.is_none() {
            let declaration = config.openbao.as_ref().ok_or_else(invalid)?;
            resolver = Some(
                OpenBaoKv2Resolver::connect(declaration)
                    .await
                    .map_err(|_| invalid())?,
            );
        }
        let resolver = resolver.as_ref().ok_or_else(invalid)?;
        bundles.push(vault_secrets(provider, resolver).await?);
    }
    Ok(bundles)
}

fn direct_secrets(provider: &ProviderConfig) -> Result<ResolvedProviderSecrets, DomainError> {
    let client_secret = direct_value(provider.client_secret.as_deref())?;
    let hmac_secret = direct_value(provider.hmac_secret.as_deref())?;
    bundle(provider, client_secret, hmac_secret)
}

fn direct_value(value: Option<&str>) -> Result<SecretValue, DomainError> {
    // Validate the raw value (including the SDK size limit) before any trim.
    SecretValue::new(value.ok_or_else(invalid)?.to_owned()).map_err(|_| invalid())
}

async fn vault_secrets(
    provider: &ProviderConfig,
    resolver: &OpenBaoKv2Resolver,
) -> Result<ResolvedProviderSecrets, DomainError> {
    let client_reference = provider.client_secret_ref.as_ref().ok_or_else(invalid)?;
    let hmac_reference = provider.hmac_secret_ref.as_ref().ok_or_else(invalid)?;
    let client_secret = resolver
        .resolve(client_reference)
        .await
        .map_err(|_| invalid())?;
    let hmac_secret = resolver
        .resolve(hmac_reference)
        .await
        .map_err(|_| invalid())?;
    bundle(provider, client_secret, hmac_secret)
}

fn bundle(
    provider: &ProviderConfig,
    client_secret: SecretValue,
    hmac_secret: SecretValue,
) -> Result<ResolvedProviderSecrets, DomainError> {
    // SDK validation already checked the *raw* HMAC, including its 16 KiB limit.
    // Only this local HMAC normalization changes bytes; client_secret is untouched.
    let raw = hmac_secret.into_inner();
    let canonical = raw.trim();
    if canonical.len() < 16 {
        return Err(invalid());
    }
    Ok(ResolvedProviderSecrets {
        id: provider.id.to_ascii_lowercase(),
        client_secret,
        hmac_secret: SecretValue::new(canonical.to_owned()).map_err(|_| invalid())?,
    })
}

fn invalid() -> DomainError {
    DomainError::InvalidInput {
        message: "OAuthConnect startup secret resolution failed".to_owned(),
    }
}
