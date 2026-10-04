//! Typed outbound IdP TLS stand-in. Generated PKI follows SDK mtls_client_auth.rs.
//! No live keys, environment trust overrides, unsafe client flags or global fixture.
use crate::owned_task;
use axum::{
    body::Bytes,
    extract::{OriginalUri, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use idp_connect_contract::{
    dto::{
        AuthorizeRequest, AuthorizeResponse, ProfileRequest, TokenRequest, AUTHORIZE_PATH,
        PROFILE_PATH, TOKEN_PATH,
    },
    hmac::{unix_timestamp_secs, verify, SIGNATURE_HEADER, TIMESTAMP_HEADER},
};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose,
};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

pub const HMAC_SECRET: &str = "SentinelABC-isolated-HMAC-fixture-secret";
pub const VERIFIER: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ";
pub const STATE: &str = "SentinelABC-nonsecret-protocol-state";
pub const REDIRECT_URI: &str = "https://iam.example/iam/api/auth/github/callback";

#[derive(Clone, Default)]
pub struct Receipts {
    pub hmac_valid: usize,
    pub authorize_valid: usize,
    pub token_valid: usize,
    pub profile_valid: usize,
}
struct Protocol {
    requests: AtomicUsize,
    receipts: Mutex<Receipts>,
    redirect: Mutex<Option<String>>,
    challenge: String,
}

fn ca() -> (Certificate, KeyPair) {
    let mut parameters = CertificateParams::new(Vec::new()).expect("generated CA parameters");
    parameters.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    parameters.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
    ];
    let key = KeyPair::generate().expect("generated isolated CA key");
    (
        parameters
            .self_signed(&key)
            .expect("generated CA certificate"),
        key,
    )
}
pub fn unrelated_root() -> reqwest::Certificate {
    reqwest::Certificate::from_pem(ca().0.pem().as_bytes()).expect("unrelated fixture public CA")
}

