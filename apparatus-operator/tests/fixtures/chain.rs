//! Helpers partagés pour la chaîne M6 (M1–M5). Pas six tests recopiés.
//!
//! Inclus seulement par `apparatus_m6_e2e_chain` (`super::fixtures` = Kind + zot).

use std::fs;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use apparatus_contracts::{BindingId, InvokeRequest, new_operation_id};
use apparatus_operator::admission::AdmitInput;
use apparatus_operator::admit::{AdmitTarget, Envelope, RegistryAuth};
use apparatus_operator::{
    POLICY_ID, ReleaseDigest, digest_manifest, evaluate_conformance, parse_manifest,
    resolve_install_ref,
};
use async_trait::async_trait;
use lazaret_application::{PluginEndpointLocator, StaticPluginLocator, empty_command_registry};
use lazaret_domain::{
    AsyncKvStore, BindingGrantSnapshot, BindingGrantSnapshotPort, CapabilityConsent,
    GrantFetchError,
};
use rcgen::{CertificateParams, KeyPair};
use rustycog::command::GenericCommandService;
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use super::fixtures;

/// Identité Apparatus de référence (descripteur 0002).
pub const REFERENCE_KV_ID: &str = "io.aiforall.reference-kv";
/// Nom d'image CRI (sans tag flottant).
pub const CRI_IMAGE_NAME: &str = "apparatus-reference-kv";
/// Tag de build local HTTP (invoke).
pub const CRI_BUILD_TAG: &str = "apparatus-reference-kv:m6";
/// Sélecteur canary T11.
pub const CANARY_LABEL: &str = "app=apparatus-system-canary";
/// URL canary Transit / system — sonde Kind, pas un scan T12.
pub const CANARY_URL: &str =
    "http://apparatus-system-canary.apparatus-system.svc.cluster.local:8080/";

/// Racine du workspace (parent de `apparatus-operator/`).
#[must_use]
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Répertoire `apparatus-reference-kv/`.
#[must_use]
pub fn reference_kv_dir() -> PathBuf {
    repo_root().join("apparatus-reference-kv")
}

/// Digest descripteur 0002 du manifeste disque.
#[must_use]
pub fn descriptor_from_disk() -> ReleaseDigest {
    let raw = fs::read_to_string(reference_kv_dir().join("apparatus.toml"))
        .unwrap_or_else(|err| panic!("apparatus.toml: {err}"));
    let validated = parse_manifest(&raw).unwrap_or_else(|err| panic!("parse_manifest: {err}"));
    assert_eq!(
        validated.manifest.apparatus.id.as_str(),
        REFERENCE_KV_ID,
        "identité Apparatus de référence"
    );
    digest_manifest(&validated.manifest).expect("digest descripteur 0002")
}

/// Forme ADR-0002 : `sha256:` + 64 hex minuscules, jamais `latest`.
pub fn assert_descriptor_shape(descriptor: &ReleaseDigest) {
    let digest = descriptor.as_str();
    let hex = digest
        .strip_prefix("sha256:")
        .unwrap_or_else(|| panic!("descripteur 0002 doit commencer par sha256: {digest}"));
    assert_eq!(
        hex.len(),
        64,
        "descripteur 0002 = sha256: + 64 hex: {digest}"
    );
    assert!(
        hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
        "descripteur 0002 hex minuscules: {digest}"
    );
    assert!(
        !digest.contains("latest"),
        "identité catalogue jamais latest: {digest}"
    );
    resolve_install_ref(digest).unwrap_or_else(|err| {
        panic!("resolve_install_ref doit accepter le digest 0002 {digest}: {err}")
    });
}

