//! Apparatus P4 — T11 Kind : runtime hors Manifesto (digest 0002 + NP réelle).
//!
//! Fail-loud si Docker/Kind absents. Schedule seulement avec CR `AdmissionRecord`
//! VALID. Preuve isolation : exec/probe depuis le Pod plugin, pas un assert YAML.
//! Tuer le plugin ne casse pas le canary `apparatus-system`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;

use apparatus_operator::admission::{AdmissionStore, AdmitInput, PersistentAdmissionStore};
use apparatus_operator::admit::{push_and_sign_envelope, AdmitTarget, Envelope, RegistryAuth};
use apparatus_operator::controller::{
    cr_name_for, pod_name_for, IsolationLabels, ReconcileOutcome, ScheduleRefuse,
    WorkloadReconciler, BINDING_LABEL, GROUP, KIND, PLUGINS_NAMESPACE, PROJECT_LABEL,
    SYSTEM_NAMESPACE, VERSION,
};
use apparatus_operator::{
    digest_manifest, evaluate_conformance, parse_manifest, ReleaseDigest, POLICY_ID,
};
use serial_test::serial;

#[allow(dead_code)]
#[path = "fixtures/with_kind.rs"]
mod fixtures;

const VALID_MANIFEST_TOML: &str = r#"
[apparatus]
id = "io.aiforall.reference-kv"
version = "0.1.11"
schema_version = 1

[backend]
protocol = "manifesto-apparatus/1"
storage = "kv-v1"

[capabilities]
requires = ["project.read", "storage.kv.read", "storage.kv.write"]

[ui]
mode = "schema"
schema_path = "ui/settings.schema.json"
"#;

const NOT_VALID_MANIFEST_TOML: &str = r#"
[apparatus]
id = "io.aiforall.p4-not-valid"
version = "0.1.0"
schema_version = 1

[backend]
protocol = "manifesto-apparatus/1"
storage = "kv-v1"

[capabilities]
requires = ["project.read", "storage.kv.read", "storage.kv.write"]

[ui]
mode = "schema"
schema_path = "ui/settings.schema.json"
"#;

const SYNTHETIC_CRI: &str = "ghcr.io/aiforall/platform-plugin@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const CANARY_LABEL: &str = "app=apparatus-system-canary";
const CANARY_URL: &str = "http://apparatus-system-canary.apparatus-system.svc.cluster.local:8080/";
const TICK_SRC: &str = "Manifesto/infra/src/apparatus_runtime/tick.rs";

fn descriptor_and_report(toml: &str) -> (ReleaseDigest, ReleaseDigest, bool) {
    let validated = parse_manifest(toml).expect("manifeste P0 / T5 candidat");
    let descriptor = digest_manifest(&validated.manifest).expect("digest 0002");
    let report = evaluate_conformance(toml);
    (descriptor, report.report_digest, report.passed)
}

fn isolation() -> IsolationLabels {
    IsolationLabels::try_new("presses-nord", "shift-0300").expect("labels isolation")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR a un parent (racine du dépôt)")
        .to_path_buf()
}

fn kubectl_text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn wget_missing(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    (lower.contains("executable file not found") && lower.contains("wget"))
        || lower.contains("wget: not found")
        || lower.contains("wget: applet not found")
        || lower.contains("wget: no such file")
        || (lower.contains("command not found") && lower.contains("wget"))
}

fn wget_dns_inconclusive(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("bad address")
        || lower.contains("name or service not known")
        || lower.contains("temporary failure in name resolution")
        || lower.contains("could not resolve host")
        || lower.contains("no such host")
}

fn wget_blocked_signal(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("connection refused")
        || lower.contains("timed out")
        || lower.contains("wget: timeout")
}

fn wget_reached_target(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("http/1.")
        || lower.contains("200 ok")
        || lower.contains("403 forbidden")
        || lower.contains("404 not")
}

