//! Apparatus P4 — T9 worker malveillant / image plugin sans clés.
//!
//! Pas d'auto-admit, pas de signature, pas d'écriture registry, pas de secrets
//! IAM. Officiel et communautaire = même confinement. Fail-loud si Docker
//! n'atteint pas les fixtures T6. Pas de YAML Kind.

use std::fs;
use std::path::{Path, PathBuf};

use apparatus_operator::admission::{
    AdmissionStore, AdmitInput, AdmitRefuse, InMemoryAdmissionStore,
};
use apparatus_operator::admit::{
    inspect_transit_key, push_and_sign_envelope, AdmitTarget, Envelope, RegistryAuth,
};
use apparatus_operator::{
    community_policy_id, digest_manifest, evaluate_conformance, official_policy_id, parse_manifest,
    refuse_privileged_identity_from, report_grants_admission, would_schedule, ReleaseDigest,
    PRIVILEGED_IDENTITY_ENV,
};
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

const IAM_IDENTS: [&str; 3] = ["iam", "jwt", "openfga"];
const IAM_SUBSTRINGS: [&str; 1] = ["client_secret"];
const ADMIT_NEEDLES: [&str; 4] = [
    "InMemoryAdmissionStore",
    "AdmissionStore",
    "would_schedule",
    "AdmitInput",
];