async fn protocol(
    State(state): State<Arc<Protocol>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    state.requests.fetch_add(1, Ordering::SeqCst);
    let validated = (|| {
        let timestamp = headers
            .get(TIMESTAMP_HEADER)?
            .to_str()
            .ok()?
            .parse::<i64>()
            .ok()?;
        let signature = headers.get(SIGNATURE_HEADER)?.to_str().ok()?;
        let text = std::str::from_utf8(&body).ok()?;
        verify(
            HMAC_SECRET.as_bytes(),
            "POST",
            uri.path(),
            timestamp,
            text,
            signature,
            unix_timestamp_secs().ok()?,
        )
        .ok()?;
        Some(())
    })()
    .is_some();
    if !validated {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    state.receipts.lock().expect("receipt lock").hmac_valid += 1;
    if let Some(location) = state.redirect.lock().expect("redirect lock").clone() {
        return (
            StatusCode::TEMPORARY_REDIRECT,
            [(axum::http::header::LOCATION, location)],
        )
            .into_response();
    }
    if uri.path() == format!("/connect{AUTHORIZE_PATH}") {
        let Ok(request) = serde_json::from_slice::<AuthorizeRequest>(&body) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        let Ok(challenge) = request.pkce_challenge() else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        if request.state != STATE
            || request.redirect_uri != REDIRECT_URI
            || challenge.as_deref() != Some(state.challenge.as_str())
        {
            return StatusCode::BAD_REQUEST.into_response();
        }
        state.receipts.lock().expect("receipt lock").authorize_valid += 1;
        let mut url = url::Url::parse("https://provider.example/authorize")
            .expect("fixed nonsecret vendor URL");
        url.query_pairs_mut()
            .append_pair("state", STATE)
            .append_pair("redirect_uri", REDIRECT_URI)
            .append_pair("code_challenge", &state.challenge)
            .append_pair("code_challenge_method", "S256");
        return Json(AuthorizeResponse {
            authorization_url: url.to_string(),
            scope: "profile".into(),
        })
        .into_response();
    }
    if uri.path() == format!("/connect{TOKEN_PATH}") {
        let Ok(request) = serde_json::from_slice::<TokenRequest>(&body) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        if request.code != "fixed-test-code"
            || request.redirect_uri != REDIRECT_URI
            || request.code_verifier.as_deref() != Some(VERIFIER)
        {
            return StatusCode::BAD_REQUEST.into_response();
        }
        state.receipts.lock().expect("receipt lock").token_valid += 1;
        return Json(crate::fixtures::idp_connect::resources::success_tokens()).into_response();
    }
    if uri.path() == format!("/connect{PROFILE_PATH}") {
        let Ok(request) = serde_json::from_slice::<ProfileRequest>(&body) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        if request.access_token
            != crate::fixtures::idp_connect::resources::success_tokens().access_token
        {
            return StatusCode::UNAUTHORIZED.into_response();
        }
        state.receipts.lock().expect("receipt lock").profile_valid += 1;
        return Json(crate::fixtures::idp_connect::resources::github_arthur()).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

pub struct VerifiedIdpTls {
    _pki: tempfile::TempDir,
    root: reqwest::Certificate,
    uri: String,
    state: Arc<Protocol>,
    stop: tokio::sync::watch::Sender<bool>,
    task: Option<owned_task::OwnedTask<anyhow::Result<()>>>,
}
impl VerifiedIdpTls {
    pub async fn start(matching_san: bool) -> Self {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let (ca, ca_key) = ca();
        let names = if matching_san {
            vec!["127.0.0.1".to_owned(), "localhost".to_owned()]
        } else {
            vec!["wrong-san.test".to_owned()]
        };
        let mut leaf = CertificateParams::new(names).expect("generated leaf SAN");
        leaf.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        leaf.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let leaf_key = KeyPair::generate().expect("generated fixture server key");
        let leaf = leaf
            .signed_by(&leaf_key, &ca, &ca_key)
            .expect("CA-signed fixture server certificate");
        let pki = tempfile::tempdir().expect("isolated generated PKI directory");
        std::fs::write(pki.path().join("ca.pem"), ca.pem()).expect("fixture public CA file");
        std::fs::write(pki.path().join("server.pem"), leaf.pem()).expect("fixture leaf file");
        std::fs::write(pki.path().join("server.key"), leaf_key.serialize_pem())
            .expect("generated fixture key file");
        let root = reqwest::Certificate::from_pem(
            &std::fs::read(pki.path().join("ca.pem")).expect("public fixture CA bytes"),
        )
        .expect("explicit instance root");
        let config = axum_server::tls_rustls::RustlsConfig::from_pem(
            leaf.pem().into_bytes(),
            leaf_key.serialize_pem().into_bytes(),
        )
        .await
        .expect("isolated TLS server config");
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("one isolated bound listener");
        let address = listener.local_addr().expect("actual random address");
        use base64::Engine;
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
            iam_domain::entity::oauth_transaction::hash_oauth_bytes(VERIFIER.as_bytes()),
        );
        let state = Arc::new(Protocol {
            requests: AtomicUsize::new(0),
            receipts: Mutex::new(Receipts::default()),
            redirect: Mutex::new(None),
            challenge,
        });
        let router = Router::new()
            .route(&format!("/connect{AUTHORIZE_PATH}"), post(protocol))
            .route(&format!("/connect{TOKEN_PATH}"), post(protocol))
            .route(&format!("/connect{PROFILE_PATH}"), post(protocol))
            .with_state(state.clone());
        let server = axum_server::Handle::new();
        let ready = server.clone();
        let (stop, mut stopping) = tokio::sync::watch::channel(false);
        let task = owned_task::spawn(async move {
            let serving = axum_server::from_tcp_rustls(listener, config)
                .handle(server.clone())
                .serve(router.into_make_service());
            tokio::pin!(serving);
            tokio::select! {
                result=&mut serving => { result.map_err(|_|anyhow::anyhow!("fixture TLS listener failed")) },
                _=stopping.changed() => {
                    server.graceful_shutdown(Some(Duration::from_secs(2)));
                    tokio::time::timeout(Duration::from_secs(4),&mut serving).await
                        .map_err(|_|anyhow::anyhow!("fixture TLS graceful join timed out"))?
                        .map_err(|_|anyhow::anyhow!("fixture TLS shutdown failed"))
                }
            }
        });
        let mut fixture = Self {
            _pki: pki,
            root,
            uri: format!("https://{address}/connect"),
            state,
            stop,
            task: Some(task),
        };
        let listening = tokio::time::timeout(Duration::from_secs(5), ready.listening()).await;
        if !matches!(listening,Ok(Some(actual)) if actual==address) {
            let _ = fixture.shutdown().await;
            panic!("owned TLS fixture did not reach listening state");
        }
        fixture
    }
    pub fn root(&self) -> reqwest::Certificate {
        self.root.clone()
    }
    pub fn uri(&self) -> &str {
        &self.uri
    }
    pub fn count(&self) -> usize {
        self.state.requests.load(Ordering::SeqCst)
    }
    pub fn receipts(&self) -> Receipts {
        self.state.receipts.lock().expect("receipt lock").clone()
    }
    pub fn challenge(&self) -> &str {
        &self.state.challenge
    }
    pub fn redirect_to(&self, other: &Self) {
        *self.state.redirect.lock().expect("redirect lock") =
            Some(format!("{}{AUTHORIZE_PATH}", other.uri()));
    }
    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        let _ = self.stop.send(true);
        if let Some(mut task) = self.task.take() {
            let result = tokio::time::timeout(Duration::from_secs(5), &mut task).await;
            if result.is_err() {
                task.abort();
                let _ = task.await;
                return Err(anyhow::anyhow!(
                    "fixture TLS join failed; parent reconciliation required"
                ));
            }
            result
                .expect("checked timeout")
                .map_err(|_| anyhow::anyhow!("fixture TLS task failed"))??;
        }
        Ok(())
    }
}
