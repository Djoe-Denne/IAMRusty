//! Apparatus P3 — T13 persisted platform CA + generate-if-absent server leaf (HTTPS).

use std::net::TcpListener;
use std::sync::Arc;
use std::time::{Duration, Instant};

use apparatus_contracts::{ApparatusError, BindingId};
use async_trait::async_trait;
use lazaret_application::{empty_command_registry, EnrollCommand, IdentityService, InvokeService};
use lazaret_configuration::IdentityConfig;
use lazaret_domain::{
    AsyncKvStore, BindingGrantSnapshot, BindingGrantSnapshotPort, CapabilityConsent,
    ConnectorRegistry, GrantFetchError, WorkloadIdentity, SESSION_AUDIENCE, SESSION_ISSUER,
};
use lazaret_http::create_prefixed_router;
use lazaret_infra::{
    DedicatedSessionSigner, DeniedSecretResolver, InMemoryEnrollmentRegistry, NamedConnectorProxy,
    PlatformInternalCa,
};
use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose,
};
use readiness::ReadinessProbe;
use rustycog::command::GenericCommandService;
use rustycog::config::ServerConfig;
use rustycog::http::{serve_router, AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};
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
    enrollments: Arc<InMemoryEnrollmentRegistry>,
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
        enrollments,
        snapshots,
        config.session_ttl_minutes,
        config.cert_ttl_hours,
    ))
}

fn foreign_client_identity_pem() -> Vec<u8> {
    let mut ca_params = CertificateParams::new(Vec::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "t13-foreign-ca");
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

fn jwt_payload(token: &str) -> serde_json::Value {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    let payload = token.split('.').nth(1).expect("payload");
    let decoded = URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .expect("b64");
    serde_json::from_slice(&decoded).expect("json")
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

async fn spawn_https(
    ca: Arc<PlatformInternalCa>,
    identity_svc: Arc<IdentityService>,
) -> LiveServer {
    install_crypto();
    let dir = tempfile::tempdir().expect("tempdir");
    let ca_path = dir.path().join("ca.crt");
    ca.write_trust_anchor(&ca_path).expect("write trust anchor");
    let cert_path = dir.path().join("server.crt");
    let key_path = dir.path().join("server.key");
    ca.ensure_server_leaf(&cert_path, &key_path)
        .expect("ca-signed server leaf");
    let server_pem = std::fs::read_to_string(&cert_path).expect("read server.crt");
    assert!(
        server_pem.contains("BEGIN CERTIFICATE"),
        "server leaf must be a certificate"
    );
    assert!(
        !server_pem.contains("PRIVATE KEY"),
        "server leaf must not include a private key"
    );

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
    }
}

#[tokio::test]
async fn t13_reload_ca_same_cert_pem_then_session_200() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ca_cert = dir.path().join("ca.crt");
    let ca_key = dir.path().join("ca.key");
    let ca1 = Arc::new(
        PlatformInternalCa::load_or_create(&ca_cert, &ca_key).expect("create platform ca"),
    );
    let pem1 = ca1.cert_pem();
    assert!(pem1.contains("BEGIN CERTIFICATE"));
    assert!(!pem1.contains("PRIVATE KEY"));
    let ca2 = Arc::new(
        PlatformInternalCa::load_or_create(&ca_cert, &ca_key).expect("reload platform ca"),
    );
    assert_eq!(
        pem1,
        ca2.cert_pem(),
        "reloaded CA must serve the same trust-anchor PEM"
    );
    assert_eq!(
        pem1,
        std::fs::read_to_string(&ca_cert).expect("re-read ca.crt"),
        "reloaded CA PEM must match ca.crt on disk"
    );

    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let enrollments = Arc::new(InMemoryEnrollmentRegistry::new());
    let svc1 = identity_service(ca1, enrollments.clone(), &expected, project_id);
    let (csr_pem, key_pem) = workload_csr();
    let issued = svc1
        .enroll(EnrollCommand {
            csr_pem,
            identity: expected.clone(),
            project_id,
        })
        .await
        .expect("enroll against CA1");

    let svc2 = identity_service(ca2.clone(), enrollments, &expected, project_id);
    let server = spawn_https(ca2, svc2).await;
    let identity_pem = format!("{}{key_pem}", issued.pem);
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
    server.handle.abort();
}

#[tokio::test]
async fn t13_foreign_client_cert_fails_handshake() {
    let expected = sample_identity();
    let project_id = Uuid::new_v4();
    let dir = tempfile::tempdir().expect("tempdir");
    let ca = Arc::new(
        PlatformInternalCa::load_or_create(&dir.path().join("ca.crt"), &dir.path().join("ca.key"))
            .expect("platform ca"),
    );
    let identity = identity_service(
        ca.clone(),
        Arc::new(InMemoryEnrollmentRegistry::new()),
        &expected,
        project_id,
    );
    let server = spawn_https(ca, identity).await;
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

#[test]
fn t13_generate_if_absent_writes_ca_crt_without_private_key() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cert_path = dir.path().join("ca.crt");
    let key_path = dir.path().join("ca.key");
    let ca = PlatformInternalCa::load_or_create(&cert_path, &key_path).expect("generate");
    let on_disk = std::fs::read_to_string(&cert_path).expect("read ca.crt");
    assert!(on_disk.contains("BEGIN CERTIFICATE"));
    assert!(!on_disk.contains("PRIVATE KEY"));
    assert_eq!(on_disk, ca.cert_pem());
}
