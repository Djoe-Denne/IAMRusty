//! Apparatus P3 — T11b live HTTPS `/lazaret/session` with optional rustycog mTLS.
//!
//! T3/T10 stay in-process (`extensions_mut`). This file talks to `serve_router`.

use std::net::TcpListener;
use std::sync::Arc;
use std::time::{Duration, Instant};

use apparatus_contracts::{ApparatusError, BindingId};
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use lazaret_application::{empty_command_registry, EnrollCommand, IdentityService, InvokeService};
use lazaret_configuration::IdentityConfig;
use lazaret_domain::{
    AsyncKvStore, BindingGrantSnapshot, BindingGrantSnapshotPort, CapabilityConsent,
    CertificateAuthority, ConnectorRegistry, GrantFetchError, VerifiedClientCertificate,
    WorkloadIdentity, DEFAULT_CERT_TTL_HOURS, SESSION_AUDIENCE, SESSION_ISSUER,
};
use lazaret_http::{create_prefixed_router, create_router};
use lazaret_infra::{
    DedicatedSessionSigner, DeniedSecretResolver, InMemoryEnrollmentRegistry, NamedConnectorProxy,
    PlatformInternalCa,
};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose,
};
use readiness::ReadinessProbe;
use rustycog::command::GenericCommandService;
use rustycog::config::ServerConfig;
use rustycog::http::{serve_router, AppState, PeerClientCertificate, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};
use tower::ServiceExt;
use uuid::Uuid;

struct MemoryPort {
    snapshot: BindingGrantSnapshot,
}

#[async_trait]
impl BindingGrantSnapshotPort for MemoryPort {
    async fn fetch(
        &self,
        _project_id: Uuid,
        _component_id: Uuid,
        _principal: Option<Uuid>,
    ) -> Result<BindingGrantSnapshot, GrantFetchError> {
        Ok(self.snapshot.clone())
    }
}

struct NoopKv;

#[async_trait]
impl AsyncKvStore for NoopKv {
    async fn get(
        &self,
        _binding: &BindingId,
        _key: &str,
    ) -> Result<Option<Vec<u8>>, ApparatusError> {
        Ok(None)
    }

    async fn put(
        &self,
        _binding: &BindingId,
        _key: &str,
        _value: &[u8],
        _expected_cas: Option<i64>,
    ) -> Result<i64, ApparatusError> {
        Ok(1)
    }

    async fn delete(&self, _binding: &BindingId, _key: &str) -> Result<bool, ApparatusError> {
        Ok(false)
    }

    async fn purge(&self, _binding: &BindingId) -> Result<(), ApparatusError> {
        Ok(())
    }
}

struct LiveServer {
    _dir: tempfile::TempDir,
    handle: tokio::task::JoinHandle<()>,
    base: String,
    ca: Arc<PlatformInternalCa>,
}

fn install_crypto() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

fn ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn snapshot_for(identity: &WorkloadIdentity, project_id: Uuid) -> BindingGrantSnapshot {
    BindingGrantSnapshot {
        component_id: identity.binding,
        project_id,
        project_status: "active".to_owned(),
        component_status: "active".to_owned(),
        source: "managed".to_owned(),
        digest: Some(identity.release.clone()),
        desired_generation: identity.generation,
        observed_generation: identity.generation,
        grant_revision: identity.grant_revision,
        declared: vec!["storage.kv.read".to_owned()],
        consents: vec![CapabilityConsent {
            capability: "storage.kv.read".to_owned(),
            status: "consented".to_owned(),
            grant_revision: identity.grant_revision,
        }],
        principal: None,
    }
}

fn sample_identity() -> WorkloadIdentity {
    WorkloadIdentity::try_new(Uuid::new_v4(), Uuid::new_v4(), "release-1".to_owned(), 1, 0)
        .expect("identity")
}

fn workload_csr() -> (String, String) {
    let key = KeyPair::generate().expect("workload key stays local");
    let params = CertificateParams::new(vec!["workload.test".to_owned()]).expect("csr params");
    let csr = params.serialize_request(&key).expect("csr");
    (csr.pem().expect("csr pem"), key.serialize_pem())
}

fn app_state() -> AppState {
    let command_registry = empty_command_registry();
    let command_service = Arc::new(GenericCommandService::new(Arc::new(command_registry)));
    let extractor = UserIdExtractor::from_resolved_secret("test-hs256-secret").expect("jwt");
    let checker: Arc<dyn PermissionChecker> = Arc::new(InMemoryPermissionChecker::new());
    AppState::new(command_service, extractor, checker)
}

fn invoke_stub(identity: Arc<IdentityService>) -> Arc<InvokeService> {
    let snapshots: Arc<dyn BindingGrantSnapshotPort> = Arc::new(MemoryPort {
        snapshot: snapshot_for(&sample_identity(), Uuid::new_v4()),
    });
    let grants = Arc::new(lazaret_application::GrantService::new(snapshots));
    let connectors =
        Arc::new(NamedConnectorProxy::new(ConnectorRegistry::default()).expect("connectors"));
    Arc::new(InvokeService::new(
        identity,
        grants,
        Arc::new(NoopKv),
        Arc::new(DeniedSecretResolver),
        connectors,
    ))
}

