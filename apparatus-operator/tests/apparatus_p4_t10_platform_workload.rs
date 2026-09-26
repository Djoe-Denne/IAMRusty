//! Apparatus P4 — T10 Kind : digest 0002 → enveloppe T6 → CR VALID → Pod piné.
//!
//! Fail-loud si Docker/Kind absents. Identité catalogue = `ReleaseDigest` 0002,
//! `spec.image` CRI `@sha256` ≠ digest OCI d'enveloppe. Isolation manquante
//! → aucun Pod.

use std::path::PathBuf;
use std::time::Duration;

use apparatus_operator::admission::{AdmissionStore, AdmitInput, PersistentAdmissionStore};
use apparatus_operator::admit::{push_and_sign_envelope, AdmitTarget, Envelope, RegistryAuth};
use apparatus_operator::controller::{
    pod_name_for, IsolationLabels, ReconcileOutcome, ScheduleRefuse, WorkloadReconciler,
    PLUGINS_NAMESPACE, SYSTEM_NAMESPACE,
};
use apparatus_operator::{
    digest_manifest, evaluate_conformance, parse_manifest, ReleaseDigest, POLICY_ID,
};
use serial_test::serial;

#[path = "fixtures/with_kind.rs"]
mod fixtures;

const SYNTHETIC_CRI: &str = "apparatus-p4-platform-plugin@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const VALID_MANIFEST_TOML: &str = r#"
[apparatus]
id = "io.aiforall.reference-kv"
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

fn descriptor_and_report(toml: &str) -> (ReleaseDigest, ReleaseDigest, bool) {
    let validated = parse_manifest(toml).expect("manifeste P0 / T5 candidat");
    let descriptor = digest_manifest(&validated.manifest).expect("digest 0002");
    let report = evaluate_conformance(toml);
    (descriptor, report.report_digest, report.passed)
}

fn isolation_manifest_toml() -> String {
    VALID_MANIFEST_TOML.replace("version = \"0.1.0\"", "version = \"0.1.1\"")
}

async fn stack() -> fixtures::SignRegistryStack {
    fixtures::SignRegistryStack::start()
        .await
        .unwrap_or_else(|err| panic!("{err}"))
}

fn admit_target(stack: &fixtures::SignRegistryStack) -> AdmitTarget {
    AdmitTarget {
        registry_host: format!("127.0.0.1:{}", stack.zot.port),
        registry_network: fixtures::zot::TestZot::network_registry(),
        docker_network: fixtures::SignRegistryStack::network_name().to_owned(),
        vault_addr_host: stack.transit.host_base_url(),
        vault_addr_network: fixtures::openbao_transit::TestOpenBaoTransit::network_base_url(),
        vault_token: fixtures::openbao_transit::TestOpenBaoTransit::token().to_owned(),
        transit_key: fixtures::openbao_transit::TestOpenBaoTransit::key_name().to_owned(),
        repository: "apparatus/envelope".to_owned(),
        tag: "t10".to_owned(),
        auth: RegistryAuth {
            username: fixtures::zot::SIGNER_USER.to_owned(),
            password: fixtures::zot::SIGNER_PASSWORD.to_owned(),
        },
    }
}

fn isolation() -> IsolationLabels {
    IsolationLabels::try_new("presses-nord", "shift-0300").expect("labels isolation")
}

fn unique_store_path(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("horloge")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "apparatus-t10-{label}-{}-{nanos}.json",
        std::process::id()
    ))
}

fn assert_ns_absent(cluster: &fixtures::kind::KindCluster, name: &str) {
    let out = cluster
        .kubectl(&["get", "ns", name])
        .unwrap_or_else(|err| panic!("{err}"));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !out.status.success() || text.contains("NotFound"),
        "namespace {name} ne doit pas exister: {text}"
    );
}

