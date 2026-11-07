//! Startup-only HTTP construction; no permissive test/production fallback.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use oauth_connect_configuration::{
    validate_config, AppConfig, OpenBaoAuthConfig, OpenBaoConfig, ProviderConfig, SecretsMode,
    VendorTransportSecurity,
};
use oauth_connect_infra::{HTTP_CONNECT_TIMEOUT, HTTP_REQUEST_TIMEOUT};
use reqwest::Client;
use rustycog::core::error::DomainError;

const CA_CERT_LIMIT: u64 = 1024 * 1024;

/// Build one injected HTTP client using the provider's explicit transport/CA mode.
///
/// Verification is enabled by default. Redirects, retries and ambient proxies are
/// disabled; only gzip decompression is enabled, with infra's fixed timeouts.
/// Infra must separately bound the decompressed body before parsing.
///
/// # Errors
/// Returns a sanitized domain error for an invalid provider, an unreadable,
/// nonregular, oversized or invalid PEM roots file, or client construction failure.
pub fn build_http_client(provider: &ProviderConfig) -> Result<Client, DomainError> {
    validate_provider_declaration(provider)?;
    let insecure = provider.transport_security != VendorTransportSecurity::Verified;
    let mut builder = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(HTTP_REQUEST_TIMEOUT)
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .gzip(true)
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .danger_accept_invalid_certs(insecure)
        .danger_accept_invalid_hostnames(insecure);
    if let Some(path) = &provider.ca_cert_file {
        let pem = read_roots(path)?;
        let certificates = reqwest::Certificate::from_pem_bundle(&pem).map_err(|_| invalid())?;
        if certificates.is_empty() {
            return Err(invalid());
        }
        for certificate in certificates {
            builder = builder.add_root_certificate(certificate);
        }
    }
    builder.build().map_err(|_| invalid())
}

fn validate_provider_declaration(provider: &ProviderConfig) -> Result<(), DomainError> {
    // This API has no global OpenBao input and must not resolve secrets. Supply
    // a shape-only, in-memory block to reuse the entire provider validator while
    // preserving the original mode/references. No token file is opened here.
    // resolve_secrets validates the *actual* global block before any acquisition.
    let openbao = (provider.secrets_mode == SecretsMode::Openbao).then(|| OpenBaoConfig {
        endpoint: "https://declarative-validation.invalid".to_owned(),
        auth: OpenBaoAuthConfig::TokenFile {
            path: PathBuf::from("declarative-validation-only"),
        },
        allow_loopback_http: false,
        ca_cert_file: None,
    });
    let declaration = AppConfig {
        providers: vec![provider.clone()],
        openbao,
        ..AppConfig::default()
    };
    validate_config(&declaration).map_err(|_| invalid())
}

fn read_roots(path: &Path) -> Result<Vec<u8>, DomainError> {
    let metadata = std::fs::metadata(path).map_err(|_| invalid())?;
    if !metadata.is_file() || metadata.len() > CA_CERT_LIMIT {
        return Err(invalid());
    }
    let file = File::open(path).map_err(|_| invalid())?;
    let metadata = file.metadata().map_err(|_| invalid())?;
    if !metadata.is_file() || metadata.len() > CA_CERT_LIMIT {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    file.take(CA_CERT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() as u64 > CA_CERT_LIMIT {
        return Err(invalid());
    }
    Ok(bytes)
}

fn invalid() -> DomainError {
    DomainError::InvalidInput {
        message: "OAuthConnect HTTP client construction failed".to_owned(),
    }
}
