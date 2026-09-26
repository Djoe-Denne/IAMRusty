//! M4 — pont desired_state Manifesto → M3 (`WorkloadReconciler`).
//!
//! Observer HTTP hors `Manifesto/*/src` : GET list des 5 routes `/components`.
//! Ready = `status == "active"` et digest `sha256:` + 64 hex. Stub local, pas
//! rustycog-testing. Isolation = `project_id` + `component_id` (ADR-0001).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use apparatus_operator::admission::{
    would_schedule, AdmissionStore, AdmitInput, PersistentAdmissionStore,
};
use apparatus_operator::admit::{push_and_sign_envelope, AdmitTarget, Envelope, RegistryAuth};
use apparatus_operator::controller::{
    pod_name_for, IsolationLabels, ReconcileOutcome, ScheduleRefuse, WorkloadReconciler,
    BINDING_LABEL, PLUGINS_NAMESPACE, PROJECT_LABEL,
};
use apparatus_operator::desired_state::{
    reconcile_ready, DesiredStateSource, HttpComponentsClient, ReadyBinding,
};
use apparatus_operator::{
    digest_manifest, evaluate_conformance, parse_manifest, ReleaseDigest, POLICY_ID,
};
use serial_test::serial;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

#[allow(dead_code)]
#[path = "fixtures/with_kind.rs"]
mod fixtures;

const REFERENCE_KV_ID: &str = "io.aiforall.reference-kv";
const CRI_IMAGE_NAME: &str = "apparatus-reference-kv";
const CRI_BUILD_TAG: &str = "apparatus-reference-kv:m1";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn reference_kv_dir() -> PathBuf {
    repo_root().join("apparatus-reference-kv")
}

fn reference_kv_manifest() -> PathBuf {
    reference_kv_dir().join("apparatus.toml")
}

fn descriptor_from_disk() -> ReleaseDigest {
    let raw = fs::read_to_string(reference_kv_manifest()).unwrap_or_else(|err| {
        panic!("manifeste disque apparatus-reference-kv/apparatus.toml: {err}")
    });
    let validated = parse_manifest(&raw).unwrap_or_else(|err| {
        panic!("parse_manifest refuse le toml disque (name/description conservés): {err}")
    });
    assert_eq!(
        validated.manifest.apparatus.id.as_str(),
        REFERENCE_KV_ID,
        "identité Apparatus de référence"
    );
    digest_manifest(&validated.manifest).expect("digest descripteur 0002")
}

fn m1_descriptor_and_report() -> (ReleaseDigest, ReleaseDigest, bool) {
    let raw = fs::read_to_string(reference_kv_manifest()).unwrap_or_else(|err| {
        panic!("manifeste disque apparatus-reference-kv/apparatus.toml: {err}")
    });
    let descriptor = descriptor_from_disk();
    let report = evaluate_conformance(&raw);
    (descriptor, report.report_digest, report.passed)
}

fn unique_store_path(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("horloge")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "apparatus-m4-{label}-{}-{nanos}.json",
        std::process::id()
    ))
}

struct TempStorePath {
    path: PathBuf,
}