async fn connect(cluster: &fixtures::kind::KindCluster, store_path: &Path) -> WorkloadReconciler {
    WorkloadReconciler::connect(&cluster.kubeconfig, store_path)
        .await
        .unwrap_or_else(|err| panic!("{err}"))
}

fn unique_store_path(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("horloge")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "apparatus-t11-{label}-{}-{nanos}.json",
        std::process::id()
    ))
}

async fn reset_digest(reconciler: &WorkloadReconciler, descriptor: &ReleaseDigest) {
    let pod_name = pod_name_for(descriptor);
    reconciler
        .delete_plugin_pod(&pod_name)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    reconciler
        .delete_admission_record(descriptor)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
}

fn apply_record_without_status(
    cluster: &fixtures::kind::KindCluster,
    descriptor: &ReleaseDigest,
    report_digest: &ReleaseDigest,
    cri_image: &str,
) {
    let name = cr_name_for(descriptor);
    let iso = isolation();
    let mut yaml = String::new();
    yaml.push_str(&format!("apiVersion: {GROUP}/{VERSION}\n"));
    yaml.push_str(&format!("kind: {KIND}\n"));
    yaml.push_str("metadata:\n");
    yaml.push_str(&format!("  name: {name}\n"));
    yaml.push_str(&format!("  namespace: {SYSTEM_NAMESPACE}\n"));
    yaml.push_str("  labels:\n");
    yaml.push_str(&format!("    \"{PROJECT_LABEL}\": \"{}\"\n", iso.project));
    yaml.push_str(&format!("    \"{BINDING_LABEL}\": \"{}\"\n", iso.binding));
    yaml.push_str("spec:\n");
    yaml.push_str(&format!("  descriptorDigest: {}\n", descriptor.as_str()));
    yaml.push_str(&format!("  policyVersion: {POLICY_ID}\n"));
    yaml.push_str(&format!("  reportDigest: {}\n", report_digest.as_str()));
    yaml.push_str(&format!("  criImage: {cri_image}\n"));
    let path = std::env::temp_dir().join(format!("{name}.yaml"));
    fs::write(&path, yaml).unwrap_or_else(|err| panic!("write CR yaml: {err}"));
    let path_s = path.to_string_lossy().into_owned();
    let apply = cluster
        .kubectl(&["apply", "-f", path_s.as_str()])
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(
        apply.status.success(),
        "kubectl apply AdmissionRecord sans status: stdout={} stderr={}",
        String::from_utf8_lossy(&apply.stdout),
        String::from_utf8_lossy(&apply.stderr)
    );
}

