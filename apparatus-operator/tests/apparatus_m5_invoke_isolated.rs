//! M5 — invoke E2E d'un plugin isolé via Lazaret `POST /invoke` (ferme T-3).
//!
//! Hop Lazaret HTTP → plugin `INVOKE_PATH` (pas de kube dans Lazaret).
//!
//! Pin M1 + VALID M2 + install M3 + digest stub M4 + session P3 + hop HTTP
//! jusqu'au Pod `apparatus-plugins`. Locator injecté (pas de kube dans Lazaret).

use std::fs;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use apparatus_contracts::{new_operation_id, BindingId, InvokeRequest};
use apparatus_operator::admission::{
    would_schedule, AdmissionStore, AdmitInput, PersistentAdmissionStore,
};
use apparatus_operator::admit::{push_and_sign_envelope, AdmitTarget, Envelope, RegistryAuth};
use apparatus_operator::controller::{
    pod_name_for, IsolationLabels, ReconcileOutcome, WorkloadReconciler, BINDING_LABEL,
    PLUGINS_NAMESPACE, PLUGIN_INVOKE_PORT, PROJECT_LABEL, SYSTEM_NAMESPACE,
};
use apparatus_operator::desired_state::{reconcile_ready, HttpComponentsClient};
use apparatus_operator::{
    digest_manifest, evaluate_conformance, parse_manifest, ReleaseDigest, POLICY_ID,
};
use async_trait::async_trait;
use lazaret_application::{
    empty_command_registry, EnrollCommand, GrantService, IdentityService, InvokeService,
    PluginEndpointLocator, StaticPluginLocator,
};
use lazaret_configuration::IdentityConfig;
use lazaret_domain::{
    AsyncKvStore, BindingGrantSnapshot, BindingGrantSnapshotPort, CapabilityConsent,
    ConnectorRegistry, GrantFetchError, WorkloadIdentity,
};
use lazaret_http::create_router;
use lazaret_infra::{
    DedicatedSessionSigner, DeniedSecretResolver, InMemoryEnrollmentRegistry, NamedConnectorProxy,
    PlatformInternalCa,
};
use rcgen::{CertificateParams, KeyPair};
use rustycog::command::GenericCommandService;
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};
use serial_test::serial;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

#[allow(dead_code)]
#[path = "fixtures/with_kind.rs"]
mod fixtures;

const REFERENCE_KV_ID: &str = "io.aiforall.reference-kv";
const CRI_IMAGE_NAME: &str = "apparatus-reference-kv";
const CRI_BUILD_TAG: &str = "apparatus-reference-kv:m5";
const CANARY_LABEL: &str = "app=apparatus-system-canary";
const CANARY_URL: &str = "http://apparatus-system-canary.apparatus-system.svc.cluster.local:8080/";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn reference_kv_dir() -> PathBuf {
    repo_root().join("apparatus-reference-kv")
}

fn descriptor_from_disk() -> ReleaseDigest {
    let raw = fs::read_to_string(reference_kv_dir().join("apparatus.toml"))
        .unwrap_or_else(|err| panic!("apparatus.toml: {err}"));
    let validated = parse_manifest(&raw).unwrap_or_else(|err| panic!("parse_manifest: {err}"));
    assert_eq!(validated.manifest.apparatus.id.as_str(), REFERENCE_KV_ID);
    digest_manifest(&validated.manifest).expect("digest descripteur 0002")
}

fn passing_input() -> (AdmitInput, ReleaseDigest) {
    let raw = fs::read_to_string(reference_kv_dir().join("apparatus.toml"))
        .unwrap_or_else(|err| panic!("apparatus.toml: {err}"));
    let descriptor = descriptor_from_disk();
    let report = evaluate_conformance(&raw);
    assert!(report.passed, "le manifeste de référence M1 doit passer T5");
    let input = AdmitInput {
        descriptor_digest: descriptor.clone(),
        observed_descriptor: descriptor.clone(),
        policy_id: POLICY_ID.to_owned(),
        report_digest: report.report_digest,
        conformance_passed: report.passed,
        signature_verified: true,
        claimed_verified: false,
    };
    (input, descriptor)
}

struct TempStorePath {
    path: PathBuf,
}

impl TempStorePath {
    fn new(label: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("horloge")
            .as_nanos();
        Self {
            path: std::env::temp_dir().join(format!(
                "apparatus-m5-{label}-{}-{nanos}.json",
                std::process::id()
            )),
        }
    }
}