fn identity_service(
    ca: Arc<PlatformInternalCa>,
    identity: &WorkloadIdentity,
    project_id: Uuid,
) -> Arc<IdentityService> {
    let config = IdentityConfig::default();
    let snapshots: Arc<dyn BindingGrantSnapshotPort> = Arc::new(MemoryPort {
        snapshot: snapshot_for(identity, project_id),
    });
    Arc::new(IdentityService::new(
        ca,
        Arc::new(DedicatedSessionSigner::from_config(&config).expect("signer")),
        Arc::new(InMemoryEnrollmentRegistry::new()),
        snapshots,
        config.session_ttl_minutes,
        config.cert_ttl_hours,
    ))
}

fn self_signed_server() -> (Certificate, KeyPair) {
    let mut params = CertificateParams::new(vec!["127.0.0.1".into(), "localhost".into()]).unwrap();
    params
        .distinguished_name
        .push(DnType::CommonName, "lazaret-t11b-server");
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params
        .extended_key_usages
        .push(ExtendedKeyUsagePurpose::ServerAuth);
    let key = KeyPair::generate().unwrap();
    let cert = params.self_signed(&key).unwrap();
    (cert, key)
}

fn foreign_client_identity_pem() -> Vec<u8> {
    let mut ca_params = CertificateParams::new(Vec::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "t11b-foreign-ca");
    ca_params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    ca_params.key_usages.push(KeyUsagePurpose::KeyCertSign);
    ca_params.key_usages.push(KeyUsagePurpose::CrlSign);
    let ca_key = KeyPair::generate().unwrap();
    let ca_cert = ca_params.self_signed(&ca_key).unwrap();

    let mut params = CertificateParams::new(vec!["foreign-client.test".into()]).unwrap();
    params
        .distinguished_name
        .push(DnType::CommonName, "foreign-client");
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params
        .extended_key_usages
        .push(ExtendedKeyUsagePurpose::ClientAuth);
    let key = KeyPair::generate().unwrap();
    let cert = params.signed_by(&key, &ca_cert, &ca_key).unwrap();
    format!("{}{}", cert.pem(), key.serialize_pem()).into_bytes()
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

fn enroll_body(identity: &WorkloadIdentity, project_id: Uuid, csr_pem: &str) -> serde_json::Value {
    serde_json::json!({
        "csr_pem": csr_pem,
        "instance": identity.instance,
        "binding": identity.binding,
        "release": identity.release,
        "generation": identity.generation,
        "grant_revision": identity.grant_revision,
        "project_id": project_id,
    })
}

fn jwt_payload(token: &str) -> serde_json::Value {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    let payload = token.split('.').nth(1).expect("payload");
    let decoded = URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .expect("b64");
    serde_json::from_slice(&decoded).expect("json")
}

async fn json_from_response(response: axum::http::Response<Body>) -> serde_json::Value {
    use futures::StreamExt;
    let mut bytes = Vec::new();
    let mut stream = response.into_body().into_data_stream();
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk.expect("chunk"));
    }
    serde_json::from_slice(&bytes).expect("json")
}