impl TempStorePath {
    fn new(label: &str) -> Self {
        Self {
            path: unique_store_path(label),
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

fn passing_input() -> (AdmitInput, ReleaseDigest, ReleaseDigest) {
    let (descriptor, report_digest, passed) = m1_descriptor_and_report();
    assert!(passed, "le manifeste de référence M1 doit passer T5");
    let input = AdmitInput {
        descriptor_digest: descriptor.clone(),
        observed_descriptor: descriptor.clone(),
        policy_id: POLICY_ID.to_owned(),
        report_digest: report_digest.clone(),
        conformance_passed: passed,
        signature_verified: true,
        claimed_verified: false,
    };
    (input, descriptor, report_digest)
}

/// Build local du contexte `apparatus-reference-kv/` (pas testdata/platform-plugin).
fn docker_build_reference_kv_cri_pin() -> String {
    fixtures::assert_docker_running().unwrap_or_else(|err| panic!("{err}"));
    let context = reference_kv_dir();
    let dockerfile = context.join("Dockerfile");
    assert!(
        dockerfile.is_file(),
        "Dockerfile produit absent: {}",
        dockerfile.display()
    );
    let build = Command::new("docker")
        .args(["build", "-t", CRI_BUILD_TAG])
        .arg(&context)
        .output()
        .unwrap_or_else(|err| panic!("Docker daemon must be running (docker build: {err})"));
    assert!(
        build.status.success(),
        "docker build {CRI_BUILD_TAG} (contexte apparatus-reference-kv): status={:?}\n{}",
        build.status.code(),
        String::from_utf8_lossy(&build.stderr)
    );
    let inspect = Command::new("docker")
        .args(["inspect", "--format", "{{.Id}}", CRI_BUILD_TAG])
        .output()
        .unwrap_or_else(|err| panic!("docker inspect {CRI_BUILD_TAG}: {err}"));
    assert!(
        inspect.status.success(),
        "docker inspect {CRI_BUILD_TAG}: {}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    let id = String::from_utf8_lossy(&inspect.stdout).trim().to_owned();
    let hex = id
        .strip_prefix("sha256:")
        .unwrap_or_else(|| panic!("docker Id doit être sha256:<64 hex>, obtenu: {id}"));
    assert_eq!(hex.len(), 64, "docker Id hex: {id}");
    assert!(
        hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
        "docker Id hex minuscules: {id}"
    );
    format!("{CRI_IMAGE_NAME}@sha256:{hex}")
}

struct ManifestoStub {
    base_url: String,
    #[allow(dead_code)]
    project_id: Uuid,
    #[allow(dead_code)]
    component_id: Uuid,
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
    let project_id = body["data"][0]["id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::nil);
    let component_id = body["data"]
        .as_array()
        .and_then(|rows| {
            rows.iter().find_map(|row| {
                (row["status"].as_str() == Some("active") && row.get("digest").is_some())
                    .then(|| row["id"].as_str().and_then(|s| Uuid::parse_str(s).ok()))
                    .flatten()
            })
        })
        .unwrap_or(project_id);
    ManifestoStub {
        base_url: format!("http://{addr}/manifesto"),
        project_id: body
            .get("_project_id")
            .and_then(serde_json::Value::as_str)
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::nil),
        component_id,
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

fn http_client(stub: &ManifestoStub) -> HttpComponentsClient {
    HttpComponentsClient::new(stub.base_url.clone(), stub.project_id, None::<String>)
        .unwrap_or_else(|err| panic!("HttpComponentsClient: {err}"))
}

#[tokio::test]
async fn m4_http_skips_non_active_and_missing_digest() {
    let project_id = Uuid::new_v4();
    let ready_id = Uuid::new_v4();
    let disabled_id = Uuid::new_v4();
    let no_digest_id = Uuid::new_v4();
    let pending_id = Uuid::new_v4();
    let malformed_id = Uuid::new_v4();
    let descriptor = descriptor_from_disk();
    let body = serde_json::json!({
        "_project_id": project_id.to_string(),
        "data": [
            {
                "id": disabled_id,
                "component_type": "apparatus",
                "status": "disabled",
                "added_at": "2026-09-22T00:00:00Z",
                "digest": descriptor.as_str(),
            },
            {
                "id": no_digest_id,
                "component_type": "apparatus",
                "status": "active",
                "added_at": "2026-09-22T00:00:00Z",
            },
            {
                "id": pending_id,
                "component_type": "apparatus",
                "status": "pending",
                "added_at": "2026-09-22T00:00:00Z",
                "digest": "not-a-digest",
            },
            {
                "id": malformed_id,
                "component_type": "apparatus",
                "status": "active",
                "added_at": "2026-09-22T00:00:00Z",
                "digest": "not-a-digest",
            },
            {
                "id": ready_id,
                "component_type": "apparatus",
                "status": "active",
                "added_at": "2026-09-22T00:00:00Z",
                "digest": descriptor.as_str(),
            }
        ]
    });
    let stub = spawn_manifesto_stub(body).await;
    let client = HttpComponentsClient::new(stub.base_url, project_id, None::<String>)
        .unwrap_or_else(|err| panic!("{err}"));
    let ready = client
        .list_ready()
        .await
        .unwrap_or_else(|err| panic!("list_ready: {err}"));
    assert_eq!(
        ready,
        vec![ReadyBinding {
            project_id,
            component_id: ready_id,
            digest: descriptor,
        }]
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn m4_ready_valid_digest_schedules() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    let stack = fixtures::SignRegistryStack::start()
        .await
        .unwrap_or_else(|err| panic!("SignRegistryStack: {err}"));
    let (mut input, descriptor, _) = passing_input();
    let cri_image = docker_build_reference_kv_cri_pin();
    assert!(
        cri_image.contains("@sha256:"),
        "pin CRI enveloppe M1: {cri_image}"
    );
    assert!(
        !cri_image.contains(":latest"),
        "pin CRI sans latest: {cri_image}"
    );
    let target = AdmitTarget {
        registry_host: format!("127.0.0.1:{}", stack.zot.port),
        registry_network: fixtures::zot::TestZot::network_registry(),
        docker_network: fixtures::SignRegistryStack::network_name().to_owned(),
        vault_addr_host: stack.transit.host_base_url(),
        vault_addr_network: fixtures::openbao_transit::TestOpenBaoTransit::network_base_url(),
        vault_token: fixtures::openbao_transit::TestOpenBaoTransit::token().to_owned(),
        transit_key: fixtures::openbao_transit::TestOpenBaoTransit::key_name().to_owned(),
        repository: "apparatus/envelope".to_owned(),
        tag: "m4".to_owned(),
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
    assert!(
        would_schedule(&store, &descriptor),
        "ligne JSON VALID requise avant schedule"
    );
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

    let exists = reconciler
        .plugin_pod_exists(&pod_name)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(exists, "JSON VALID + isolation stub : Pod {pod_name}");

    let image = cluster
        .kubectl(&[
            "get",
            "pod",
            &pod_name,
            "-n",
            PLUGINS_NAMESPACE,
            "-o",
            "jsonpath={.spec.containers[0].image}",
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    let image_txt = String::from_utf8_lossy(&image.stdout).trim().to_owned();
    assert_eq!(image_txt, cri_image, "spec.containers[0].image == pin JSON");

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
        "label projet = project_id stub: {labels_txt}"
    );
    assert!(
        labels_txt.contains(&format!("{BINDING_LABEL}={}", isolation.binding)),
        "label binding = component_id stub: {labels_txt}"
    );

    let uid_first = cluster
        .kubectl(&[
            "get",
            "pod",
            &pod_name,
            "-n",
            PLUGINS_NAMESPACE,
            "-o",
            "jsonpath={.metadata.uid}",
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    let uid_first = String::from_utf8_lossy(&uid_first.stdout).trim().to_owned();
    let again = reconcile_ready(&source, &reconciler, &store)
        .await
        .unwrap_or_else(|err| panic!("2e pont: {err}"));
    assert_eq!(again.len(), 1);
    match &again[0] {
        ReconcileOutcome::Scheduled {
            pod_name: scheduled,
        } => assert_eq!(*scheduled, pod_name),
        ReconcileOutcome::Refused(refuse) => panic!("2e pont Scheduled attendu, refus={refuse}"),
    }
    let uid_second = cluster
        .kubectl(&[
            "get",
            "pod",
            &pod_name,
            "-n",
            PLUGINS_NAMESPACE,
            "-o",
            "jsonpath={.metadata.uid}",
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    let uid_second = String::from_utf8_lossy(&uid_second.stdout)
        .trim()
        .to_owned();
    assert_eq!(uid_first, uid_second, "2e reconcile_ready : uid Pod stable");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn m4_ready_not_valid_does_not_schedule() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    let descriptor = descriptor_from_disk();
    let project_id = Uuid::new_v4();
    let component_id = Uuid::new_v4();
    let stub =
        spawn_manifesto_stub(active_component_json(project_id, component_id, &descriptor)).await;
    let source = http_client(&stub);

    let tmp = TempStorePath::new("empty");
    let store = PersistentAdmissionStore::open(&tmp.path).expect("store JSON vide");
    assert!(
        store.get(&descriptor).is_none(),
        "aucune ligne JSON VALID pour le digest M1"
    );

    let reconciler = WorkloadReconciler::connect(&cluster.kubeconfig, &tmp.path)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    let pod_name = pod_name_for(&descriptor);
    reconciler
        .delete_plugin_pod(&pod_name)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    reconciler
        .delete_admission_record(&descriptor)
        .await
        .unwrap_or_else(|err| panic!("{err}"));

    let outcomes = reconcile_ready(&source, &reconciler, &store)
        .await
        .unwrap_or_else(|err| panic!("bridge: {err}"));
    assert_eq!(outcomes.len(), 1, "un binding ready HTTP");
    match &outcomes[0] {
        ReconcileOutcome::Refused(refuse) => {
            assert_eq!(
                *refuse,
                ScheduleRefuse::MissingAdmissionRecord,
                "store vide sans CR : MissingAdmissionRecord, obtenu={refuse}"
            );
        }
        ReconcileOutcome::Scheduled { pod_name } => {
            panic!("zéro Pod attendu, scheduled={pod_name}")
        }
    }
    let exists = reconciler
        .plugin_pod_exists(&pod_name)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(!exists, "store vide : zéro Pod {pod_name}");
}
