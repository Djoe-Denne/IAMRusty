//! Apparatus P4 — T6 signataire + registry (IT Docker, pas Kind).
//!
//! Enveloppe ORAS non-CRI, identité catalogue = digest 0002, Cosign+Transit,
//! zot signer-only push. Fail-loud si Docker ne démarre pas les fixtures.

use std::fs;
use std::path::{Path, PathBuf};

use apparatus_operator::admit::{
    push_and_sign_envelope, signature_tag_exists, verify_cosign_signature, AdmitTarget, Envelope,
    RegistryAuth, ENVELOPE_ARTIFACT_TYPE,
};
#[cfg(feature = "controller")]
use apparatus_operator::controller::{envelope_to_podspec, IsolationLabels};
use apparatus_operator::{digest_manifest, parse_manifest};
use serial_test::serial;

#[path = "fixtures/mod.rs"]
mod fixtures;

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

const SYNTHETIC_CRI: &str = "ghcr.io/aiforall/platform-plugin@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const FORBIDDEN_BUILD_IDENTS: [&str; 9] = [
    "kube",
    "k8s_openapi",
    "reqwest",
    "cosign",
    "oras",
    "vault",
    "axum",
    "TcpListener",
    "create_router",
];

const FORBIDDEN_BUILD_SUBSTRINGS: [&str; 2] = ["k8s-openapi", "TransitClient"];

const LAZARET_NEEDLES: [&str; 2] = ["lazaret_test-openbao", "lazaret-dev-root"];

fn repo_src(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn descriptor_digest() -> apparatus_operator::ReleaseDigest {
    let validated = parse_manifest(VALID_MANIFEST_TOML).expect("manifeste P0 / T5 candidat");
    digest_manifest(&validated.manifest).expect("digest 0002")
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
        tag: "t6".to_owned(),
        auth: RegistryAuth {
            username: fixtures::zot::SIGNER_USER.to_owned(),
            password: fixtures::zot::SIGNER_PASSWORD.to_owned(),
        },
    }
}

fn envelope() -> Envelope {
    Envelope {
        descriptor_digest: descriptor_digest(),
        cri_image: SYNTHETIC_CRI.to_owned(),
    }
}

