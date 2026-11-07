//! C TLS fixture adapts the proven B PKI/owned-task pattern to actual OAuth POST+GET.
//! No request client is constructed here; C's returned adapter is the sole consumer.

use super::c_flow::{emails, tokens, user, ACCESS, EMAIL_PATH, TOKEN_PATH, USER_PATH};
use axum::{
    body::Bytes,
    extract::State,
    routing::{get, post},
    Json, Router,
};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose,
};
use std::{
    future::Future,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

#[derive(Default)]
pub struct Receipts {
    pub token: AtomicUsize,
    pub user: AtomicUsize,
    pub emails: AtomicUsize,
    pub forms: Mutex<Vec<Vec<u8>>>,
}

pub struct View {
    pub origin: String,
    pub root: PathBuf,
    pub unrelated_root: PathBuf,
    pub receipts: Arc<Receipts>,
}

async fn token(State(state): State<Arc<Receipts>>, body: Bytes) -> Json<serde_json::Value> {
    state.token.fetch_add(1, Ordering::SeqCst);
    state
        .forms
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(body.to_vec());
    Json(tokens(ACCESS))
}
async fn profile(State(state): State<Arc<Receipts>>) -> Json<serde_json::Value> {
    state.user.fetch_add(1, Ordering::SeqCst);
    Json(user(71, "caseuser"))
}
async fn addresses(State(state): State<Arc<Receipts>>) -> Json<serde_json::Value> {
    state.emails.fetch_add(1, Ordering::SeqCst);
    Json(emails("primary@example.test"))
}

fn ca() -> (Certificate, KeyPair) {
    let mut p = CertificateParams::new(Vec::new()).expect("C CA parameters");
    p.distinguished_name
        .push(DnType::CommonName, "OAuthConnect C root");
    p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    p.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
    ];
    let key = KeyPair::generate().expect("C root key");
    (p.self_signed(&key).expect("C public root"), key)
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
        let mut leaf = CertificateParams::new(vec![san.to_owned()]).expect("C leaf SAN");
        leaf.distinguished_name
            .push(DnType::CommonName, "OAuthConnect C leaf");
        leaf.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        leaf.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let key = KeyPair::generate().expect("C leaf key");
        let leaf = leaf
            .signed_by(&key, &root, &root_key)
            .expect("C signed leaf");
        let pki = tempfile::tempdir().expect("C isolated PKI");
        let root_path = pki.path().join("ca.pem");
        let unrelated_root = pki.path().join("other-ca.pem");
        std::fs::write(&root_path, root.pem()).expect("C public CA file");
        std::fs::write(&unrelated_root, ca().0.pem()).expect("C unrelated public CA");
        let tls = axum_server::tls_rustls::RustlsConfig::from_pem(
            leaf.pem().into_bytes(),
            key.serialize_pem().into_bytes(),
        )
        .await
        .expect("C TLS configuration");
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("C owned random-port listener");
        let address = listener.local_addr().expect("C actual TLS address");
        let receipts = Arc::new(Receipts::default());
        let router = Router::new()
            .route(TOKEN_PATH, post(token))
            .route(USER_PATH, get(profile))
            .route(EMAIL_PATH, get(addresses))
            .with_state(receipts.clone());
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
            panic!("C TLS fixture failed readiness, not a product RED");
        }
        (
            fixture,
            View {
                origin: format!("https://{address}"),
                root: root_path,
                unrelated_root,
                receipts,
            },
        )
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
        self.handle.shutdown();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

/// Always join the owned TLS server before resuming a panic from the test body.
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
        "C TLS shutdown/join failed; parent reconciliation required"
    );
    result
        .unwrap_or_else(|_| panic!("C TLS test watchdog fired; no transport result proven"))
        .unwrap_or_else(|error| {
            if error.is_panic() {
                std::panic::resume_unwind(error.into_panic());
            }
            panic!("C TLS assertion task was cancelled");
        });
}