impl Drop for TempStorePath {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let mut tmp = self.path.clone().into_os_string();
        tmp.push(".tmp");
        let _ = fs::remove_file(PathBuf::from(tmp));
    }
}

fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap_or_else(|err| panic!("mkdir {}: {err}", dst.display()));
    for entry in fs::read_dir(src).unwrap_or_else(|err| panic!("read {}: {err}", src.display())) {
        let entry = entry.unwrap_or_else(|err| panic!("entry: {err}"));
        let name = entry.file_name();
        if name == "target" || name == ".git" || name == "vendor" || name == "tests" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::copy(&from, &to).unwrap_or_else(|err| panic!("copy {}: {err}", from.display()));
        }
    }
}

fn docker_build_reference_kv_http() {
    fixtures::assert_docker_running().unwrap_or_else(|err| panic!("{err}"));
    let inspect = Command::new("docker")
        .args(["image", "inspect", CRI_BUILD_TAG])
        .output();
    if inspect.is_ok_and(|out| out.status.success()) {
        return;
    }
    let staging = std::env::temp_dir().join(format!("apparatus-m5-cri-{}", std::process::id()));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).expect("staging");
    fs::write(
        staging.join("Cargo.toml"),
        r#"
[workspace]
resolver = "2"
members = ["apparatus-contracts", "apparatus-reference-kv"]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Djoé Denne <djoe.denne@gmail.com>"]
license = "MIT OR Apache-2.0"

[workspace.dependencies]
serde = { version = "1.0.197", features = ["derive"] }
serde_json = "1.0.120"
toml = "0.8.23"
thiserror = "2.0.11"
uuid = { version = "1.4.1", features = ["v4", "serde"] }
sha2 = "0.10.8"
semver = "1.0.23"
axum = { version = "0.8.4", features = ["macros", "json"] }
tokio = { version = "1.44", features = ["rt-multi-thread", "macros", "net", "signal"] }

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
cargo = { level = "warn", priority = -1 }
cargo_common_metadata = "allow"
multiple_crate_versions = "allow"
module_name_repetitions = "allow"
must_use_candidate = "allow"
"#,
    )
    .expect("workspace Cargo.toml");
    copy_tree(
        &repo_root().join("apparatus-contracts"),
        &staging.join("apparatus-contracts"),
    );
    copy_tree(&reference_kv_dir(), &staging.join("apparatus-reference-kv"));
    fs::copy(
        reference_kv_dir().join("Dockerfile.http"),
        staging.join("Dockerfile"),
    )
    .expect("Dockerfile.http");
    let build = Command::new("docker")
        .args(["build", "-t", CRI_BUILD_TAG])
        .arg(&staging)
        .output()
        .unwrap_or_else(|err| panic!("Docker daemon must be running (docker build: {err})"));
    assert!(
        build.status.success(),
        "docker build {CRI_BUILD_TAG}: {:?}\n{}",
        build.status.code(),
        String::from_utf8_lossy(&build.stderr)
    );
}

struct ManifestoStub {
    base_url: String,
}

async fn spawn_manifesto_stub(body: serde_json::Value) -> ManifestoStub {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|err| panic!("bind stub manifesto: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| panic!("local_addr stub: {err}"));
    let payload = serde_json::to_string(&body).expect("json stub");
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let mut buf = Vec::new();
            let mut tmp = [0u8; 512];
            loop {
                let Ok(n) = sock.read(&mut tmp).await else {
                    break;
                };
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() > 16_384 {
                    break;
                }
            }
            let req = String::from_utf8_lossy(&buf);
            let first = req.lines().next().unwrap_or("");
            let ok = first.contains("/manifesto/api/projects/")
                && first.contains("/components")
                && first.starts_with("GET ");
            let (status, body) = if ok {
                ("200 OK", payload.as_str())
            } else {
                ("404 Not Found", "{\"error\":\"not found\"}")
            };
            let resp = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = sock.write_all(resp.as_bytes()).await;
        }
    });
    ManifestoStub {
        base_url: format!("http://{addr}/manifesto"),
    }
}

fn active_component_json(
    project_id: Uuid,
    component_id: Uuid,
    digest: &ReleaseDigest,
) -> serde_json::Value {
    serde_json::json!({
        "_project_id": project_id.to_string(),
        "data": [{
            "id": component_id,
            "component_type": "apparatus",
            "status": "active",
            "added_at": "2026-09-22T00:00:00Z",
            "configured_at": serde_json::Value::Null,
            "activated_at": "2026-09-22T00:00:00Z",
            "disabled_at": serde_json::Value::Null,
            "digest": digest.as_str(),
        }]
    })
}

