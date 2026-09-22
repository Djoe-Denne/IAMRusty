//! M1 — pin produit de l'Apparatus de référence (`io.aiforall.reference-kv`).
//!
//! Enveloppe pinée sur zot : digest descripteur 0002 (`sha256:` + 64 hex, jamais
//! `latest`) + blob image CRI distinct, issus du manifeste et du contexte
//! `apparatus-reference-kv/`. Pas `testdata/platform-plugin`. Premier artifact
//! de la chaîne, pas un rejeu T3–T6.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use apparatus_operator::admit::{
    push_and_sign_envelope, AdmitTarget, Envelope, RegistryAuth, ENVELOPE_ARTIFACT_TYPE,
};
use apparatus_operator::{digest_manifest, parse_manifest, resolve_install_ref};
use serial_test::serial;

#[path = "fixtures/mod.rs"]
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

fn descriptor_from_disk() -> apparatus_operator::ReleaseDigest {
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

fn assert_descriptor_shape(descriptor: &apparatus_operator::ReleaseDigest) {
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

fn admit_target(stack: &fixtures::SignRegistryStack) -> AdmitTarget {
    AdmitTarget {
        registry_host: format!("127.0.0.1:{}", stack.zot.port),
        registry_network: fixtures::zot::TestZot::network_registry(),
        docker_network: fixtures::SignRegistryStack::network_name().to_owned(),
        vault_addr_host: stack.transit.host_base_url(),
        vault_addr_network: fixtures::openbao_transit::TestOpenBaoTransit::network_base_url(),
        vault_token: fixtures::openbao_transit::TestOpenBaoTransit::token().to_owned(),
        transit_key: fixtures::openbao_transit::TestOpenBaoTransit::key_name().to_owned(),
        repository: "apparatus/reference-kv".to_owned(),
        tag: "m1".to_owned(),
        auth: RegistryAuth {
            username: fixtures::zot::SIGNER_USER.to_owned(),
            password: fixtures::zot::SIGNER_PASSWORD.to_owned(),
        },
    }
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn m1_reference_kv_pinned_envelope_on_zot() {
    let descriptor = descriptor_from_disk();
    assert_descriptor_shape(&descriptor);
    let descriptor_hex = descriptor
        .as_str()
        .strip_prefix("sha256:")
        .expect("shape déjà assertée");

    let cri_image = docker_build_reference_kv_cri_pin();
    let cri_hex = cri_image
        .split_once("@sha256:")
        .map(|(_, hex)| hex)
        .unwrap_or_else(|| panic!("cri_image pin @sha256: {cri_image}"));
    assert_ne!(
        cri_hex,
        descriptor_hex,
        "blob CRI ≠ hex descripteur 0002 (cri={cri_image}, descriptor={})",
        descriptor.as_str()
    );
    assert!(
        !cri_image.contains(":latest"),
        "cri_image sans latest: {cri_image}"
    );

    let stack = fixtures::SignRegistryStack::start()
        .await
        .unwrap_or_else(|err| panic!("{err}"));
    let envelope = Envelope {
        descriptor_digest: descriptor.clone(),
        cri_image: cri_image.clone(),
    };
    let signed = push_and_sign_envelope(&admit_target(&stack), &envelope)
        .await
        .unwrap_or_else(|err| panic!("push+sign enveloppe reference-kv: {err}"));

    assert_eq!(
        signed.catalog_digest, descriptor,
        "identité catalogue = digest descripteur 0002"
    );
    assert_ne!(
        signed.envelope_oci_digest,
        descriptor.as_str(),
        "digest OCI d'enveloppe ≠ identité catalogue"
    );
    assert!(
        signed.envelope_oci_digest.starts_with("sha256:"),
        "enveloppe OCI={}",
        signed.envelope_oci_digest
    );
    assert!(
        signed.cri_image.contains("@sha256:"),
        "cri_image pinée @sha256: {}",
        signed.cri_image
    );
    assert!(
        !signed.cri_image.contains(":latest"),
        "cri_image sans latest: {}",
        signed.cri_image
    );
    assert_eq!(signed.cri_image, cri_image);
    assert_eq!(signed.artifact_type, ENVELOPE_ARTIFACT_TYPE);
    eprintln!(
        "M1 pin: descriptor={} cri={} envelope_oci={}",
        descriptor.as_str(),
        signed.cri_image,
        signed.envelope_oci_digest
    );
}
