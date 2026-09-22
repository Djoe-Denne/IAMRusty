//! M3 — install fail-closed si non-`VALID` (JSON produit, pas la CR Kind).
//!
//! Une CR `AdmissionRecord` phase VALID sans ligne JSON n'installe pas.
//! Identité = digest descripteur 0002 (`apparatus-reference-kv/apparatus.toml`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use apparatus_operator::admission::{
    would_schedule, AdmissionRecord, AdmissionStatus, AdmissionStore, AdmitInput, AdmitRefuse,
    BindCriError, PersistentAdmissionStore,
};
use apparatus_operator::controller::{
    pod_name_for, IsolationLabels, ReconcileOutcome, ScheduleRefuse, WorkloadReconciler,
    PLUGINS_NAMESPACE,
};
use apparatus_operator::{
    digest_manifest, evaluate_conformance, parse_manifest, ReleaseDigest, POLICY_ID,
};
use serial_test::serial;

#[allow(dead_code)]
#[path = "fixtures/with_kind.rs"]
mod fixtures;

const REFERENCE_KV_ID: &str = "io.aiforall.reference-kv";
const CRI_IMAGE_NAME: &str = "apparatus-reference-kv";
const CRI_BUILD_TAG: &str = "apparatus-reference-kv:m1";
const UNKNOWN_DIGEST: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SYNTHETIC_CRI: &str = "ghcr.io/aiforall/reference-kv@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const ADMITTED_CRI: &str = "ghcr.io/aiforall/admitted@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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
        "apparatus-m3-{label}-{}-{nanos}.json",
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

fn isolation() -> IsolationLabels {
    IsolationLabels::try_new("presses-nord", "shift-0300").expect("labels isolation")
}

