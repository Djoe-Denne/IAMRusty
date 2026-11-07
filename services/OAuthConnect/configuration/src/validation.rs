//! Declarative checks only; errors intentionally contain no configuration data.

use std::collections::BTreeSet;

use url::Url;

use crate::{
    AppConfig, DomainError, JsonGetApi, OAuthProfile, OidcProfile, OpenBaoAuthConfig,
    OpenBaoConfig, ProviderConfig, ProviderProfile, SecretsMode, VendorTransportSecurity,
};

// Three allowlisted header names, each with a bounded, nonsecret static value.
const HEADER_VALUE_LIMIT: usize = 1024;

fn invalid() -> DomainError {
    DomainError::InvalidInput {
        message: "Invalid OAuthConnect configuration".to_owned(),
    }
}

pub fn validate(config: &AppConfig) -> Result<(), DomainError> {
    if config.providers.is_empty() {
        return Err(invalid());
    }
    if let Some(openbao) = &config.openbao {
        validate_openbao(openbao)?;
    }
    let mut ids = BTreeSet::new();
    for provider in &config.providers {
        if !ids.insert(provider.id.to_ascii_lowercase()) {
            return Err(invalid());
        }
        validate_provider(provider, config.openbao.is_some())?;
    }
    Ok(())
}

pub fn validate_provider(provider: &ProviderConfig, has_openbao: bool) -> Result<(), DomainError> {
    if provider.id.is_empty()
        || provider.id.len() > 50
        || !provider.id.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return Err(invalid());
    }
    if provider.client_id.trim().is_empty() || provider.redirect_uris.is_empty() {
        return Err(invalid());
    }
    validate_scopes(&provider.scopes)?;
    validate_secrets(provider, has_openbao)?;
    for redirect in &provider.redirect_uris {
        endpoint(redirect, provider.transport_security)?;
    }
    match &provider.profile {
        ProviderProfile::Oauth(profile) => validate_oauth(profile, provider.transport_security),
        ProviderProfile::Oidc(profile) => validate_oidc(profile, provider),
    }
}

fn validate_scopes(scopes: &[String]) -> Result<(), DomainError> {
    let mut seen = BTreeSet::new();
    if scopes.is_empty()
        || scopes.iter().any(|scope| {
            scope.is_empty()
                || scope.chars().any(|c| c.is_whitespace() || c.is_control())
                || !seen.insert(scope)
        })
    {
        return Err(invalid());
    }
    Ok(())
}