/// Entrée Adm-A qui passe T5 pour le manifeste de référence.
#[must_use]
pub fn passing_input() -> (AdmitInput, ReleaseDigest) {
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

/// Fichier JSON `PersistentAdmissionStore` (pas `InMemoryAdmissionStore`).
pub struct TempStorePath {
    /// Chemin du JSON produit.
    pub path: PathBuf,
}

impl TempStorePath {
    /// Alloue un chemin unique sous `{temp}`.
    #[must_use]
    pub fn new(label: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("horloge")
            .as_nanos();
        Self {
            path: std::env::temp_dir().join(format!(
                "apparatus-m6-{label}-{}-{nanos}.json",
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

/// Build `Dockerfile.http` (serveur invoke). Skip si le tag existe déjà.
pub fn docker_build_reference_kv_http() {
    fixtures::assert_docker_running().unwrap_or_else(|err| panic!("{err}"));
    let inspect = Command::new("docker")
        .args(["image", "inspect", CRI_BUILD_TAG])
        .output();
    if inspect.is_ok_and(|out| out.status.success()) {
        return;
    }
    let staging = std::env::temp_dir().join(format!("apparatus-m6-cri-{}", std::process::id()));
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

/// Cible ORAS/Cosign (zot + Transit) pour l'enveloppe M1.
#[must_use]
pub fn admit_target(stack: &fixtures::SignRegistryStack) -> AdmitTarget {
    AdmitTarget {
        registry_host: format!("127.0.0.1:{}", stack.zot.port),
        registry_network: fixtures::zot::TestZot::network_registry(),
        docker_network: fixtures::SignRegistryStack::network_name().to_owned(),
        vault_addr_host: stack.transit.host_base_url(),
        vault_addr_network: fixtures::openbao_transit::TestOpenBaoTransit::network_base_url(),
        vault_token: fixtures::openbao_transit::TestOpenBaoTransit::token().to_owned(),
        transit_key: fixtures::openbao_transit::TestOpenBaoTransit::key_name().to_owned(),
        repository: "apparatus/reference-kv".to_owned(),
        tag: "m6".to_owned(),
        auth: RegistryAuth {
            username: fixtures::zot::SIGNER_USER.to_owned(),
            password: fixtures::zot::SIGNER_PASSWORD.to_owned(),
        },
    }
}

/// Enveloppe catalogue + pin CRI (push zot).
#[must_use]
pub fn catalog_envelope(descriptor: ReleaseDigest, cri_image: String) -> Envelope {
    Envelope {
        descriptor_digest: descriptor,
        cri_image,
    }
}

/// Stub HTTP Manifesto (contrat observateur M4, pas un fake operator).
pub struct ManifestoStub {
    /// Base `/manifesto`.
    pub base_url: String,
}

/// Sert GET `.../components` avec le JSON fourni.
pub async fn spawn_manifesto_stub(body: serde_json::Value) -> ManifestoStub {
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

/// Binding ready : `status==active` + digest 0002.
#[must_use]
pub fn active_component_json(
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

/// `kubectl port-forward` (Drop = kill).
pub struct PortForward {
    child: Child,
}

impl PortForward {
    /// Enveloppe un child kubectl.
    #[must_use]
    pub fn new(child: Child) -> Self {
        Self { child }
    }
}

impl Drop for PortForward {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Attend un TCP local.
pub fn wait_tcp(addr: &str, timeout: Duration) {
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

/// Grant MemoryPort (chemin allow/deny, pas OpenFGA).
pub struct MemoryPort {
    snapshot: BindingGrantSnapshot,
}

impl MemoryPort {
    /// Snapshot figé.
    #[must_use]
    pub fn new(snapshot: BindingGrantSnapshot) -> Self {
        Self { snapshot }
    }
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

/// KV no-op (non-hop M5).
pub struct NoopKv;

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

/// Compte les hops `PluginEndpointLocator` (preuve deny sans hop).
pub struct CountingLocator {
    inner: StaticPluginLocator,
    hops: Arc<AtomicUsize>,
}

impl CountingLocator {
    /// Locator injecté vers l'URL port-forward du Pod.
    #[must_use]
    pub fn new(plugin_url: String, hops: Arc<AtomicUsize>) -> Self {
        Self {
            inner: StaticPluginLocator::new(plugin_url),
            hops,
        }
    }
}

#[async_trait]
impl PluginEndpointLocator for CountingLocator {
    async fn locate(&self, binding_id: &str, instance_id: &str) -> Option<String> {
        self.hops.fetch_add(1, Ordering::SeqCst);
        self.inner.locate(binding_id, instance_id).await
    }
}

/// Snapshot grant (consentement optionnel).
#[must_use]
pub fn snapshot(
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

/// CSR workload P3.
#[must_use]
pub fn workload_csr() -> String {
    let key = KeyPair::generate().expect("key");
    let params = CertificateParams::new(vec!["workload.test".to_owned()]).expect("params");
    params
        .serialize_request(&key)
        .expect("csr")
        .pem()
        .expect("pem")
}

/// `AppState` rustycog in-process (composition unique, pas un 6e hexagone).
#[must_use]
pub fn app_state() -> AppState {
    let command_registry = empty_command_registry();
    let command_service = Arc::new(GenericCommandService::new(Arc::new(command_registry)));
    let extractor = UserIdExtractor::from_resolved_secret("test-hs256-secret").expect("jwt");
    let checker: Arc<dyn PermissionChecker> = Arc::new(InMemoryPermissionChecker::new());
    AppState::new(command_service, extractor, checker)
}

/// Corps `POST /invoke`.
#[must_use]
pub fn invoke_body(binding: Uuid, operation: &str, params: serde_json::Value) -> InvokeRequest {
    InvokeRequest {
        binding_id: binding.to_string().parse::<BindingId>().expect("id"),
        operation_id: new_operation_id(),
        operation: operation.to_owned(),
        params,
    }
}

/// HTTP 200/404 = canary atteint (T11 échoué).
#[must_use]
pub fn wget_reached_target(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("http/1.") || lower.contains(" 200 ") || lower.contains(" 404 ")
}

/// Signaux de blocage NetworkPolicy (sonde Kind, pas T12).
#[must_use]
pub fn wget_blocked_signal(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("download timed out")
        || lower.contains("wget: timeout")
        || lower.contains("connection refused")
        || lower.contains("can't connect")
        || lower.contains("network is unreachable")
        || lower.contains("no route to host")
}

/// Hex CRI `@sha256:` (fail-loud si pin absent).
#[must_use]
pub fn cri_hex(cri_image: &str) -> &str {
    cri_image
        .split_once("@sha256:")
        .map(|(_, hex)| hex)
        .unwrap_or_else(|| panic!("cri_image pin @sha256: {cri_image}"))
}
