//! B-builder local CA/SAN fixture, adapted from IAM's verified_idp_tls pattern.
//! No S2S protocol, legacy reqwest types, environment trust or invalid-cert bypass.

use axum::{extract::State, routing::get, Router};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose,
};
use std::{
    future::Future,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

fn ca() -> (Certificate, KeyPair) {
    let mut parameters = CertificateParams::new(Vec::new()).expect("CA parameters");
    parameters
        .distinguished_name
        .push(DnType::CommonName, "OAuthConnect B test root");
    parameters.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    parameters.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
    ];
    let key = KeyPair::generate().expect("isolated root key");
    (parameters.self_signed(&key).expect("isolated CA"), key)
}

/// Nonsecret request target, public roots and receipt counter for the test body.
pub struct View {
    /// Exact random-port request target with a tiny response.
    pub uri: String,
    /// Public signing CA PEM file consumed by the production builder.
    pub root: PathBuf,
    /// Independent public CA for the wrong-trust negative witness.
    pub unrelated_root: PathBuf,
    /// Handler receipts; handshake failures must leave it zero.
    pub count: Arc<AtomicUsize>,
}

async fn probe(
    State(count): State<Arc<AtomicUsize>>,
) -> ([(&'static str, &'static str); 1], &'static str) {
    count.fetch_add(1, Ordering::SeqCst);
    ([("content-type", "application/json")], "{\"ok\":true}")
}

struct Fixture {
    _pki: tempfile::TempDir,
    handle: axum_server::Handle,
    task: Option<tokio::task::JoinHandle<std::io::Result<()>>>,
}

impl Fixture {
    async fn start(matching_san: bool) -> (Self, View) {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let (root, root_key) = ca();
        let san = if matching_san {
            "127.0.0.1"
        } else {
            "wrong-san.example.invalid"
        };
        let mut leaf = CertificateParams::new(vec![san.to_owned()]).expect("leaf SAN");
        leaf.distinguished_name
            .push(DnType::CommonName, "OAuthConnect B TLS server");
        leaf.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        leaf.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let key = KeyPair::generate().expect("isolated leaf key");
        let leaf = leaf
            .signed_by(&key, &root, &root_key)
            .expect("root-signed leaf");
        let pki = tempfile::tempdir().expect("isolated PKI directory");
        let root_path = pki.path().join("ca.pem");
        let unrelated_path = pki.path().join("other-ca.pem");
        std::fs::write(&root_path, root.pem()).expect("public test root");
        std::fs::write(&unrelated_path, ca().0.pem()).expect("unrelated public test root");
        let tls = axum_server::tls_rustls::RustlsConfig::from_pem(
            leaf.pem().into_bytes(),
            key.serialize_pem().into_bytes(),
        )
        .await
        .expect("TLS configuration");
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("owned random-port listener");
        let address = listener.local_addr().expect("actual TLS address");
        let count = Arc::new(AtomicUsize::new(0));
        let router = Router::new()
            .route("/probe", get(probe))
            .with_state(count.clone());
        let handle = axum_server::Handle::new();
        let server_handle = handle.clone();
        let task = tokio::spawn(async move {
            axum_server::from_tcp_rustls(listener, tls)
                .handle(server_handle)
                .serve(router.into_make_service())
                .await
        });
        let mut fixture = Self {
            _pki: pki,
            handle,
            task: Some(task),
        };
        let ready = tokio::time::timeout(Duration::from_secs(5), fixture.handle.listening()).await;
        if !matches!(ready, Ok(Some(actual)) if actual == address) {
            let _ = fixture.shutdown().await;
            panic!("B TLS fixture never became ready");
        }
        let view = View {
            uri: format!("https://{address}/probe"),
            root: root_path,
            unrelated_root: unrelated_path,
            count,
        };
        (fixture, view)
    }

    async fn shutdown(&mut self) -> bool {
        self.handle.graceful_shutdown(Some(Duration::from_secs(1)));
        let Some(mut task) = self.task.take() else {
            return true;
        };
        match tokio::time::timeout(Duration::from_secs(4), &mut task).await {
            Ok(Ok(Ok(()))) => true,
            Ok(_) => false,
            Err(_) => {
                task.abort();
                let _ = task.await;
                false
            }
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Cancellation fallback only. Normal and assertion-panic paths go through joined shutdown.
        self.handle.shutdown();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

/// Join TLS before propagating a test assertion panic; never detach an owned server/task.
pub async fn run<F, Fut>(matching_san: bool, check: F)
where
    F: FnOnce(View) -> Fut + Send,
    Fut: Future<Output = ()> + Send + 'static,
{
    let (mut fixture, view) = Fixture::start(matching_san).await;
    let mut assertions = tokio::spawn(check(view));
    let result = tokio::time::timeout(Duration::from_secs(15), &mut assertions).await;
    if result.is_err() {
        assertions.abort();
        let _ = assertions.await;
    }
    let joined = fixture.shutdown().await;
    assert!(
        joined,
        "B TLS graceful shutdown/join failed; parent reconciliation required"
    );
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
        Ok(Err(_)) => panic!("B TLS assertion task was cancelled"),
        Err(_) => panic!("B TLS fixture watchdog fired; no transport result established"),
    }
}
