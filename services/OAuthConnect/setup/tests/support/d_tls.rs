//! C-local adaptation of `c_tls`: real OIDC discovery/JWKS/token over verified TLS.
use super::d_vendor::{jwks, metadata, token};
use axum::{
    extract::{OriginalUri, State},
    routing::{get, post},
    Json, Router,
};
use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose,
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

#[derive(Clone)]
struct Documents {
    origin: String,
    hits: Arc<AtomicUsize>,
}
async fn document(
    State(state): State<Documents>,
    OriginalUri(uri): OriginalUri,
) -> Json<serde_json::Value> {
    state.hits.fetch_add(1, Ordering::SeqCst);
    Json(match uri.path() {
        "/.well-known/openid-configuration" => metadata(&state.origin),
        "/keys" => jwks(),
        "/token" => token(&state.origin),
        _ => unreachable!("only three exact fixture routes are installed"),
    })
}
pub struct View {
    pub origin: String,
    pub ca: PathBuf,
    pub hits: Arc<AtomicUsize>,
}
struct Fixture {
    _files: tempfile::TempDir,
    handle: axum_server::Handle,
    task: Option<tokio::task::JoinHandle<std::io::Result<()>>>,
}
impl Fixture {
    async fn new(matching_san: bool) -> (Self, View) {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let mut parameters = CertificateParams::new(Vec::new()).expect("fixture CA parameters");
        // Like c_tls and IAM's verified_idp_tls, keep issuer/subject distinct:
        // identical rcgen defaults make the signed leaf appear self-issued.
        parameters
            .distinguished_name
            .push(DnType::CommonName, "OAuthConnect D root");
        parameters.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        parameters.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::CrlSign,
        ];
        let root_key = KeyPair::generate().expect("fixture root key");
        let root = parameters
            .self_signed(&root_key)
            .expect("fixture root certificate");
        let san = if matching_san {
            "127.0.0.1"
        } else {
            "wrong.example.invalid"
        };
        let mut leaf = CertificateParams::new(vec![san.to_owned()]).expect("fixture leaf SAN");
        leaf.distinguished_name
            .push(DnType::CommonName, "OAuthConnect D leaf");
        leaf.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        leaf.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let key = KeyPair::generate().expect("fixture leaf key");
        let cert = leaf
            .signed_by(&key, &root, &root_key)
            .expect("fixture signed leaf");
        let files = tempfile::tempdir().expect("isolated TLS files");
        let ca = files.path().join("ca.pem");
        std::fs::write(&ca, root.pem()).expect("fixture public CA");
        let tls = axum_server::tls_rustls::RustlsConfig::from_pem(
            cert.pem().into_bytes(),
            key.serialize_pem().into_bytes(),
        )
        .await
        .expect("fixture TLS configuration");
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("owned TLS random-port listener");
        let address = listener.local_addr().expect("owned TLS address");
        let origin = format!("https://{address}");
        let hits = Arc::new(AtomicUsize::new(0));
        let router = Router::new()
            .route("/.well-known/openid-configuration", get(document))
            .route("/keys", get(document))
            .route("/token", post(document))
            .with_state(Documents {
                origin: origin.clone(),
                hits: hits.clone(),
            });
        let handle = axum_server::Handle::new();
        let owned = handle.clone();
        let task = tokio::spawn(async move {
            axum_server::from_tcp_rustls(listener, tls)
                .handle(owned)
                .serve(router.into_make_service())
                .await
        });
        let mut fixture = Self {
            _files: files,
            handle,
            task: Some(task),
        };
        if !matches!(tokio::time::timeout(Duration::from_secs(5), fixture.handle.listening()).await,
            Ok(Some(actual)) if actual == address)
        {
            let _ = fixture.close().await;
            panic!("TLS fixture readiness failed, not a product RED");
        }
        (fixture, View { origin, ca, hits })
    }
    async fn close(&mut self) -> bool {
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
pub async fn run<F, Fut>(matching_san: bool, check: F)
where
    F: FnOnce(View) -> Fut + Send,
    Fut: Future<Output = ()> + Send + 'static,
{
    let (mut fixture, view) = Fixture::new(matching_san).await;
    let mut assertions = tokio::spawn(check(view));
    let result = tokio::time::timeout(Duration::from_secs(20), &mut assertions).await;
    if result.is_err() {
        assertions.abort();
        let _ = assertions.await;
    }
    assert!(fixture.close().await, "TLS fixture shutdown/join failed");
    result
        .unwrap_or_else(|_| panic!("TLS test watchdog fired"))
        .unwrap_or_else(|e| {
            if e.is_panic() {
                std::panic::resume_unwind(e.into_panic());
            }
            panic!("TLS assertion task cancelled");
        });
}