async fn wait_until_ready(
    handle: &tokio::task::JoinHandle<()>,
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

async fn spawn_live(identity: &WorkloadIdentity, project_id: Uuid) -> LiveServer {
    install_crypto();
    let dir = tempfile::tempdir().expect("tempdir");
    let ca = Arc::new(PlatformInternalCa::new().expect("platform ca"));
    let ca_path = dir.path().join("client-ca.crt");
    std::fs::write(&ca_path, ca.cert_pem()).expect("write ca pem");

    let (server_cert, server_key) = self_signed_server();
    let cert_path = dir.path().join("server.crt");
    let key_path = dir.path().join("server.key");
    std::fs::write(&cert_path, server_cert.pem()).expect("write server cert");
    std::fs::write(&key_path, server_key.serialize_pem()).expect("write server key");

    let identity_svc = identity_service(ca.clone(), identity, project_id);
    let invoke = invoke_stub(identity_svc.clone());
    let router = create_prefixed_router(
        app_state(),
        Arc::new(ReadinessProbe::new("lazaret")),
        identity_svc,
        invoke,
    );

    let port = ephemeral_port();
    let config = ServerConfig {
        host: "127.0.0.1".into(),
        port: 0,
        tls_enabled: true,
        tls_cert_path: cert_path.to_string_lossy().into_owned(),
        tls_key_path: key_path.to_string_lossy().into_owned(),
        tls_client_ca_path: ca_path.to_string_lossy().into_owned(),
        tls_port: port,
    };
    let handle = tokio::spawn(async move {
        serve_router(router, config).await.expect("serve_router");
    });
    let base = format!("https://127.0.0.1:{port}");
    let probe = https_client(None);
    wait_until_ready(&handle, &probe, &format!("{base}/lazaret/health")).await;
    LiveServer {
        _dir: dir,
        handle,
        base,
        ca,
    }
}

#[tokio::test]
async fn t11b_enrolled_client_cert_session_is_200() {
    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let server = spawn_live(&expected, project_id).await;
    let (csr_pem, key_pem) = workload_csr();
    let enroll = https_client(None)
        .post(format!("{}/lazaret/enroll", server.base))
        .json(&enroll_body(&expected, project_id, &csr_pem))
        .send()
        .await
        .expect("enroll");
    assert_eq!(
        enroll.status(),
        reqwest::StatusCode::OK,
        "{}",
        enroll.text().await.unwrap_or_default()
    );
    let body: serde_json::Value = enroll.json().await.expect("enroll json");
    let cert_pem = body["certificate_pem"].as_str().expect("certificate_pem");
    let identity_pem = format!("{cert_pem}{key_pem}");

    let session = https_client(Some(identity_pem.as_bytes()))
        .post(format!("{}/lazaret/session", server.base))
        .send()
        .await
        .expect("session");
    assert_eq!(
        session.status(),
        reqwest::StatusCode::OK,
        "{}",
        session.text().await.unwrap_or_default()
    );
    let session_body: serde_json::Value = session.json().await.expect("session json");
    let token = session_body["session_token"]
        .as_str()
        .expect("session_token");
    let claims = jwt_payload(token);
    assert_eq!(claims["iss"], SESSION_ISSUER);
    assert_eq!(claims["aud"], SESSION_AUDIENCE);
    assert_eq!(claims["binding"], expected.binding.to_string());
    assert_eq!(claims["instance"], expected.instance.to_string());

    server.handle.abort();
}

#[tokio::test]
async fn t11b_session_without_client_cert_is_401() {
    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let server = spawn_live(&expected, project_id).await;
    let session = https_client(None)
        .post(format!("{}/lazaret/session", server.base))
        .send()
        .await
        .expect("handshake without client cert");
    assert_eq!(session.status(), reqwest::StatusCode::UNAUTHORIZED);
    server.handle.abort();
}

#[tokio::test]
async fn t11b_foreign_client_cert_fails_handshake() {
    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let server = spawn_live(&expected, project_id).await;
    let client = https_client(Some(&foreign_client_identity_pem()));
    let url = format!("{}/lazaret/session", server.base);
    match client.post(&url).send().await {
        Err(_) => {}
        Ok(response) => assert!(
            !response.status().is_success(),
            "foreign client cert must not get 2xx, got {}",
            response.status()
        ),
    }
    server.handle.abort();
}

#[tokio::test]
async fn t11b_enroll_without_client_cert_is_200() {
    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let server = spawn_live(&expected, project_id).await;
    let (csr_pem, _) = workload_csr();
    let enroll = https_client(None)
        .post(format!("{}/lazaret/enroll", server.base))
        .json(&enroll_body(&expected, project_id, &csr_pem))
        .send()
        .await
        .expect("enroll without client cert");
    assert_eq!(
        enroll.status(),
        reqwest::StatusCode::OK,
        "{}",
        enroll.text().await.unwrap_or_default()
    );
    server.handle.abort();
}

#[tokio::test]
async fn map_peer_preserves_pre_injected_verified_cert() {
    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let ca = Arc::new(PlatformInternalCa::new().expect("platform ca"));
    let identity = identity_service(ca, &expected, project_id);
    let (csr_pem, _) = workload_csr();
    let issued = identity
        .enroll(EnrollCommand {
            csr_pem,
            identity: expected.clone(),
            project_id,
        })
        .await
        .expect("enroll");
    let invoke = invoke_stub(identity.clone());
    let router = create_router(app_state(), identity, invoke);
    let mut request = Request::builder()
        .method("POST")
        .uri("/session")
        .body(Body::empty())
        .expect("request");
    request
        .extensions_mut()
        .insert(VerifiedClientCertificate::from_der(issued.der));
    request.extensions_mut().insert(PeerClientCertificate {
        der: b"unused-peer-bytes".to_vec(),
    });
    let response = router.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);
    let session_body = json_from_response(response).await;
    let token = session_body["session_token"]
        .as_str()
        .expect("session_token");
    let claims = jwt_payload(token);
    assert_eq!(claims["binding"], expected.binding.to_string());
}

#[tokio::test]
async fn t11b_trusted_unenrolled_client_cert_is_401() {
    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let server = spawn_live(&expected, project_id).await;
    let (csr_pem, key_pem) = workload_csr();
    let issued = server
        .ca
        .sign_csr(&csr_pem, DEFAULT_CERT_TTL_HOURS, expected.binding)
        .expect("sign csr with live platform ca");
    let identity_pem = format!("{}{key_pem}", issued.pem);
    let session = https_client(Some(identity_pem.as_bytes()))
        .post(format!("{}/lazaret/session", server.base))
        .send()
        .await
        .expect("session with trusted unenrolled cert");
    assert_eq!(session.status(), reqwest::StatusCode::UNAUTHORIZED);
    server.handle.abort();
}