fn repo_src(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn descriptor_digest() -> ReleaseDigest {
    let validated = parse_manifest(VALID_MANIFEST_TOML).expect("manifeste P0 / T5 candidat");
    digest_manifest(&validated.manifest).expect("digest 0002")
}

async fn stack() -> fixtures::SignRegistryStack {
    fixtures::SignRegistryStack::start()
        .await
        .unwrap_or_else(|err| panic!("{err}"))
}

fn admit_target(stack: &fixtures::SignRegistryStack, repository: &str, tag: &str) -> AdmitTarget {
    AdmitTarget {
        registry_host: format!("127.0.0.1:{}", stack.zot.port),
        registry_network: fixtures::zot::TestZot::network_registry(),
        docker_network: fixtures::SignRegistryStack::network_name().to_owned(),
        vault_addr_host: stack.transit.host_base_url(),
        vault_addr_network: fixtures::openbao_transit::TestOpenBaoTransit::network_base_url(),
        vault_token: fixtures::openbao_transit::TestOpenBaoTransit::token().to_owned(),
        transit_key: fixtures::openbao_transit::TestOpenBaoTransit::key_name().to_owned(),
        repository: repository.to_owned(),
        tag: tag.to_owned(),
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

fn unsigned_built_input(policy_id: &str) -> (InMemoryAdmissionStore, AdmitInput, ReleaseDigest) {
    let report = evaluate_conformance(VALID_MANIFEST_TOML);
    assert!(report.passed, "artifact construit T5 peut passer la suite");
    assert!(
        !report_grants_admission(&report),
        "conformance verte n'accorde pas VALID"
    );
    let descriptor = descriptor_digest();
    let input = AdmitInput {
        descriptor_digest: descriptor.clone(),
        observed_descriptor: descriptor.clone(),
        policy_id: policy_id.to_owned(),
        report_digest: report.report_digest,
        conformance_passed: report.passed,
        signature_verified: false,
        claimed_verified: true,
    };
    (InMemoryAdmissionStore::new(), input, descriptor)
}

async fn assert_push_forbidden(base: &str, repository: &str) {
    let url = format!("{base}/v2/{repository}/blobs/uploads/");
    let client = reqwest::Client::new();
    let anonymous = client
        .post(&url)
        .send()
        .await
        .unwrap_or_else(|e| panic!("POST anonyme zot {repository}: {e}"));
    let anon_status = anonymous.status().as_u16();
    assert!(
        anon_status == 401 || anon_status == 403,
        "push sans creds signer {repository} doit être 401/403, got {anon_status}"
    );
    let fake_worker = client
        .post(&url)
        .basic_auth("plugin-worker", Some("no-signer-creds"))
        .send()
        .await
        .unwrap_or_else(|e| panic!("POST worker zot {repository}: {e}"));
    let worker_status = fake_worker.status().as_u16();
    assert!(
        worker_status == 401 || worker_status == 403,
        "push worker sans creds signer {repository} doit être 401/403, got {worker_status}"
    );
}

fn assert_store_empty(store: &InMemoryAdmissionStore, digest: &ReleaseDigest, label: &str) {
    assert!(
        store.get(digest).is_none(),
        "{label}: InMemoryAdmissionStore doit rester vide"
    );
    assert!(
        !would_schedule(store, digest),
        "{label}: pas de schedule sans VALID"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t9_worker_without_zot_creds_cannot_push() {
    let stack = stack().await;
    assert_push_forbidden(&stack.zot.host_base_url(), "apparatus/malicious-worker").await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t9_worker_without_transit_token_cannot_sign() {
    let stack = stack().await;
    let addr = stack.transit.host_base_url();
    let key = fixtures::openbao_transit::TestOpenBaoTransit::key_name();
    inspect_transit_key(&addr, "", key)
        .await
        .expect_err("Transit sans token doit échouer");
    inspect_transit_key(&addr, "not-a-transit-token", key)
        .await
        .expect_err("Transit token worker doit échouer");

    let mut target = admit_target(&stack, "apparatus/malicious-unsigned", "t9-no-transit");
    target.vault_token.clear();
    target.auth = RegistryAuth {
        username: String::new(),
        password: String::new(),
    };
    let err = push_and_sign_envelope(&target, &envelope())
        .await
        .expect_err("push+sign sans Transit doit échouer");
    let message = err.to_string();
    assert!(
        message.contains("Transit") || message.contains("403") || message.contains("401"),
        "échec Cosign/Transit attendu, got {message}"
    );
    assert!(
        !message.contains(fixtures::openbao_transit::ROOT_TOKEN),
        "le message d'erreur ne doit pas fuiter le token Transit"
    );
}

#[test]
fn t9_plugin_worker_cannot_auto_admit_without_signature() {
    let (mut store, input, descriptor) = unsigned_built_input(official_policy_id());
    let refused = store.admit(input);
    assert_eq!(refused, Err(AdmitRefuse::UnexpectedSignature));
    assert_store_empty(&store, &descriptor, "plugin/worker unsigned");
}

#[test]
fn t9_build_has_no_iam_secrets_privileged_identity_enforced() {
    assert!(
        PRIVILEGED_IDENTITY_ENV.contains(&"VAULT_TOKEN"),
        "PRIVILEGED_IDENTITY_ENV doit toujours refuser Transit"
    );
    assert!(
        PRIVILEGED_IDENTITY_ENV.contains(&"REGISTRY_PASSWORD"),
        "PRIVILEGED_IDENTITY_ENV doit toujours refuser les creds registry"
    );
    for name in ["VAULT_TOKEN", "REGISTRY_PASSWORD", "KUBECONFIG"] {
        let err = refuse_privileged_identity_from(&[name], false)
            .expect_err("identité privilégiée rejetée");
        assert_eq!(err.code(), "APPARATUS_INVALID_OPERATION", "pour {name}");
    }
    refuse_privileged_identity_from(&[], false).expect("aucune identité injectée");

    let files = [repo_src("src/bin/build.rs"), repo_src("src/build.rs")];
    let mut hits = Vec::new();
    for file in &files {
        let content =
            fs::read_to_string(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let lower = content.to_ascii_lowercase();
        for ident in IAM_IDENTS {
            if contains_ident(&lower, ident) {
                hits.push(format!("{}: ident {ident}", file.display()));
            }
        }
        for needle in IAM_SUBSTRINGS {
            if lower.contains(needle) {
                hits.push(format!("{}: {needle}", file.display()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "secrets IAM interdits dans le worker build: {hits:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t9_official_and_community_same_confinement() {
    assert_eq!(
        official_policy_id(),
        community_policy_id(),
        "un seul protocole T5 / ADR-0005"
    );
    for policy in [official_policy_id(), community_policy_id()] {
        let (mut store, input, descriptor) = unsigned_built_input(policy);
        let refused = store.admit(input);
        assert_eq!(refused, Err(AdmitRefuse::UnexpectedSignature));
        assert_store_empty(&store, &descriptor, policy);
    }

    let stack = stack().await;
    let base = stack.zot.host_base_url();
    assert_push_forbidden(&base, "apparatus/official-plugin").await;
    assert_push_forbidden(&base, "apparatus/community-plugin").await;
}

#[test]
fn t9_build_module_does_not_call_admit() {
    let files = [repo_src("src/bin/build.rs"), repo_src("src/build.rs")];
    let mut hits = Vec::new();
    for file in &files {
        let content =
            fs::read_to_string(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        for needle in ADMIT_NEEDLES {
            if content.contains(needle) {
                hits.push(format!("{}: {needle}", file.display()));
            }
        }
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!")
            {
                continue;
            }
            if trimmed.contains("InMemoryAdmissionStore::admit") || trimmed.contains(".admit(") {
                hits.push(format!("{}:{}: appel admit", file.display(), idx + 1));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "le module build ne doit pas appeler InMemoryAdmissionStore::admit: {hits:?}"
    );
}