fn assert_sa(cluster: &fixtures::kind::KindCluster, name: &str) {
    let out = cluster
        .kubectl(&["get", "sa", name, "-n", SYSTEM_NAMESPACE])
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(
        out.status.success(),
        "ServiceAccount {name} manquant: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t10_valid_admission_runs_pinned_cri_pod() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    assert_ns_absent(&cluster, "shared");
    assert_ns_absent(&cluster, "organization");
    for sa in [
        "apparatus-build",
        "apparatus-admit",
        "apparatus-controller",
        "apparatus-gateway",
    ] {
        assert_sa(&cluster, sa);
    }
    let automount = cluster
        .kubectl(&[
            "get",
            "sa",
            "apparatus-build",
            "-n",
            SYSTEM_NAMESPACE,
            "-o",
            "jsonpath={.automountServiceAccountToken}",
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    let automount_txt = String::from_utf8_lossy(&automount.stdout);
    assert_eq!(
        automount_txt.trim(),
        "false",
        "apparatus-build doit avoir automountServiceAccountToken: false, got {automount_txt}"
    );

    let stack = stack().await;
    let cri_image = cluster
        .build_and_load_platform_plugin()
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(cri_image.contains("@sha256:"), "pin CRI: {cri_image}");
    assert!(
        !cri_image.contains(":latest"),
        "pin CRI ne doit pas contenir latest: {cri_image}"
    );

    let zot_url = format!("http://host.docker.internal:{}/v2/", stack.zot.port);
    cluster
        .probe_http_from_cluster(&zot_url)
        .unwrap_or_else(|err| panic!("{err}"));

    let (descriptor, report_digest, passed) = descriptor_and_report(VALID_MANIFEST_TOML);
    assert!(passed, "conformance T5 du manifeste P0");
    let signed = push_and_sign_envelope(
        &admit_target(&stack),
        &Envelope {
            descriptor_digest: descriptor.clone(),
            cri_image: cri_image.clone(),
        },
    )
    .await
    .unwrap_or_else(|err| panic!("push+sign: {err}"));
    assert_eq!(signed.catalog_digest, descriptor);
    assert_ne!(
        signed.envelope_oci_digest,
        descriptor.as_str(),
        "digest OCI enveloppe ≠ identity catalogue 0002"
    );
    assert_ne!(
        cri_image, signed.envelope_oci_digest,
        "pin CRI ≠ digest OCI enveloppe"
    );

    let store_path = unique_store_path("valid");
    let mut store = PersistentAdmissionStore::open(&store_path).expect("open JSON M3");
    let record = store
        .admit(AdmitInput {
            descriptor_digest: descriptor.clone(),
            observed_descriptor: signed.catalog_digest.clone(),
            policy_id: POLICY_ID.to_owned(),
            report_digest: report_digest.clone(),
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

    let reconciler = WorkloadReconciler::connect(&cluster.kubeconfig, &store_path)
        .await
        .unwrap_or_else(|err| panic!("{err}"))
        .with_schedule_admit_target(admit_target(&stack));
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
        .apply_valid_record(&record, &cri_image, Some(&isolation()))
        .await
        .unwrap_or_else(|err| panic!("apply CR VALID: {err}"));

    let outcome = reconciler
        .reconcile_digest(&descriptor)
        .await
        .unwrap_or_else(|err| panic!("reconcile: {err}"));
    match outcome {
        ReconcileOutcome::Scheduled {
            pod_name: scheduled,
        } => {
            assert_eq!(scheduled, pod_name);
        }
        ReconcileOutcome::Refused(refuse) => {
            panic!("schedule attendu, refus={refuse}");
        }
    }

    let image = reconciler
        .wait_pod_running(&pod_name, Duration::from_secs(120))
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(
        image.contains("@sha256:"),
        "spec.image doit être pinée @sha256: {image}"
    );
    assert!(
        !image.contains(":latest"),
        "spec.image ne doit pas être latest: {image}"
    );
    assert_ne!(
        image, signed.envelope_oci_digest,
        "spec.image ≠ digest OCI enveloppe"
    );
    assert_eq!(image, cri_image);

    let ns = cluster
        .kubectl(&["get", "pod", &pod_name, "-n", PLUGINS_NAMESPACE])
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(
        ns.status.success(),
        "Pod {pod_name} dans {PLUGINS_NAMESPACE}: stderr={}",
        String::from_utf8_lossy(&ns.stderr)
    );

    let pod_automount = cluster
        .kubectl(&[
            "get",
            "pod",
            &pod_name,
            "-n",
            PLUGINS_NAMESPACE,
            "-o",
            "jsonpath={.spec.automountServiceAccountToken}",
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    let pod_automount_txt = String::from_utf8_lossy(&pod_automount.stdout);
    assert_eq!(
        pod_automount_txt.trim(),
        "false",
        "Pod plugin automountServiceAccountToken doit être false, got {pod_automount_txt}"
    );
    let volumes = cluster
        .kubectl(&[
            "get",
            "pod",
            &pod_name,
            "-n",
            PLUGINS_NAMESPACE,
            "-o",
            "jsonpath={.spec.volumes[*].name}",
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    let volumes_txt = String::from_utf8_lossy(&volumes.stdout);
    assert!(
        !volumes_txt
            .split_whitespace()
            .any(|name| name.starts_with("kube-api-access-")),
        "Pod plugin ne doit pas monter kube-api-access-*: {volumes_txt}"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t10_missing_isolation_does_not_create_pod() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    let isolation_toml = isolation_manifest_toml();
    let (descriptor, report_digest, passed) = descriptor_and_report(&isolation_toml);
    let cri_image = SYNTHETIC_CRI;

    let store_path = unique_store_path("no-iso");
    let mut store = PersistentAdmissionStore::open(&store_path).expect("open JSON M3");
    let record = store
        .admit(AdmitInput {
            descriptor_digest: descriptor.clone(),
            observed_descriptor: descriptor.clone(),
            policy_id: POLICY_ID.to_owned(),
            report_digest,
            conformance_passed: passed,
            signature_verified: true,
            claimed_verified: false,
        })
        .expect("admit VALID sans isolation");
    store
        .bind_cri(&record.descriptor_digest, cri_image)
        .expect("bind_cri pin JSON");

    let reconciler = WorkloadReconciler::connect(&cluster.kubeconfig, &store_path)
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
        .apply_valid_record(&record, cri_image, None)
        .await
        .unwrap_or_else(|err| panic!("apply CR VALID sans labels: {err}"));

    let outcome = reconciler
        .reconcile_digest(&descriptor)
        .await
        .unwrap_or_else(|err| panic!("reconcile: {err}"));
    assert_eq!(
        outcome,
        ReconcileOutcome::Refused(ScheduleRefuse::MissingIsolation)
    );
    let exists = reconciler
        .plugin_pod_exists(&pod_name)
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(
        !exists,
        "isolation manquante ne doit créer aucun Pod {pod_name}"
    );
}

const ZOT_PULL_MANIFEST_TOML: &str = r#"
[apparatus]
id = "io.aiforall.p4-zot-pull"
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

/// Pull kubelet d'une image CRI pinée depuis zot HTTP (pas kind-load).
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t10_kubelet_pulls_pinned_cri_from_zot() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    let stack = stack().await;
    let cri_image = fixtures::kind::push_platform_plugin_to_zot(stack.zot.port)
        .unwrap_or_else(|err| panic!("{err}"));
    let store_path = unique_store_path("zot-pull");
    let (descriptor, report_digest, passed) = descriptor_and_report(ZOT_PULL_MANIFEST_TOML);
    assert!(passed, "conformance T5 du manifeste P0");
    let reconciler = WorkloadReconciler::connect(&cluster.kubeconfig, &store_path)
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
    cluster
        .ensure_zot_http_mirror(stack.zot.port)
        .unwrap_or_else(|err| panic!("{err}"));
    cluster
        .assert_registry_cri_absent(&cri_image)
        .unwrap_or_else(|err| panic!("{err}"));
    let mut target = admit_target(&stack);
    target.tag = "t10-zot".to_owned();
    let signed = push_and_sign_envelope(
        &target,
        &Envelope {
            descriptor_digest: descriptor.clone(),
            cri_image: cri_image.clone(),
        },
    )
    .await
    .unwrap_or_else(|err| panic!("push+sign: {err}"));
    assert_ne!(
        signed.envelope_oci_digest,
        descriptor.as_str(),
        "digest OCI enveloppe ≠ identity catalogue 0002"
    );

    let mut store = PersistentAdmissionStore::open(&store_path).expect("open JSON M3");
    let record = store
        .admit(AdmitInput {
            descriptor_digest: descriptor.clone(),
            observed_descriptor: signed.catalog_digest.clone(),
            policy_id: POLICY_ID.to_owned(),
            report_digest: report_digest.clone(),
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

    let reconciler = WorkloadReconciler::connect(&cluster.kubeconfig, &store_path)
        .await
        .unwrap_or_else(|err| panic!("{err}"))
        .with_schedule_admit_target(target);

    reconciler
        .apply_valid_record(&record, &cri_image, Some(&isolation()))
        .await
        .unwrap_or_else(|err| panic!("apply CR VALID: {err}"));

    let outcome = reconciler
        .reconcile_digest(&descriptor)
        .await
        .unwrap_or_else(|err| panic!("reconcile: {err}"));
    match outcome {
        ReconcileOutcome::Scheduled {
            pod_name: scheduled,
        } => {
            assert_eq!(scheduled, pod_name);
        }
        ReconcileOutcome::Refused(refuse) => {
            panic!("schedule attendu, refus={refuse}");
        }
    }

    let image = reconciler
        .wait_pod_running(&pod_name, Duration::from_secs(120))
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    assert!(
        image.starts_with("host.docker.internal:"),
        "spec.image doit être qualifiée registre: {image}"
    );
    let Some((_, hex)) = image.split_once("@sha256:") else {
        panic!("spec.image doit être pinée @sha256: {image}");
    };
    assert_eq!(hex.len(), 64, "digest CRI 64 hex: {image}");
    assert!(
        hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
        "digest CRI hex minuscules: {image}"
    );
    assert!(
        !image.contains(":latest"),
        "spec.image ne doit pas être latest: {image}"
    );
    assert_ne!(
        image, signed.envelope_oci_digest,
        "spec.image ≠ digest OCI enveloppe"
    );
    assert_eq!(image, cri_image);

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
    let policy_txt = String::from_utf8_lossy(&policy.stdout);
    let policy_trim = policy_txt.trim();
    assert_eq!(
        policy_trim, "IfNotPresent",
        "imagePullPolicy IfNotPresent: {policy_txt}"
    );

    let waiting = cluster
        .kubectl(&[
            "get",
            "pod",
            &pod_name,
            "-n",
            PLUGINS_NAMESPACE,
            "-o",
            "jsonpath={.status.containerStatuses[*].state.waiting.reason}",
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    let waiting_txt = String::from_utf8_lossy(&waiting.stdout);
    for needle in ["ErrImagePull", "ErrImageNeverPull"] {
        assert!(
            !waiting_txt.contains(needle),
            "waiting Pod sans {needle}: {waiting_txt}"
        );
    }
    let uid = cluster
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
    let uid_txt = String::from_utf8_lossy(&uid.stdout);
    let uid_trim = uid_txt.trim();
    assert!(!uid_trim.is_empty(), "uid Pod {pod_name} attendu");
    let events = cluster
        .kubectl(&[
            "get",
            "events",
            "-n",
            PLUGINS_NAMESPACE,
            "--field-selector",
            &format!("involvedObject.uid={uid_trim}"),
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    let events_txt = format!(
        "{}{}",
        String::from_utf8_lossy(&events.stdout),
        String::from_utf8_lossy(&events.stderr)
    );
    for needle in ["ErrImagePull", "ErrImageNeverPull"] {
        assert!(
            !events_txt.contains(needle),
            "événements uid={uid_trim} sans {needle}: {events_txt}"
        );
    }
}

fn can_i(cluster: &fixtures::kind::KindCluster, sa: &str, verb: &str, resource: &str) -> String {
    let as_user = format!("system:serviceaccount:{SYSTEM_NAMESPACE}:{sa}");
    let out = cluster
        .kubectl(&[
            "auth",
            "can-i",
            verb,
            resource,
            "--as",
            &as_user,
            "-n",
            SYSTEM_NAMESPACE,
        ])
        .unwrap_or_else(|err| panic!("{err}"));
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// Seul `apparatus-admit` peut créer la CR et patcher le statut VALID.
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t10_rbac_only_admit_writes_valid() {
    let cluster = fixtures::kind::KindCluster::ensure().unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(
        can_i(&cluster, "apparatus-admit", "create", "admissionrecords"),
        "yes",
        "apparatus-admit doit pouvoir create admissionrecords"
    );
    assert_eq!(
        can_i(
            &cluster,
            "apparatus-admit",
            "patch",
            "admissionrecords/status"
        ),
        "yes",
        "apparatus-admit doit pouvoir patch admissionrecords/status"
    );
    for sa in [
        "apparatus-controller",
        "apparatus-build",
        "apparatus-gateway",
    ] {
        assert_eq!(
            can_i(&cluster, sa, "create", "admissionrecords"),
            "no",
            "{sa} ne doit pas create admissionrecords"
        );
        assert_eq!(
            can_i(&cluster, sa, "patch", "admissionrecords/status"),
            "no",
            "{sa} ne doit pas patch admissionrecords/status"
        );
    }
}