fn validate_secrets(provider: &ProviderConfig, has_openbao: bool) -> Result<(), DomainError> {
    match provider.secrets_mode {
        SecretsMode::Direct => {
            let client = provider.client_secret.as_deref().ok_or_else(invalid)?;
            let hmac = provider.hmac_secret.as_deref().ok_or_else(invalid)?;
            if client.trim().is_empty()
                || hmac.trim().is_empty()
                || hmac.len() < 16
                || provider.client_secret_ref.is_some()
                || provider.hmac_secret_ref.is_some()
            {
                return Err(invalid());
            }
        }
        SecretsMode::Openbao => {
            // SecretReference has private fields and validating SDK constructors/
            // deserialization; a present reference cannot bypass those checks.
            if !has_openbao
                || provider.client_secret_ref.is_none()
                || provider.hmac_secret_ref.is_none()
                || provider.client_secret.is_some()
                || provider.hmac_secret.is_some()
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

fn has_template(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.contains(['{', '}']) || lower.contains("%7b") || lower.contains("%7d")
}

fn endpoint(raw: &str, security: VendorTransportSecurity) -> Result<Url, DomainError> {
    if raw.chars().any(|c| c.is_whitespace() || c.is_control())
        || raw.contains('\\')
        || has_template(raw)
    {
        return Err(invalid());
    }
    let url = Url::parse(raw).map_err(|_| invalid())?;
    // Also refuse explicit empty userinfo, which URL normalization may erase.
    let authority = raw
        .split_once("://")
        .and_then(|(_, rest)| rest.split(['/', '?', '#']).next())
        .ok_or_else(invalid)?;
    if url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || authority.is_empty()
        || authority.contains('@')
        || url.fragment().is_some()
        || !(url.scheme() == "https"
            || (url.scheme() == "http" && security != VendorTransportSecurity::Verified))
    {
        return Err(invalid());
    }
    Ok(url)
}

fn origin(raw: &str, security: VendorTransportSecurity) -> Result<Url, DomainError> {
    let url = endpoint(raw, security)?;
    let path = raw
        .split_once("://")
        .and_then(|(_, rest)| rest.find('/').map(|index| &rest[index..]));
    if url.query().is_some() || path.is_some_and(|path| path != "/") {
        return Err(invalid());
    }
    Ok(url)
}

/// Whether `name` is one of the reserved OAuth authorization query parameters.
#[must_use]
pub fn is_reserved_authorization_parameter(name: &str) -> bool {
    matches!(
        name,
        "state"
            | "redirect_uri"
            | "client_id"
            | "scope"
            | "response_type"
            | "code_challenge"
            | "code_challenge_method"
            | "code_verifier"
            | "nonce"
    )
}

fn validate_oauth(
    profile: &OAuthProfile,
    security: VendorTransportSecurity,
) -> Result<(), DomainError> {
    let authorize = endpoint(&profile.authorization_url, security)?;
    if authorize
        .query_pairs()
        .any(|(key, _)| is_reserved_authorization_parameter(key.as_ref()))
    {
        return Err(invalid());
    }
    endpoint(&profile.token_url, security)?;
    validate_api_usage(profile)?;
    for api in profile.apis.values() {
        endpoint(&api.url, security)?;
        validate_headers(api)?;
    }
    Ok(())
}

fn validate_api_usage(profile: &OAuthProfile) -> Result<(), DomainError> {
    if profile.apis.is_empty() || profile.apis.keys().any(|id| id.trim().is_empty()) {
        return Err(invalid());
    }
    let claims = &profile.claims;
    let mut used = BTreeSet::new();
    for source in [
        &claims.sub,
        &claims.email,
        &claims.email_verified,
        &claims.preferred_username,
    ]
    .into_iter()
    .chain(claims.picture.iter())
    {
        if !profile.apis.contains_key(&source.api) || source.json_path.trim().is_empty() {
            return Err(invalid());
        }
        used.insert(&source.api);
    }
    if used.len() != profile.apis.len() {
        return Err(invalid());
    }
    Ok(())
}

fn validate_headers(api: &JsonGetApi) -> Result<(), DomainError> {
    let mut seen = BTreeSet::new();
    for (name, value) in &api.headers {
        let canonical = name.to_ascii_lowercase();
        if !matches!(
            canonical.as_str(),
            "accept" | "user-agent" | "x-github-api-version"
        ) || !seen.insert(canonical)
            || value.len() > HEADER_VALUE_LIMIT
            || value.chars().any(char::is_control)
            || has_template(value)
        {
            return Err(invalid());
        }
    }
    Ok(())
}

fn validate_oidc(profile: &OidcProfile, provider: &ProviderConfig) -> Result<(), DomainError> {
    if !provider.pkce_supported || !provider.scopes.iter().any(|scope| scope == "openid") {
        return Err(invalid());
    }
    let issuer = endpoint(&profile.issuer, provider.transport_security)?;
    if issuer.query().is_some() {
        return Err(invalid());
    }
    let mut origins = BTreeSet::new();
    for allowed in &profile.allowed_endpoint_origins {
        let url = origin(allowed, provider.transport_security)?;
        origins.insert(url.origin().ascii_serialization());
    }
    if !origins.contains(&issuer.origin().ascii_serialization()) {
        return Err(invalid());
    }
    Ok(())
}

fn validate_openbao(config: &OpenBaoConfig) -> Result<(), DomainError> {
    // SDK endpoint() is private. Mirror its declarative origin/loopback policy;
    // never call connect(), which would read files and perform authentication.
    let url = origin(&config.endpoint, VendorTransportSecurity::IsolatedTest)?;
    let loopback = match url.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    };
    if url.scheme() == "http" && !(config.allow_loopback_http && loopback) {
        return Err(invalid());
    }
    match &config.auth {
        OpenBaoAuthConfig::TokenFile { path } => {
            if path.as_os_str().is_empty() || path.to_string_lossy().trim().is_empty() {
                return Err(invalid());
            }
        }
    }
    Ok(())
}
