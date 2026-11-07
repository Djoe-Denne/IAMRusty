//! Declarative configuration only: no network, secret resolution or JSONPath engine.

use std::{collections::BTreeMap, fmt, path::PathBuf};

pub use oauth_connect_domain::{ApiId, ClaimSource, OAuthClaims};
pub use rustycog::config::secrets::{OpenBaoAuthConfig, OpenBaoConfig, SecretReference};
pub use rustycog::config::{AuthConfig, ConfigError, ConfigLoader, LoggingConfig, ServerConfig};
pub use rustycog::core::error::DomainError;
use serde::{Deserialize, Serialize};

pub const CONFIG_PREFIX: &str = "OAUTH_CONNECT";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecretsMode {
    Direct,
    Openbao,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorTransportSecurity {
    #[default]
    Verified,
    LocalInsecure,
    IsolatedTest,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenAuthMethod {
    ClientSecretPost,
    ClientSecretBasic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessTokenAuth {
    Bearer,
    Token,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonGetApi {
    pub url: String,
    pub access_token_auth: AccessTokenAuth,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OAuthProfile {
    pub authorization_url: String,
    pub token_url: String,
    pub apis: BTreeMap<ApiId, JsonGetApi>,
    pub claims: OAuthClaims,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OidcProfile {
    pub issuer: String,
    pub allowed_endpoint_origins: Vec<String>,
}

/// The discriminator is kept inside `profile`; no flatten/unknown-field escape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProviderProfile {
    Oauth(OAuthProfile),
    Oidc(OidcProfile),
}

/// Unvalidated declaration. Secret values/references are not serialized or debugged.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    pub id: String,
    pub client_id: String,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    pub pkce_supported: bool,
    pub secrets_mode: SecretsMode,
    #[serde(default, skip_serializing)]
    pub client_secret: Option<String>,
    #[serde(default, skip_serializing)]
    pub hmac_secret: Option<String>,
    #[serde(default, skip_serializing)]
    pub client_secret_ref: Option<SecretReference>,
    #[serde(default, skip_serializing)]
    pub hmac_secret_ref: Option<SecretReference>,
    #[serde(default)]
    pub transport_security: VendorTransportSecurity,
    #[serde(default)]
    pub ca_cert_file: Option<PathBuf>,
    pub token_auth_method: TokenAuthMethod,
    pub profile: ProviderProfile,
}

impl fmt::Debug for ProviderConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProviderConfig([REDACTED])")
    }
}

/// Default exists only to satisfy the SDK trait; it is not a valid catalogue.
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    #[serde(default, skip_serializing)]
    pub openbao: Option<OpenBaoConfig>,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    pub providers: Vec<ProviderConfig>,
}

impl fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AppConfig([REDACTED])")
    }
}

impl ConfigLoader<Self> for AppConfig {
    fn create_default() -> Self {
        Self::default()
    }

    fn config_prefix() -> &'static str {
        CONFIG_PREFIX
    }
}

/// Load through the real fresh SDK loader, then validate the entire declaration.
///
/// # Errors
/// Returns a sanitized SDK error for invalid sources or unsupported validation.
/// T0 deliberately refuses even nominal declarations until the T1 RED exists.
pub fn load_config() -> Result<AppConfig, ConfigError> {
    let config = rustycog::config::load_config_fresh_strict::<AppConfig>()?;
    validate_config(&config)
        .map_err(|_| ConfigError::Message("OAuthConnect validation not implemented".to_owned()))?;
    Ok(config)
}

/// Validate declarations only; JSONPath AST checks belong to infra compilation.
///
/// # Errors
/// Always refuses in T0. No catalogue/URL/scope/secret validation is claimed.
pub fn validate_config(_config: &AppConfig) -> Result<(), DomainError> {
    Err(DomainError::InvalidInput {
        message: "OAuthConnect configuration validation not implemented".to_owned(),
    })
}
