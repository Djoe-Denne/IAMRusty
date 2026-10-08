//! T14b — IAM real-core dual-bind HTTP+HTTPS with optional mesh client CA.
//! Existing testcontainers harness, final-only execution under the parent lease.
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "support/owned_task.rs"]
mod owned_task;
mod utils;

use std::net::TcpListener;
use std::path::Path;
use std::time::{Duration, Instant};

use iam_http_server::SERVICE_PREFIX;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose,
};
use rustycog::config::ServerConfig;
use rustycog::http::serve_router;

struct TestPki {
    _dir: tempfile::TempDir,
    server_cert_path: String,
    server_key_path: String,
    client_ca_path: String,
    server_ca_pem: String,
    foreign_ca_pem: String,
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
    let (server_ca, server_ca_key) = new_ca("rustycog-test-server-ca");
    let (server_cert, server_key) = new_end_entity(
        vec!["127.0.0.1".into(), "localhost".into()],
        "rustycog-test-server",
        ExtendedKeyUsagePurpose::ServerAuth,
        Some((&server_ca, &server_ca_key)),
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
        server_ca_pem: server_ca.pem(),
        foreign_ca_pem: foreign_ca.pem(),
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
        tls_require_client_cert: false,
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

async fn spawn_server(
    fixture: &common::TestFixture,
    config: ServerConfig,
) -> owned_task::OwnedTask<anyhow::Result<()>> {
    let mut security = iam_configuration::security::SecurityConfig::default();
    security.mode = iam_configuration::security::SecurityMode::LocalInsecure;
    security.rate_limit.disabled = false;
    // Real shared composition root supplies all three mandatory HTTP dependencies;
    // no unrelated fake OAuth use-case or manually injected transport extensions.
    let app = common::build_test_iam_app(fixture, security)
        .await
        .expect("actual IAM core/security builder");
    let router = axum::Router::new().nest(iam_http_server::SERVICE_PREFIX, app.router());
    owned_task::spawn(async move { serve_router(router, config).await })
}

async fn wait_until_ready(
    handle: &tokio::task::JoinHandle<anyhow::Result<()>>,
    client: &reqwest::Client,
    url: &str,
) {
    let start = Instant::now();
    loop {
        assert!(
            !handle.is_finished(),
            "TLS server exited before becoming ready"
        );
        match client.get(url).send().await {
            Ok(_) => return,
            Err(err) => {
                assert!(
                    start.elapsed() <= Duration::from_secs(5),
                    "TLS server not ready after 5s: {err}"
                );
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
#[serial_test::serial]
async fn dual_bind_http_and_optional_mtls() {
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("owned protocol fixture");
    fixture_cleanup::run(
        &fixture,
        Box::pin(async {
            install_crypto();
            let pki = generate_pki();
            let cleartext_port = ephemeral_port();
            let tls_listen = ephemeral_port();
            let health = format!("{SERVICE_PREFIX}/health");
            let cleartext_url = format!("http://127.0.0.1:{cleartext_port}{health}");
            let tls_url = format!("https://127.0.0.1:{tls_listen}{health}");
            let handle = spawn_server(
                &fixture,
                dual_bind_config(&pki, cleartext_port, tls_listen, pki.client_ca_path.clone()),
            )
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
            let _ = handle.await;
        }),
    )
    .await;
}

#[tokio::test]
#[serial_test::serial]
async fn verified_https_no_client_ca_rejects_wrong_ca_and_wrong_server_san() {
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("owned protocol fixture");
    fixture_cleanup::run(
        &fixture,
        Box::pin(async {
            install_crypto();
            let pki = generate_pki();
            let cleartext_port = ephemeral_port();
            let tls_port = ephemeral_port();
            let handle = spawn_server(
                &fixture,
                dual_bind_config(&pki, cleartext_port, tls_port, String::new()),
            )
            .await;
            let url = format!("https://localhost:{tls_port}{SERVICE_PREFIX}/health");
            let verified = reqwest::Client::builder()
                .use_rustls_tls()
                .no_proxy()
                .add_root_certificate(
                    reqwest::Certificate::from_pem(pki.server_ca_pem.as_bytes())
                        .expect("server CA"),
                )
                .timeout(Duration::from_secs(5))
                .build()
                .expect("verified client");
            wait_until_ready(&handle, &verified, &url).await;
            let positive = verified.get(&url).send().await;
            let foreign = reqwest::Client::builder()
                .use_rustls_tls()
                .no_proxy()
                .tls_built_in_root_certs(false)
                .add_root_certificate(
                    reqwest::Certificate::from_pem(pki.foreign_ca_pem.as_bytes())
                        .expect("foreign CA"),
                )
                .timeout(Duration::from_secs(5))
                .build()
                .expect("wrong-CA client");
            let wrong_ca = foreign.get(&url).send().await;
            let wrong_name = reqwest::Client::builder()
                .use_rustls_tls()
                .no_proxy()
                .add_root_certificate(
                    reqwest::Certificate::from_pem(pki.server_ca_pem.as_bytes())
                        .expect("server CA"),
                )
                .resolve(
                    "wrong-san.test",
                    std::net::SocketAddr::from(([127, 0, 0, 1], tls_port)),
                )
                .timeout(Duration::from_secs(5))
                .build()
                .expect("wrong-SAN client");
            let wrong_san = wrong_name
                .get(format!(
                    "https://wrong-san.test:{tls_port}{SERVICE_PREFIX}/health"
                ))
                .send()
                .await;
            handle.abort();
            let _ = handle.await;
            assert!(positive
                .expect("trusted server without client certificate")
                .status()
                .is_success());
            assert!(wrong_ca.is_err(), "wrong server CA must fail before HTTP");
            assert!(
                wrong_san.is_err(),
                "trusted CA alone cannot bypass server SAN"
            );
        }),
    )
    .await;
}