async fn schedule_plugin(
    cluster: &fixtures::kind::KindCluster,
) -> (WorkloadReconciler, String, ReleaseDigest) {
    let (descriptor, report_digest, passed) = descriptor_and_report(VALID_MANIFEST_TOML);
    assert!(passed, "conformance T5 du manifeste P0");
    let cri_image = cluster
        .build_and_load_platform_plugin()
        .unwrap_or_else(|err| panic!("{err}"));
    wait_system_canary(cluster);
    let stack = fixtures::SignRegistryStack::start()
        .await
        .unwrap_or_else(|err| panic!("SignRegistryStack: {err}"));
    let target = AdmitTarget {
        registry_host: format!("127.0.0.1:{}", stack.zot.port),
        registry_network: fixtures::zot::TestZot::network_registry(),
        docker_network: fixtures::SignRegistryStack::network_name().to_owned(),
        vault_addr_host: stack.transit.host_base_url(),
        vault_addr_network: fixtures::openbao_transit::TestOpenBaoTransit::network_base_url(),
        vault_token: fixtures::openbao_transit::TestOpenBaoTransit::token().to_owned(),
        transit_key: fixtures::openbao_transit::TestOpenBaoTransit::key_name().to_owned(),
        repository: "apparatus/envelope".to_owned(),
        tag: "t11".to_owned(),
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
    let store_path = unique_store_path("schedule");
    let mut store = PersistentAdmissionStore::open(&store_path).expect("open JSON M3");
    let record = store
        .admit(AdmitInput {
            descriptor_digest: descriptor.clone(),
            observed_descriptor: signed.catalog_digest.clone(),
            policy_id: POLICY_ID.to_owned(),
            report_digest,
            conformance_passed: passed,
            signature_verified: signed.signature_verified,
            claimed_verified: false,
        })
        .expect("admit VALID");
    store
        .bind_cri(&record.descriptor_digest, &cri_image)
        .expect("bind_cri pin JSON");
    store
        .bind_envelope(&record.descriptor_digest, &signed.reference)
        .expect("bind_envelope ref T6");
    let reconciler = connect(cluster, &store_path)
        .await
        .with_schedule_admit_target(target);
    reset_digest(&reconciler, &descriptor).await;
    reconciler
        .apply_valid_record(&record, &cri_image, Some(&isolation()))
        .await
        .unwrap_or_else(|err| panic!("apply CR VALID: {err}"));
    let outcome = reconciler
        .reconcile_digest(&descriptor)
        .await
        .unwrap_or_else(|err| panic!("reconcile: {err}"));
    let pod_name = pod_name_for(&descriptor);
    match outcome {
        ReconcileOutcome::Scheduled {
            pod_name: scheduled,
        } => {
            assert_eq!(scheduled, pod_name);
        }
        ReconcileOutcome::Refused(refuse) => panic!("schedule attendu, refus={refuse}"),
    }
    reconciler
        .wait_pod_running(&pod_name, Duration::from_secs(120))
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    (reconciler, pod_name, descriptor)
}

fn wait_system_canary(cluster: &fixtures::kind::KindCluster) {
    let _ = cluster.kubectl(&[
        "delete",
        "pod",
        "-n",
        SYSTEM_NAMESPACE,
        "-l",
        CANARY_LABEL,
        "--wait=true",
    ]);
    let wait = cluster
        .kubectl(&[
            "wait",
            "--for=condition=Ready",
            "pod",
            "-n",
            SYSTEM_NAMESPACE,
            "-l",
            CANARY_LABEL,
            "--timeout=120s",
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(
        wait.status.success(),
        "canary apparatus-system Ready: {}",
        kubectl_text(&wait)
    );
}

fn assert_canary_ready(cluster: &fixtures::kind::KindCluster) {
    let wait = cluster
        .kubectl(&[
            "wait",
            "--for=condition=Ready",
            "pod",
            "-n",
            SYSTEM_NAMESPACE,
            "-l",
            CANARY_LABEL,
            "--timeout=30s",
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(
        wait.status.success(),
        "canary apparatus-system doit rester Ready: {}",
        kubectl_text(&wait)
    );
}

fn exec_wget(
    cluster: &fixtures::kind::KindCluster,
    namespace: &str,
    pod: &str,
    url: &str,
) -> Output {
    cluster
        .kubectl(&[
            "--request-timeout=20s",
            "exec",
            "-n",
            namespace,
            pod,
            "--",
            "wget",
            "-S",
            "-T",
            "5",
            "-O",
            "-",
            url,
        ])
        .unwrap_or_else(|err| panic!("{err}"))
}

fn canary_pod_name(cluster: &fixtures::kind::KindCluster) -> String {
    let out = cluster
        .kubectl(&[
            "get",
            "pod",
            "-n",
            SYSTEM_NAMESPACE,
            "-l",
            CANARY_LABEL,
            "-o",
            "jsonpath={.items[0].metadata.name}",
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    let name = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    assert!(!name.is_empty(), "canary pod name: {}", kubectl_text(&out));
    name
}

fn assert_plugin_blocked(cluster: &fixtures::kind::KindCluster, pod: &str, url: &str) {
    let out = exec_wget(cluster, PLUGINS_NAMESPACE, pod, url);
    let text = kubectl_text(&out);
    assert!(
        !wget_missing(&text),
        "wget absent dans le Pod plugin (pas un blocage NP): {text}"
    );
    assert!(
        !wget_dns_inconclusive(&text),
        "échec DNS depuis le plugin (pas un blocage NP): {text}"
    );
    let lower = text.to_ascii_lowercase();
    assert!(
        !lower.contains("network is unreachable") && !lower.contains("no route to host"),
        "CNI injoignable (pas une preuve NP): {text}"
    );
    assert!(
        !wget_reached_target(&text),
        "plugin {pod} ne doit pas joindre {url} (NP réelle, HTTP): status={:?} {text}",
        out.status.code()
    );
    assert!(
        wget_blocked_signal(&text),
        "plugin {pod} doit être bloqué (timeout/refused) vers {url}: status={:?} {text}",
        out.status.code()
    );
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
    for entry in entries {
        let path = entry.expect("entrée lisible").path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name == ".git" {
                continue;
            }
            collect_rs(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn manifesto_crate_src_rs() -> Vec<PathBuf> {
    let manifesto = repo_root().join("Manifesto");
    let mut files = Vec::new();
    let entries = fs::read_dir(&manifesto).expect("Manifesto/ lisible");
    for entry in entries {
        let path = entry.expect("entrée lisible").path();
        if !path.is_dir() {
            continue;
        }
        let src = path.join("src");
        if src.is_dir() {
            collect_rs(&src, &mut files);
        }
    }
    files
}

fn token_hits(files: &[PathBuf], needles: &[&str]) -> Vec<String> {
    let mut hits = Vec::new();
    for path in files {
        let content =
            fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        let lower = content.to_ascii_lowercase();
        for needle in needles {
            if lower.contains(needle) {
                hits.push(format!("{}: {needle}", path.display()));
            }
        }
    }
    hits
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t11_no_schedule_without_admission_record() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    let (descriptor, _report_digest, passed) = descriptor_and_report(VALID_MANIFEST_TOML);
    assert!(passed, "conformance T5 du manifeste P0");

    let store_path = unique_store_path("no-cr");
    let reconciler = connect(&cluster, &store_path).await;
    reset_digest(&reconciler, &descriptor).await;
    let exists = reconciler
        .plugin_pod_exists(&pod_name_for(&descriptor))
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(
        !exists,
        "sans AdmissionRecord : zéro Pod {}",
        pod_name_for(&descriptor)
    );
}

/// CR présente hors phase VALID : pas de Pod (garde NotValid).
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t11_no_schedule_when_admission_record_not_valid() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    let (descriptor, report_digest, passed) = descriptor_and_report(NOT_VALID_MANIFEST_TOML);
    assert!(passed, "conformance T5 du manifeste P0");

    let store_path = unique_store_path("not-valid");
    let reconciler = connect(&cluster, &store_path).await;
    reset_digest(&reconciler, &descriptor).await;
    apply_record_without_status(&cluster, &descriptor, &report_digest, SYNTHETIC_CRI);

    let outcome = reconciler
        .reconcile_digest(&descriptor)
        .await
        .unwrap_or_else(|err| panic!("reconcile: {err}"));
    assert_eq!(outcome, ReconcileOutcome::Refused(ScheduleRefuse::NotValid));
    let exists = reconciler
        .plugin_pod_exists(&pod_name_for(&descriptor))
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(
        !exists,
        "AdmissionRecord NotValid : zéro Pod {}",
        pod_name_for(&descriptor)
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t11_plugin_exec_cannot_reach_transit_or_system() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    let transit = fixtures::openbao_transit::TestOpenBaoTransit::new()
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    let transit_url = format!("http://host.docker.internal:{}/v1/sys/health", transit.port);
    let (_reconciler, pod_name, _descriptor) = schedule_plugin(&cluster).await;

    let canary = canary_pod_name(&cluster);
    let local = exec_wget(
        &cluster,
        SYSTEM_NAMESPACE,
        &canary,
        "http://127.0.0.1:8080/",
    );
    let local_txt = kubectl_text(&local);
    assert!(
        wget_reached_target(&local_txt),
        "canary system doit servir HTTP en local: {local_txt}"
    );

    assert_plugin_blocked(&cluster, &pod_name, &transit_url);
    assert_plugin_blocked(&cluster, &pod_name, CANARY_URL);
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t11_delete_plugin_leaves_system_canary_ready() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    let (reconciler, pod_name, _descriptor) = schedule_plugin(&cluster).await;
    assert_canary_ready(&cluster);
    reconciler
        .delete_plugin_pod(&pod_name)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    let exists = reconciler
        .plugin_pod_exists(&pod_name)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(!exists, "Pod plugin {pod_name} doit être Gone");
    assert_canary_ready(&cluster);
}

#[test]
fn t11_manifesto_src_has_no_k8s_and_controller_is_not_tick() {
    let files = manifesto_crate_src_rs();
    assert!(!files.is_empty(), "Manifesto/*/src doit contenir du Rust");

    let adapter_files: Vec<_> = files
        .iter()
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case("kubernetes_adapter.rs"))
        })
        .map(|path| path.display().to_string())
        .collect();
    assert!(
        adapter_files.is_empty(),
        "pas de kubernetes_adapter.rs sous Manifesto/*/src: {adapter_files:?}"
    );

    let type_hits = token_hits(&files, &["kubernetesadapter"]);
    assert!(
        type_hits.is_empty(),
        "pas de type KubernetesAdapter sous Manifesto/*/src: {type_hits:?}"
    );

    let k8s_hits = token_hits(&files, &["k8s", "kubernetes"]);
    assert!(
        k8s_hits.is_empty(),
        "zéro token k8s/kubernetes sous Manifesto/*/src: {k8s_hits:?}"
    );

    let operator_hits = token_hits(&files, &["apparatus-operator", "apparatus_operator"]);
    assert!(
        operator_hits.is_empty(),
        "Manifesto n'importe pas apparatus-operator: {operator_hits:?}"
    );

    let controller = fs::read_to_string(repo_root().join("apparatus-operator/src/controller.rs"))
        .expect("controller.rs lisible");
    assert!(
        !controller.contains("tick.rs"),
        "contrôleur P4 ≠ {TICK_SRC}"
    );
    assert!(
        !controller.contains("apparatus_runtime"),
        "contrôleur P4 n'importe pas Manifesto tick.rs"
    );
    assert!(
        !controller.to_ascii_lowercase().contains("wasmtime"),
        "WASM n'est pas le premier runtime P4"
    );

    let tick = repo_root().join(TICK_SRC);
    assert!(
        tick.is_file(),
        "{TICK_SRC} (P2) existe toujours, distinct du contrôleur"
    );
    let tick_src = fs::read_to_string(&tick).expect("tick.rs lisible");
    assert!(
        !tick_src.contains("apparatus-operator") && !tick_src.contains("apparatus_operator"),
        "tick.rs n'importe pas apparatus-operator"
    );
}

#[test]
fn t11_wget_oracle_is_http_only() {
    assert!(wget_reached_target("  HTTP/1.1 200 OK"));
    assert!(wget_reached_target("HTTP/1.0 403 Forbidden"));
    assert!(wget_reached_target("404 Not Found"));
    assert!(!wget_reached_target("initialized"));
    assert!(!wget_reached_target("wget: not found"));
    assert!(!wget_reached_target("wget: download timed out"));
    assert!(!wget_reached_target(
        "wget: can't connect to remote host: Connection refused"
    ));
}

#[test]
fn t11_wget_missing_is_not_np_block() {
    assert!(wget_missing("/bin/sh: wget: not found"));
    assert!(wget_missing(
        "exec: \"wget\": executable file not found in $PATH"
    ));
    assert!(!wget_missing("HTTP/1.1 404 Not Found"));
    assert!(wget_dns_inconclusive("wget: bad address 'missing.svc'"));
    assert!(wget_blocked_signal("wget: download timed out"));
    assert!(wget_blocked_signal("can't connect: Connection refused"));
    assert!(!wget_blocked_signal("wget: not found"));
}