const fn is_ident_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn contains_ident(haystack: &str, ident: &str) -> bool {
    let mut from = 0;
    while from < haystack.len() {
        let Some(rel) = haystack[from..].find(ident) else {
            return false;
        };
        let abs = from + rel;
        let before = abs
            .checked_sub(1)
            .and_then(|i| haystack.as_bytes().get(i).copied());
        let after = haystack.as_bytes().get(abs + ident.len()).copied();
        let left_ok = before.is_none_or(|c| !is_ident_char(c));
        let right_ok = after.is_none_or(|c| !is_ident_char(c));
        if left_ok && right_ok {
            return true;
        }
        from = abs + ident.len();
    }
    false
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("dossier {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("entrée lisible").path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t6_docker_zot_openbao_start_or_fail_loud() {
    let stack = stack().await;
    let zot = reqwest::Client::new()
        .get(format!("{}/v2/", stack.zot.host_base_url()))
        .send()
        .await
        .unwrap_or_else(|e| panic!("zot /v2/ injoignable (Docker?): {e}"));
    assert!(
        zot.status().as_u16() < 500,
        "zot /v2/ status={}",
        zot.status()
    );
    let bao = reqwest::Client::new()
        .get(format!("{}/v1/sys/health", stack.transit.host_base_url()))
        .send()
        .await
        .unwrap_or_else(|e| panic!("OpenBao health injoignable (Docker?): {e}"));
    assert!(
        matches!(bao.status().as_u16(), 200 | 429 | 472 | 473),
        "OpenBao health status={}",
        bao.status()
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t6_admit_signer_pushes_envelope_and_signs() {
    let stack = stack().await;
    let signed = push_and_sign_envelope(&admit_target(&stack), &envelope())
        .await
        .unwrap_or_else(|e| panic!("push+sign: {e}"));
    assert!(
        signed.signature_verified,
        "Cosign+Transit doit produire une signature vérifiable"
    );
    assert_eq!(signed.artifact_type, ENVELOPE_ARTIFACT_TYPE);
    assert!(
        !signed.artifact_type.contains("vnd.oci.image.index"),
        "artifact type ne doit pas être un index CRI: {}",
        signed.artifact_type
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t6_verify_ko_with_sig_tag_refuses() {
    let stack = stack().await;
    let target = admit_target(&stack);
    let signed = push_and_sign_envelope(&target, &envelope())
        .await
        .unwrap_or_else(|e| panic!("push+sign: {e}"));
    assert!(
        signature_tag_exists(&target, &signed.reference).unwrap_or(false),
        "précondition: tag .sig présent pour {}",
        signed.reference
    );
    let mut wrong_key = target.clone();
    wrong_key.transit_key = "t6-wrong-verify-key".to_owned();
    let err = verify_cosign_signature(&wrong_key, &signed.reference)
        .expect_err("verify KO + tag .sig ne doit pas admettre");
    let msg = format!("{err}");
    assert!(
        msg.to_ascii_lowercase().contains("cosign verify"),
        "refus verify, pas succès via tag .sig: {msg}"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t6_catalog_identity_is_descriptor_digest_not_envelope_oci() {
    let stack = stack().await;
    let descriptor = descriptor_digest();
    let signed = push_and_sign_envelope(&admit_target(&stack), &envelope())
        .await
        .unwrap_or_else(|e| panic!("push+sign: {e}"));
    assert_eq!(
        signed.catalog_digest, descriptor,
        "identité catalogue = ReleaseDigest 0002"
    );
    assert_ne!(
        signed.envelope_oci_digest,
        descriptor.as_str(),
        "digest OCI d'enveloppe ≠ descriptor_digest"
    );
    assert!(
        signed.envelope_oci_digest.starts_with("sha256:"),
        "digest OCI={}",
        signed.envelope_oci_digest
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t6_envelope_pins_cri_image_digest() {
    let stack = stack().await;
    let signed = push_and_sign_envelope(&admit_target(&stack), &envelope())
        .await
        .unwrap_or_else(|e| panic!("push+sign: {e}"));
    assert!(
        signed.cri_image.contains("@sha256:"),
        "cri_image doit être pinée @sha256: {}",
        signed.cri_image
    );
    assert!(
        !signed.cri_image.ends_with(":latest"),
        "cri_image ne doit pas être latest: {}",
        signed.cri_image
    );
    assert_eq!(signed.cri_image, SYNTHETIC_CRI);
}

/// Pin CRI : après `@sha256:` exactement 64 hex minuscules ; `latest` interdit.
#[tokio::test]
async fn t6_cri_pin_rejects_short_or_non_hex() {
    const HEX64: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const HEX64_UPPER: &str = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";
    const HEX63: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde";

    let dummy = AdmitTarget {
        registry_host: "unused".to_owned(),
        registry_network: "unused".to_owned(),
        docker_network: "unused".to_owned(),
        vault_addr_host: "http://127.0.0.1:1".to_owned(),
        vault_addr_network: "unused".to_owned(),
        vault_token: "unused".to_owned(),
        transit_key: "unused".to_owned(),
        repository: "unused".to_owned(),
        tag: "unused".to_owned(),
        auth: RegistryAuth {
            username: "unused".to_owned(),
            password: "unused".to_owned(),
        },
    };

    let rejected = [
        "foo@sha256:dead".to_owned(),
        format!("foo@sha256:{HEX63}"),
        format!("foo@sha256:{HEX64_UPPER}"),
        format!("foo:latest@sha256:{HEX64}"),
    ];

    for cri_image in &rejected {
        let env = Envelope {
            descriptor_digest: descriptor_digest(),
            cri_image: cri_image.clone(),
        };
        let err = push_and_sign_envelope(&dummy, &env)
            .await
            .expect_err(&format!(
                "admit doit refuser le pin court/non-hex/latest: {cri_image}"
            ));
        let msg = format!("{err}");
        assert!(
            msg.contains("cri_image must be pinned") || msg.contains("must not use tag latest"),
            "admit doit échouer sur le pin (pas Transit): {cri_image}: {msg}"
        );
    }

    #[cfg(feature = "controller")]
    {
        let isolation = IsolationLabels::try_new("p", "b").expect("isolation p+b");
        for cri_image in &rejected {
            let err = envelope_to_podspec(cri_image, &isolation).expect_err(&format!(
                "contrôleur doit refuser le pin court/non-hex/latest: {cri_image}"
            ));
            let msg = format!("{err}");
            assert!(
                msg.contains("spec.image must be pinned")
                    || msg.contains("must not use tag latest"),
                "contrôleur doit échouer sur le pin: {cri_image}: {msg}"
            );
        }
        envelope_to_podspec(SYNTHETIC_CRI, &isolation).unwrap_or_else(|err| {
            panic!("SYNTHETIC_CRI 64 hex minuscules doit rester accepté: {err}")
        });
    }
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t6_build_anonymous_push_forbidden() {
    let stack = stack().await;
    let url = format!(
        "{}/v2/apparatus/build-forbidden/blobs/uploads/",
        stack.zot.host_base_url()
    );
    let response = reqwest::Client::new()
        .post(&url)
        .send()
        .await
        .unwrap_or_else(|e| panic!("POST anonyme zot: {e}"));
    let status = response.status().as_u16();
    assert!(
        status == 401 || status == 403,
        "push anonyme/build doit être 401/403, got {status}"
    );
}

#[test]
#[serial]
fn t6_fixture_names_are_not_lazaret() {
    assert_eq!(fixtures::zot::CONTAINER_NAME, "apparatus_operator_test-zot");
    assert_eq!(
        fixtures::openbao_transit::CONTAINER_NAME,
        "apparatus_operator_test-openbao-transit"
    );
    assert_eq!(
        fixtures::openbao_transit::ROOT_TOKEN,
        "apparatus-operator-transit-dev"
    );
    let dir = repo_src("tests/fixtures");
    let mut files = Vec::new();
    collect_rs(&dir, &mut files);
    assert!(!files.is_empty(), "fixtures rust attendues");
    let mut hits = Vec::new();
    for file in &files {
        let content =
            fs::read_to_string(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        for needle in LAZARET_NEEDLES {
            if content.contains(needle) {
                hits.push(format!("{}: {needle}", file.display()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "fixtures T6 ne doivent pas réutiliser les noms Lazaret: {hits:?}"
    );
}

#[test]
#[serial]
fn t6_build_sources_have_no_transit_client() {
    let files = [repo_src("src/bin/build.rs"), repo_src("src/build.rs")];
    let mut hits = Vec::new();
    for file in &files {
        let content =
            fs::read_to_string(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        for ident in FORBIDDEN_BUILD_IDENTS {
            if contains_ident(&content, ident) {
                hits.push(format!("{}: ident {ident}", file.display()));
            }
        }
        for needle in FORBIDDEN_BUILD_SUBSTRINGS {
            if content.contains(needle) {
                hits.push(format!("{}: {needle}", file.display()));
            }
        }
    }
    let admit = fs::read_to_string(repo_src("src/admit.rs")).expect("src/admit.rs T6");
    assert!(
        !admit.contains("Command::new(\"cargo\")"),
        "le signer ne doit pas exécuter le compilateur / code auteur"
    );
    assert!(
        hits.is_empty(),
        "Transit/registry interdits dans build: {hits:?}"
    );
}