struct PortForward {
    child: Child,
}

impl Drop for PortForward {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn wait_tcp(addr: &str, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        if TcpStream::connect(addr).is_ok() {
            return;
        }
        if Instant::now() > deadline {
            panic!("port-forward {addr} injoignable");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

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
    ) -> Result<Option<Vec<u8>>, apparatus_contracts::ApparatusError> {
        Ok(None)
    }

    async fn put(
        &self,
        _binding: &BindingId,
        _key: &str,
        _value: &[u8],
        _expected_cas: Option<i64>,
    ) -> Result<i64, apparatus_contracts::ApparatusError> {
        Ok(1)
    }

    async fn delete(
        &self,
        _binding: &BindingId,
        _key: &str,
    ) -> Result<bool, apparatus_contracts::ApparatusError> {
        Ok(false)
    }

    async fn purge(&self, _binding: &BindingId) -> Result<(), apparatus_contracts::ApparatusError> {
        Ok(())
    }
}

struct CountingLocator {
    inner: StaticPluginLocator,
    hops: Arc<AtomicUsize>,
}

#[async_trait]
impl PluginEndpointLocator for CountingLocator {
    async fn locate(&self, binding_id: &str, instance_id: &str) -> Option<String> {
        self.hops.fetch_add(1, Ordering::SeqCst);
        self.inner.locate(binding_id, instance_id).await
    }
}

fn snapshot(
    project_id: Uuid,
    binding: Uuid,
    grant_revision: i64,
    caps: &[&str],
    with_consent: bool,
) -> BindingGrantSnapshot {
    BindingGrantSnapshot {
        component_id: binding,
        project_id,
        project_status: "active".to_owned(),
        component_status: "active".to_owned(),
        source: "managed".to_owned(),
        digest: Some("release-1".to_owned()),
        desired_generation: 1,
        observed_generation: 1,
        grant_revision,
        declared: caps.iter().map(|name| (*name).to_owned()).collect(),
        consents: if with_consent {
            caps.iter()
                .map(|name| CapabilityConsent {
                    capability: (*name).to_owned(),
                    status: "consented".to_owned(),
                    grant_revision,
                })
                .collect()
        } else {
            Vec::new()
        },
        principal: None,
    }
}

fn workload_csr() -> String {
    let key = KeyPair::generate().expect("key");
    let params = CertificateParams::new(vec!["workload.test".to_owned()]).expect("params");
    params
        .serialize_request(&key)
        .expect("csr")
        .pem()
        .expect("pem")
}

fn app_state() -> AppState {
    let command_registry = empty_command_registry();
    let command_service = Arc::new(GenericCommandService::new(Arc::new(command_registry)));
    let extractor = UserIdExtractor::from_resolved_secret("test-hs256-secret").expect("jwt");
    let checker: Arc<dyn PermissionChecker> = Arc::new(InMemoryPermissionChecker::new());
    AppState::new(command_service, extractor, checker)
}

fn invoke_body(binding: Uuid, operation: &str, params: serde_json::Value) -> InvokeRequest {
    InvokeRequest {
        binding_id: binding.to_string().parse::<BindingId>().expect("id"),
        operation_id: new_operation_id(),
        operation: operation.to_owned(),
        params,
    }
}

fn wget_reached_target(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("http/1.") || lower.contains(" 200 ") || lower.contains(" 404 ")
}

fn wget_blocked_signal(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("download timed out")
        || lower.contains("wget: timeout")
        || lower.contains("connection refused")
        || lower.contains("can't connect")
        || lower.contains("network is unreachable")
        || lower.contains("no route to host")
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn m5_invoke_reaches_isolated_plugin_pod() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    cluster
        .build_and_load_platform_plugin()
        .unwrap_or_else(|err| panic!("{err}"));
    docker_build_reference_kv_http();
    let cri_image = cluster
        .load_and_pin_image(CRI_BUILD_TAG, CRI_IMAGE_NAME)
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(cri_image.contains("@sha256:"), "pin CRI M5: {cri_image}");

    let stack = fixtures::SignRegistryStack::start()
        .await
        .unwrap_or_else(|err| panic!("SignRegistryStack: {err}"));
    let (mut input, descriptor) = passing_input();
    let target = AdmitTarget {
        registry_host: format!("127.0.0.1:{}", stack.zot.port),
        registry_network: fixtures::zot::TestZot::network_registry(),
        docker_network: fixtures::SignRegistryStack::network_name().to_owned(),
        vault_addr_host: stack.transit.host_base_url(),
        vault_addr_network: fixtures::openbao_transit::TestOpenBaoTransit::network_base_url(),
        vault_token: fixtures::openbao_transit::TestOpenBaoTransit::token().to_owned(),
        transit_key: fixtures::openbao_transit::TestOpenBaoTransit::key_name().to_owned(),
        repository: "apparatus/envelope".to_owned(),
        tag: "m5".to_owned(),
        auth: RegistryAuth {
            username: fixtures::zot::SIGNER_USER.to_owned(),
            password: fixtures::zot::SIGNER_PASSWORD.to_owned(),
        },
    };
    let signed = push_and_sign_envelope(
        &target,
        &Envelope {
            descriptor_digest: descriptor.clone(),
            cri_image: cri_image.clone(),
        },
    )
    .await
    .unwrap_or_else(|err| panic!("push+sign: {err}"));
    input.signature_verified = signed.signature_verified;

    let project_id = Uuid::new_v4();
    let component_id = Uuid::new_v4();
    let stub =
        spawn_manifesto_stub(active_component_json(project_id, component_id, &descriptor)).await;
    let source = HttpComponentsClient::new(stub.base_url, project_id, None::<String>)
        .unwrap_or_else(|err| panic!("{err}"));

    let tmp = TempStorePath::new("valid-m1");
    let mut store = PersistentAdmissionStore::open(&tmp.path).expect("open JSON");
    store.admit(input).expect("admit M1 JSON VALID");
    store
        .bind_cri(&descriptor, &cri_image)
        .expect("bind_cri pin M1 JSON");
    store
        .bind_envelope(&descriptor, &signed.reference)
        .expect("bind_envelope ref T6");
    assert!(would_schedule(&store, &descriptor), "ligne JSON VALID");
    drop(store);

    let reconciler = WorkloadReconciler::connect(&cluster.kubeconfig, &tmp.path)
        .await
        .unwrap_or_else(|err| panic!("{err}"))
        .with_schedule_admit_target(target);
    let pod_name = pod_name_for(&descriptor);
    reconciler
        .delete_plugin_pod(&pod_name)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    reconciler
        .delete_admission_record(&descriptor)
        .await
        .unwrap_or_else(|err| panic!("{err}"));

    let store = PersistentAdmissionStore::open(&tmp.path).expect("reopen");
    let isolation = IsolationLabels::try_new(project_id.to_string(), component_id.to_string())
        .expect("labels isolation");
    let outcomes = reconcile_ready(&source, &reconciler, &store)
        .await
        .unwrap_or_else(|err| panic!("bridge: {err}"));
    assert_eq!(outcomes.len(), 1, "un binding ready");
    match &outcomes[0] {
        ReconcileOutcome::Scheduled {
            pod_name: scheduled,
        } => assert_eq!(*scheduled, pod_name),
        ReconcileOutcome::Refused(refuse) => panic!("schedule attendu, refus={refuse}"),
    }
    reconciler
        .wait_pod_running(&pod_name, Duration::from_secs(180))
        .await
        .unwrap_or_else(|err| panic!("{err}"));

    let labels = cluster
        .kubectl(&[
            "get",
            "pod",
            &pod_name,
            "-n",
            PLUGINS_NAMESPACE,
            "--show-labels",
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    let labels_txt = String::from_utf8_lossy(&labels.stdout);
    assert!(
        labels_txt.contains(&format!("{PROJECT_LABEL}={}", isolation.project)),
        "label projet: {labels_txt}"
    );
    assert!(
        labels_txt.contains(&format!("{BINDING_LABEL}={}", isolation.binding)),
        "label binding: {labels_txt}"
    );

    let local_port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind éphémère");
        listener.local_addr().expect("local_addr").port()
    };
    let pf = PortForward {
        child: cluster
            .spawn_kubectl(&[
                "port-forward",
                "-n",
                PLUGINS_NAMESPACE,
                &pod_name,
                &format!("{local_port}:{PLUGIN_INVOKE_PORT}"),
            ])
            .unwrap_or_else(|err| panic!("{err}")),
    };
    wait_tcp(&format!("127.0.0.1:{local_port}"), Duration::from_secs(30));
    let plugin_url = format!("http://127.0.0.1:{local_port}");

    let hops = Arc::new(AtomicUsize::new(0));
    let locator: Arc<dyn PluginEndpointLocator> = Arc::new(CountingLocator {
        inner: StaticPluginLocator::new(plugin_url),
        hops: hops.clone(),
    });

    let identity_body =
        WorkloadIdentity::try_new(Uuid::new_v4(), component_id, "release-1".to_owned(), 1, 1)
            .expect("identity");
    let enroll_port: Arc<dyn BindingGrantSnapshotPort> = Arc::new(MemoryPort {
        snapshot: snapshot(
            project_id,
            component_id,
            1,
            &["storage.kv.read", "storage.kv.write"],
            true,
        ),
    });
    let cfg = IdentityConfig::default();
    let identity = Arc::new(IdentityService::new(
        Arc::new(PlatformInternalCa::new().expect("ca")),
        Arc::new(DedicatedSessionSigner::from_config(&cfg).expect("signer")),
        Arc::new(InMemoryEnrollmentRegistry::new()),
        enroll_port,
        cfg.session_ttl_minutes,
        cfg.cert_ttl_hours,
    ));
    let issued = identity
        .enroll(EnrollCommand {
            csr_pem: workload_csr(),
            identity: identity_body,
            project_id,
        })
        .await
        .expect("enroll");
    let cert = lazaret_domain::VerifiedClientCertificate::from_pem(&issued.pem).expect("cert");
    let token = identity.issue_session(&cert).await.expect("session");

    let connectors =
        Arc::new(NamedConnectorProxy::new(ConnectorRegistry::default()).expect("connectors"));
    let deny_grants = Arc::new(GrantService::new(Arc::new(MemoryPort {
        snapshot: snapshot(
            project_id,
            component_id,
            1,
            &["storage.kv.read", "storage.kv.write"],
            false,
        ),
    })));
    let allow_grants = Arc::new(GrantService::new(Arc::new(MemoryPort {
        snapshot: snapshot(
            project_id,
            component_id,
            1,
            &["storage.kv.read", "storage.kv.write"],
            true,
        ),
    })));
    let deny_invoke = Arc::new(InvokeService::new_with_locator(
        identity.clone(),
        deny_grants,
        Arc::new(NoopKv),
        Arc::new(DeniedSecretResolver),
        connectors.clone(),
        locator.clone(),
    ));
    let allow_invoke = Arc::new(InvokeService::new_with_locator(
        identity.clone(),
        allow_grants,
        Arc::new(NoopKv),
        Arc::new(DeniedSecretResolver),
        connectors,
        locator,
    ));
    let deny_server =
        axum_test::TestServer::new(create_router(app_state(), identity.clone(), deny_invoke))
            .expect("deny server");
    let denied = deny_server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            component_id,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(denied.status_code(), 403, "{}", denied.text());
    assert_eq!(
        hops.load(Ordering::SeqCst),
        0,
        "sans consentement : pas de hop"
    );

    let allow_server =
        axum_test::TestServer::new(create_router(app_state(), identity, allow_invoke))
            .expect("allow server");
    let ok = allow_server
        .post(&format!("/invoke?project_id={project_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&invoke_body(
            component_id,
            "kv.get",
            serde_json::json!({"key": "k"}),
        ))
        .await;
    assert_eq!(ok.status_code(), 200, "{}", ok.text());
    assert!(
        hops.load(Ordering::SeqCst) >= 1,
        "grant allow doit localiser le plugin"
    );
    let body: serde_json::Value = ok.json();
    assert_eq!(
        body["result"]["hostname"].as_str(),
        Some(pod_name.as_str()),
        "preuve Pod: hostname={body}"
    );

    let wget = cluster
        .kubectl(&[
            "--request-timeout=20s",
            "exec",
            "-n",
            PLUGINS_NAMESPACE,
            &pod_name,
            "--",
            "wget",
            "-S",
            "-T",
            "5",
            "-O",
            "-",
            CANARY_URL,
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    let wget_txt = format!(
        "{}{}",
        String::from_utf8_lossy(&wget.stdout),
        String::from_utf8_lossy(&wget.stderr)
    );
    assert!(
        !wget_txt.to_ascii_lowercase().contains("wget: not found"),
        "wget doit rester dans l'image CRI: {wget_txt}"
    );
    assert!(
        !wget_reached_target(&wget_txt),
        "plugin ne doit pas joindre le canary system: {wget_txt}"
    );
    assert!(
        wget_blocked_signal(&wget_txt) || !wget.status.success(),
        "T11: canary bloqué depuis le plugin: {wget_txt}"
    );

    let _ = cluster.kubectl(&[
        "wait",
        "--for=condition=Ready",
        "pod",
        "-n",
        SYSTEM_NAMESPACE,
        "-l",
        CANARY_LABEL,
        "--timeout=30s",
    ]);
    drop(pf);
}
