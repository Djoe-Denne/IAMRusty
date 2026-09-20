//! T14b — IAM dual-bind HTTP+HTTPS with optional mesh client CA (no docker).

use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iam_http_server::{create_prefixed_router, SERVICE_PREFIX};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose,
};
use readiness::ReadinessProbe;
use rustycog::command::{CommandRegistry, GenericCommandService};
use rustycog::config::ServerConfig;
use rustycog::http::{serve_router, AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};

struct TestPki {
    _dir: tempfile::TempDir,
    server_cert_path: String,
    server_key_path: String,
    client_ca_path: String,
    client_identity_pem: Vec<u8>,
    foreign_identity_pem: Vec<u8>,
}

fn install_crypto() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

fn new_ca(common_name: &str) -> (Certificate, KeyPair) {
    let mut params = CertificateParams::new(Vec::new()).unwrap();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params
        .distinguished_name
        .push(DnType::CommonName, common_name);
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params.key_usages.push(KeyUsagePurpose::KeyCertSign);
    params.key_usages.push(KeyUsagePurpose::CrlSign);
    let key_pair = KeyPair::generate().unwrap();
    (params.self_signed(&key_pair).unwrap(), key_pair)
}

fn new_end_entity(
    sans: Vec<String>,
    common_name: &str,
    eku: ExtendedKeyUsagePurpose,
    issuer: Option<(&Certificate, &KeyPair)>,
) -> (Certificate, KeyPair) {
    let mut params = CertificateParams::new(sans).unwrap();
    params
        .distinguished_name
        .push(DnType::CommonName, common_name);
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params.extended_key_usages.push(eku);
    let key_pair = KeyPair::generate().unwrap();
    let cert = match issuer {
        Some((ca, ca_key)) => params.signed_by(&key_pair, ca, ca_key).unwrap(),
        None => params.self_signed(&key_pair).unwrap(),
    };
    (cert, key_pair)
}

fn identity_pem(cert: &Certificate, key: &KeyPair) -> Vec<u8> {
    format!("{}{}", cert.pem(), key.serialize_pem()).into_bytes()
}

fn write_pem(path: &Path, contents: &str) -> String {
    std::fs::write(path, contents).unwrap();
    path.to_string_lossy().into_owned()
}

fn generate_pki() -> TestPki {
    let dir = tempfile::tempdir().unwrap();
    let (ca_cert, ca_key) = new_ca("rustycog-test-client-ca");
    let (foreign_ca, foreign_ca_key) = new_ca("rustycog-foreign-client-ca");
    let (server_cert, server_key) = new_end_entity(
        vec!["127.0.0.1".into(), "localhost".into()],
        "rustycog-test-server",
        ExtendedKeyUsagePurpose::ServerAuth,
        None,
    );
    let (client_cert, client_key) = new_end_entity(
        vec!["mtls-client.test".into()],
        "mtls-client",
        ExtendedKeyUsagePurpose::ClientAuth,
        Some((&ca_cert, &ca_key)),
    );
    let (foreign_cert, foreign_key) = new_end_entity(
        vec!["foreign-client.test".into()],
        "foreign-client",
        ExtendedKeyUsagePurpose::ClientAuth,
        Some((&foreign_ca, &foreign_ca_key)),
    );

    let server_cert_path = write_pem(&dir.path().join("server.crt"), &server_cert.pem());
    let server_key_path = write_pem(&dir.path().join("server.key"), &server_key.serialize_pem());
    let client_ca_path = write_pem(&dir.path().join("client-ca.crt"), &ca_cert.pem());

    TestPki {
        server_cert_path,
        server_key_path,
        client_ca_path,
        client_identity_pem: identity_pem(&client_cert, &client_key),
        foreign_identity_pem: identity_pem(&foreign_cert, &foreign_key),
        _dir: dir,
    }
}

fn ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn app_state() -> AppState {
    let command_service = Arc::new(GenericCommandService::new(Arc::new(
        CommandRegistry::default(),
    )));
    let extractor =
        UserIdExtractor::from_resolved_secret("rustycog-test-hs256-secret").expect("jwt");
    let checker: Arc<dyn PermissionChecker> = Arc::new(InMemoryPermissionChecker::new());
    AppState::new(command_service, extractor, checker)
}

fn dual_bind_config(
    pki: &TestPki,
    cleartext_port: u16,
    tls_listen: u16,
    tls_client_ca_path: String,
) -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".into(),
        port: cleartext_port,
        tls_enabled: true,
        tls_cert_path: pki.server_cert_path.clone(),
        tls_key_path: pki.server_key_path.clone(),
        tls_client_ca_path,
        tls_port: tls_listen,
    }
}

fn https_client(identity_pem: Option<&[u8]>) -> reqwest::Client {
    // Workspace reqwest still enables default-tls; rustycog tests disable it.
    // Force rustls so PEM client identities are not applied via native-tls.
    let mut builder = reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .timeout(Duration::from_secs(5));
    if let Some(pem) = identity_pem {
        builder = builder.identity(reqwest::Identity::from_pem(pem).unwrap());
    }
    builder.build().unwrap()
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap()
}

async fn spawn_server(config: ServerConfig) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    let router = create_prefixed_router(app_state(), Arc::new(ReadinessProbe::new("iam")));
    tokio::spawn(async move { serve_router(router, config).await })
}

async fn wait_until_ready(
    handle: &tokio::task::JoinHandle<anyhow::Result<()>>,
    client: &reqwest::Client,
    url: &str,
) {
    let start = Instant::now();
    loop {
        if handle.is_finished() {
            panic!("TLS server exited before becoming ready");
        }
        match client.get(url).send().await {
            Ok(_) => return,
            Err(err) => {
                if start.elapsed() > Duration::from_secs(5) {
                    panic!("TLS server not ready after 5s: {err}");
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}

async fn assert_2xx(client: &reqwest::Client, url: &str) {
    let response = client.get(url).send().await.unwrap();
    assert!(
        response.status().is_success(),
        "expected 2xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn dual_bind_http_and_optional_mtls() {
    install_crypto();
    let pki = generate_pki();
    let cleartext_port = ephemeral_port();
    let tls_listen = ephemeral_port();
    let health = format!("{SERVICE_PREFIX}/health");
    let cleartext_url = format!("http://127.0.0.1:{cleartext_port}{health}");
    let tls_url = format!("https://127.0.0.1:{tls_listen}{health}");
    let handle = spawn_server(dual_bind_config(
        &pki,
        cleartext_port,
        tls_listen,
        pki.client_ca_path.clone(),
    ))
    .await;

    let plain = http_client();
    wait_until_ready(&handle, &plain, &cleartext_url).await;
    let tls_probe = https_client(None);
    wait_until_ready(&handle, &tls_probe, &tls_url).await;

    assert_2xx(&plain, &cleartext_url).await;
    assert_2xx(&tls_probe, &tls_url).await;

    let mesh = https_client(Some(&pki.client_identity_pem));
    assert_2xx(&mesh, &tls_url).await;

    let foreign = https_client(Some(&pki.foreign_identity_pem));
    match foreign.get(&tls_url).send().await {
        Err(_) => {}
        Ok(response) => assert!(
            !response.status().is_success(),
            "foreign client cert must not get 2xx, got {}",
            response.status()
        ),
    }

    handle.abort();
}
