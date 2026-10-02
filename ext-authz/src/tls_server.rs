//! TLS listener: client certificates are required. Cleartext is not served.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::RootCertStore;

/// Serve Check over mTLS. Missing certificate material refuses to start.
///
/// # Errors
///
/// Returns an error when TLS files are missing or the handshake config is rejected.
pub async fn serve(app: Router, port: u16) -> Result<(), String> {
    let cert_path = required_env("EXT_AUTHZ_TLS_CERT")?;
    let key_path = required_env("EXT_AUTHZ_TLS_KEY")?;
    let ca_path = required_env("EXT_AUTHZ_TLS_CA")?;
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let certs = load_certs(&cert_path)?;
    let key = load_key(&key_path)?;
    let roots = load_ca(&ca_path)?;
    let verifier = WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .map_err(|err| err.to_string())?;
    let mut server = rustls::ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(certs, key)
        .map_err(|err| err.to_string())?;
    server.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    let tls = RustlsConfig::from_config(Arc::new(server));
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    axum_server::bind(addr)
        .acceptor(axum_server::tls_rustls::RustlsAcceptor::new(tls))
        .serve(app.into_make_service())
        .await
        .map_err(|err| err.to_string())
}

fn required_env(name: &str) -> Result<String, String> {
    let value = std::env::var(name).unwrap_or_default();
    if value.trim().is_empty() {
        return Err(format!("{name} is required (fail-closed)"));
    }
    Ok(value)
}

fn load_certs(path: &str) -> Result<Vec<CertificateDer<'static>>, String> {
    CertificateDer::pem_file_iter(path)
        .map_err(|err| format!("read cert {path}: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("parse cert {path}: {err}"))
}

fn load_key(path: &str) -> Result<PrivateKeyDer<'static>, String> {
    PrivateKeyDer::from_pem_file(path).map_err(|err| format!("read key {path}: {err}"))
}

fn load_ca(path: &str) -> Result<RootCertStore, String> {
    let mut roots = RootCertStore::empty();
    let certs = CertificateDer::pem_file_iter(path)
        .map_err(|err| format!("read CA {path}: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("parse CA {path}: {err}"))?;
    if certs.is_empty() {
        return Err(format!("no certificates in {path}"));
    }
    for cert in certs {
        roots
            .add(cert)
            .map_err(|err| format!("invalid CA certificate: {err}"))?;
    }
    Ok(roots)
}