fn cr_only_record(descriptor: &ReleaseDigest, report_digest: ReleaseDigest) -> AdmissionRecord {
    AdmissionRecord {
        descriptor_digest: descriptor.clone(),
        policy_version: POLICY_ID.to_owned(),
        report_digest,
        status: AdmissionStatus::Valid,
        cri_image: None,
    }
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

#[test]
fn m3_would_schedule_only_after_json_valid_admit() {
    let tmp = TempStorePath::new("unit");
    let store = PersistentAdmissionStore::open(&tmp.path).expect("open store vide");
    let (input, descriptor, _) = passing_input();
    assert!(
        !would_schedule(&store, &descriptor),
        "store vide : would_schedule == false"
    );
    let unknown = ReleaseDigest::new(UNKNOWN_DIGEST).expect("digest canonique");
    assert!(
        !would_schedule(&store, &unknown),
        "digest non admis : would_schedule == false"
    );
    drop(store);

    let mut refused_store = PersistentAdmissionStore::open(&tmp.path).expect("open refus");
    let mut refused_input = input.clone();
    refused_input.conformance_passed = false;
    refused_input.signature_verified = false;
    let refused = refused_store.admit(refused_input);
    assert!(
        matches!(
            refused,
            Err(AdmitRefuse::ManifestNonconformant | AdmitRefuse::UnexpectedSignature)
        ),
        "refus Adm-A attendu, obtenu {refused:?}"
    );
    assert!(
        !would_schedule(&refused_store, &descriptor),
        "après refus admit : would_schedule == false"
    );
    drop(refused_store);

    let mut store = PersistentAdmissionStore::open(&tmp.path).expect("open admit VALID");
    store.admit(input).expect("admit M1 VALID");
    assert!(
        would_schedule(&store, &descriptor),
        "après admit VALID du digest M1 : would_schedule == true"
    );
    assert!(
        !would_schedule(&store, &unknown),
        "digest étranger reste non installable"
    );
}

#[test]
fn m3_bind_cri_persists_pin_only_when_valid() {
    let tmp = TempStorePath::new("bind-cri");
    let (input, descriptor, _) = passing_input();
    let mut store = PersistentAdmissionStore::open(&tmp.path).expect("open store vide");
    let unknown = ReleaseDigest::new(UNKNOWN_DIGEST).expect("digest canonique");
    let missing = store.bind_cri(&descriptor, SYNTHETIC_CRI);
    assert!(
        matches!(missing, Err(BindCriError::NotValid)),
        "bind_cri sans VALID : {missing:?}"
    );
    assert!(store.get(&descriptor).is_none());
    assert!(!would_schedule(&store, &descriptor));

    store.admit(input).expect("admit M1 VALID");
    assert!(
        would_schedule(&store, &descriptor),
        "would_schedule true seulement après admit"
    );
    assert_eq!(
        store.get(&descriptor).and_then(|record| record.cri_image),
        None,
        "admit n'écrit pas le pin CRI"
    );
    store
        .bind_cri(&descriptor, SYNTHETIC_CRI)
        .expect("bind_cri pin M1-shape");
    assert_eq!(
        store.get(&descriptor).and_then(|record| record.cri_image),
        Some(SYNTHETIC_CRI.to_owned())
    );
    assert!(
        would_schedule(&store, &descriptor),
        "bind_cri ne retire pas la ligne VALID"
    );
    let still_missing = store.bind_cri(&unknown, SYNTHETIC_CRI);
    assert!(
        matches!(still_missing, Err(BindCriError::NotValid)),
        "bind_cri digest étranger : {still_missing:?}"
    );
    drop(store);

    let reopened = PersistentAdmissionStore::open(&tmp.path).expect("reopen");
    assert_eq!(
        reopened
            .get(&descriptor)
            .and_then(|record| record.cri_image),
        Some(SYNTHETIC_CRI.to_owned()),
        "pin persisté dans le JSON"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn m3_kind_cr_only_valid_does_not_schedule() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    let (descriptor, report_digest, passed) = m1_descriptor_and_report();
    assert!(passed, "conformance T5 du manifeste M1");
    let tmp = TempStorePath::new("cr-only");
    let store = PersistentAdmissionStore::open(&tmp.path).expect("store JSON vide");
    assert!(
        store.get(&descriptor).is_none(),
        "aucune ligne JSON VALID pour le digest M1"
    );
    drop(store);

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

    reconciler
        .apply_valid_record(
            &cr_only_record(&descriptor, report_digest),
            SYNTHETIC_CRI,
            Some(&isolation()),
        )
        .await
        .unwrap_or_else(|err| panic!("apply CR VALID sans JSON: {err}"));

    let outcome = reconciler
        .reconcile_digest(&descriptor)
        .await
        .unwrap_or_else(|err| panic!("reconcile: {err}"));
    assert_eq!(
        outcome,
        ReconcileOutcome::Refused(ScheduleRefuse::MissingAdmissionRecord),
        "CR-only VALID n'est pas une install produit"
    );
    let exists = reconciler
        .plugin_pod_exists(&pod_name)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(
        !exists,
        "CR Kind VALID sans ligne JSON : zéro Pod {pod_name}"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn m3_kind_json_valid_m1_schedules_pinned_pod() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    let (input, descriptor, _) = passing_input();
    let cri_image = docker_build_reference_kv_cri_pin();
    assert!(
        cri_image.contains("@sha256:"),
        "pin CRI enveloppe M1: {cri_image}"
    );
    assert!(
        !cri_image.contains(":latest"),
        "pin CRI sans latest: {cri_image}"
    );

    let tmp = TempStorePath::new("valid-m1");
    let mut store = PersistentAdmissionStore::open(&tmp.path).expect("open JSON");
    store.admit(input).expect("admit M1 JSON VALID");
    store
        .bind_cri(&descriptor, &cri_image)
        .expect("bind_cri pin M1 JSON");
    assert!(
        would_schedule(&store, &descriptor),
        "ligne JSON VALID requise avant schedule"
    );
    drop(store);

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

    let record = PersistentAdmissionStore::open(&tmp.path)
        .expect("reopen")
        .get(&descriptor)
        .expect("ligne VALID");
    reconciler
        .apply_valid_record(&record, &cri_image, Some(&isolation()))
        .await
        .unwrap_or_else(|err| panic!("apply CR + isolation: {err}"));

    let outcome = reconciler
        .reconcile_digest(&descriptor)
        .await
        .unwrap_or_else(|err| panic!("reconcile: {err}"));
    match outcome {
        ReconcileOutcome::Scheduled {
            pod_name: scheduled,
        } => assert_eq!(scheduled, pod_name),
        ReconcileOutcome::Refused(refuse) => panic!("schedule attendu, refus={refuse}"),
    }

    let exists = reconciler
        .plugin_pod_exists(&pod_name)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(exists, "JSON VALID + isolation + cri pin : Pod {pod_name}");

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
    assert!(
        !image_txt.contains(":latest"),
        "spec.image jamais latest: {image_txt}"
    );

    let policy = cluster
        .kubectl(&[
            "get",
            "pod",
            &pod_name,
            "-n",
            PLUGINS_NAMESPACE,
            "-o",
            "jsonpath={.spec.containers[0].imagePullPolicy}",
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(
        String::from_utf8_lossy(&policy.stdout).trim(),
        "IfNotPresent"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn m3_json_valid_cr_cri_mismatch_does_not_schedule() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    let (input, descriptor, _) = passing_input();
    let tmp = TempStorePath::new("mismatch");
    let mut store = PersistentAdmissionStore::open(&tmp.path).expect("open JSON");
    store.admit(input).expect("admit M1 JSON VALID");
    store
        .bind_cri(&descriptor, ADMITTED_CRI)
        .expect("bind_cri pin A");
    drop(store);

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

    let record = PersistentAdmissionStore::open(&tmp.path)
        .expect("reopen")
        .get(&descriptor)
        .expect("ligne VALID");
    reconciler
        .apply_valid_record(&record, SYNTHETIC_CRI, Some(&isolation()))
        .await
        .unwrap_or_else(|err| panic!("apply CR cri différent: {err}"));

    let outcome = reconciler
        .reconcile_digest(&descriptor)
        .await
        .unwrap_or_else(|err| panic!("reconcile: {err}"));
    assert_eq!(
        outcome,
        ReconcileOutcome::Refused(ScheduleRefuse::CriImageMismatch),
        "CR criImage ≠ pin JSON : refus, zéro Pod"
    );
    let exists = reconciler
        .plugin_pod_exists(&pod_name)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(!exists, "mismatch CRI : zéro Pod {pod_name}");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn m3_kind_json_valid_without_cr_does_not_schedule() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    let (input, descriptor, _) = passing_input();
    let tmp = TempStorePath::new("json-no-cr");
    let mut store = PersistentAdmissionStore::open(&tmp.path).expect("open JSON");
    store.admit(input).expect("admit M1 JSON VALID");
    store
        .bind_cri(&descriptor, SYNTHETIC_CRI)
        .expect("bind_cri pin JSON");
    drop(store);

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

    let outcome = reconciler
        .reconcile_digest(&descriptor)
        .await
        .unwrap_or_else(|err| panic!("reconcile: {err}"));
    assert_eq!(
        outcome,
        ReconcileOutcome::Refused(ScheduleRefuse::MissingAdmissionRecord),
        "JSON VALID sans CR : isolation/CR absente"
    );
    let exists = reconciler
        .plugin_pod_exists(&pod_name)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(!exists, "JSON VALID sans CR : zéro Pod {pod_name}");
}
