//! Declarative configuration only: no network, secret resolution or `JSONPath` engine.

use std::{collections::BTreeMap, fmt, path::PathBuf};

pub use oauth_connect_domain::{ApiId, ClaimSource, OAuthClaims};
pub use rustycog::config::secrets::{OpenBaoAuthConfig, OpenBaoConfig, SecretReference};
pub use rustycog::config::{AuthConfig, ConfigError, ConfigLoader, LoggingConfig, ServerConfig};
pub use rustycog::core::error::DomainError;
use serde::{Deserialize, Serialize};

mod validation;

/// Environment prefix passed to the existing strict SDK loader.
pub const CONFIG_PREFIX: &str = "OAUTH_CONNECT";

/// Explicit startup secret source, with no implicit fallback between modes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecretsMode {
    /// Directly configured values, exclusive of secret references.
    Direct,
    /// SDK KV v2 references resolved at startup by a later composition layer.
    Openbao,
}

/// Vendor transport policy; exceptions must never be inferred from build mode.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorTransportSecurity {
    /// Verified HTTPS, including any explicitly configured trusted roots.
    #[default]
    Verified,
    /// Explicit local transport exception, subject to declarative validation.
    LocalInsecure,
    /// Explicit isolated-test transport exception, not a production default.
    IsolatedTest,
}

/// Token endpoint client authentication, never detected or retried automatically.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenAuthMethod {
    /// Send client credentials in the token request form.
    ClientSecretPost,
    /// Send client credentials using HTTP Basic authentication.
    ClientSecretBasic,
}

/// Fixed authorization scheme for a configured GET JSON API.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessTokenAuth {
    /// Send the access token using the Bearer authorization scheme.
    Bearer,
    /// Send the access token using the token authorization scheme.
    Token,
}

/// One declarative GET JSON endpoint; no templates, body or request chaining.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonGetApi {
    /// Fixed absolute endpoint, subject to URL and transport validation.
    pub url: String,
    /// Authorization scheme used for this API's access token.
    pub access_token_auth: AccessTokenAuth,
    /// Nonsecret headers, contractually restricted to the allowed header names.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

/// Adaptable OAuth endpoints and typed claims, independent of instance slug.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OAuthProfile {
    /// Fixed authorization endpoint, without reserved OAuth query parameters.
    pub authorization_url: String,
    /// Fixed token endpoint; no endpoint is derived from a returned claim.
    pub token_url: String,
    /// Named GET JSON definitions referenced by the claim mappings.
    pub apis: BTreeMap<ApiId, JsonGetApi>,
    /// Minimal profile mappings; no scripts, rich claims or vendor branches.
    pub claims: OAuthClaims,
}

/// Declarative OIDC authority; this type does not validate tokens or run discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OidcProfile {
    /// Exact pinned issuer, not inferred from a token or the instance slug.
    pub issuer: String,
    /// Explicit issuer and endpoint origins permitted for later discovery checks.
    pub allowed_endpoint_origins: Vec<String>,
}

/// The discriminator is kept inside `profile`; no flatten/unknown-field escape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProviderProfile {
    /// Configurable OAuth GET JSON projection.
    Oauth(Box<OAuthProfile>),
    /// Standard OIDC authority with fixed profile claim semantics.
    Oidc(OidcProfile),
}

/// Unvalidated declaration. Secret values/references are not serialized or debugged.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    /// Logical authority slug, distinct from profile kind and runtime incarnation.
    pub id: String,
    /// Public OAuth client identifier for this instance.
    pub client_id: String,
    /// Exact allowed IAM callback and relink callback URIs.
    pub redirect_uris: Vec<String>,
    /// Explicit scope tokens; no vendor scope is added implicitly.
    pub scopes: Vec<String>,
    /// Explicit receiver capability; the OIDC contract requires S256 support.
    pub pkce_supported: bool,
    /// Required direct/reference selection, validated before secret resolution.
    pub secrets_mode: SecretsMode,
    /// Direct client secret; omitted from serialization and provider Debug.
    #[serde(default, skip_serializing)]
    pub client_secret: Option<String>,
    /// Direct instance HMAC key; omitted from serialization and provider Debug.
    #[serde(default, skip_serializing)]
    pub hmac_secret: Option<String>,
    /// SDK client-secret reference; its potentially sensitive path is not serialized.
    #[serde(default, skip_serializing)]
    pub client_secret_ref: Option<SecretReference>,
    /// SDK instance HMAC reference, never an implicit shared key or fallback.
    #[serde(default, skip_serializing)]
    pub hmac_secret_ref: Option<SecretReference>,
    /// Explicit vendor transport policy, defaulting to verified HTTPS.
    #[serde(default)]
    pub transport_security: VendorTransportSecurity,
    /// Optional additional trusted PEM roots; not permission to bypass verification.
    #[serde(default)]
    pub ca_cert_file: Option<PathBuf>,
    /// Explicit token endpoint authentication method for this instance.
    pub token_auth_method: TokenAuthMethod,
    /// Exactly one typed OAuth or OIDC declaration.
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
    /// Optional SDK secret transport/identity declaration, omitted from serialization.
    #[serde(default, skip_serializing)]
    pub openbao: Option<OpenBaoConfig>,
    /// Standard SDK server settings, retained for later HTTP composition.
    #[serde(default)]
    pub server: ServerConfig,
    /// Standard SDK authentication settings, not a vendor authentication policy.
    #[serde(default)]
    pub auth: AuthConfig,
    /// Standard SDK logging settings; the whole application Debug remains redacted.
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Complete instance catalogue; validation must reject empty or invalid lists.
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
/// Returns a sanitized SDK error for invalid sources or declarations. No secrets
/// are resolved and no endpoints are contacted by this loader.
pub fn load_config() -> Result<AppConfig, ConfigError> {
    let mut config = rustycog::config::load_config_fresh_strict::<AppConfig>()?;
    validate_config(&config)
        .map_err(|_| ConfigError::Message("Invalid OAuthConnect configuration".to_owned()))?;
    for provider in &mut config.providers {
        provider.id.make_ascii_lowercase();
    }
    Ok(config)
}

/// Validate declarations only; `JSONPath` AST checks belong to infra compilation.
///
/// # Errors
/// Returns a sanitized error for an invalid catalogue, endpoint, scope, API
/// declaration or secret-source shape. This does not validate `JSONPath` ASTs,
/// resolved secret values, discovery metadata or tokens.
pub fn validate_config(config: &AppConfig) -> Result<(), DomainError> {
    validation::validate(config)
}
