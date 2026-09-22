//! Apparatus P4 — T7 store d'admission in-memory (register ≠ admit).
//!
//! Seul le chemin `admit` (politique T5 + rapport + signature T6 booléenne)
//! écrit un enregistrement `Valid`. Enregistrer une release comme Manifesto
//! n'écrit pas de CR. Harness / publisher / plugin / `VERIFIED` ≠ admit.
//! Pas d'HTTP, pas de kube/CRD (T10).

use std::fs;
use std::path::{Path, PathBuf};

use apparatus_operator::admission::{
    manifesto_register_does_not_admit, register_manifesto_release, AdmissionRecord,
    AdmissionStatus, AdmissionStore, AdmitInput, AdmitRefuse, InMemoryAdmissionStore,
};
use apparatus_operator::{
    digest_manifest, evaluate_conformance, parse_manifest, report_grants_admission, ReleaseDigest,
    POLICY_ID,
};

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

const HARNESS_SRC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../apparatus-contracts/src/harness.rs"
));

const HTTP_LISTEN_TOKENS: [&str; 2] = ["axum", "TcpListener"];

const MANIFESTO_SRC_CRATES: [&str; 5] = ["domain", "application", "infra", "http", "setup"];

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

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR a un parent (racine du dépôt)")
        .to_path_buf()
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("dossier lisible {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("entrée lisible").path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name == ".git" {
                continue;
            }
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn operator_admission_src() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("admission.rs"),
    )
    .expect("src/admission.rs")
}

fn passing_report_and_descriptor() -> (ReleaseDigest, ReleaseDigest, bool) {
    let report = evaluate_conformance(VALID_MANIFEST_TOML);
    assert!(report.passed, "fixture T5 valide doit passer la suite");
    assert!(
        !report_grants_admission(&report),
        "T5 report_grants_admission reste false"
    );
    let validated = parse_manifest(VALID_MANIFEST_TOML).expect("manifeste P0");
    let descriptor = digest_manifest(&validated.manifest).expect("digest descripteur");
    (descriptor, report.report_digest, report.passed)
}

fn passing_input(claimed_verified: bool) -> (InMemoryAdmissionStore, AdmitInput, ReleaseDigest) {
    let (descriptor, report_digest, passed) = passing_report_and_descriptor();
    let input = AdmitInput {
        descriptor_digest: descriptor.clone(),
        observed_descriptor: descriptor.clone(),
        policy_id: POLICY_ID.to_owned(),
        report_digest,
        conformance_passed: passed,
        signature_verified: true,
        claimed_verified,
    };
    (InMemoryAdmissionStore::new(), input, descriptor)
}

#[test]
fn t7_successful_admit_get_returns_record() {
    let (mut store, input, descriptor) = passing_input(false);
    let report_digest = input.report_digest.clone();
    let record = store.admit(input).expect("admit valide");
    assert_eq!(record.descriptor_digest, descriptor);
    assert_eq!(record.policy_version, POLICY_ID);
    assert_eq!(record.report_digest, report_digest);
    assert_eq!(record.status, AdmissionStatus::Valid);
    let got = store
        .get(&descriptor)
        .expect("identité catalogue = descripteur");
    assert_eq!(got.descriptor_digest, descriptor);
    assert_eq!(got.policy_version, POLICY_ID);
    assert_eq!(got.report_digest, report_digest);
    assert_eq!(got.status, AdmissionStatus::Valid);
    assert_ne!(
        report_digest, descriptor,
        "le digest de rapport n'est pas l'identité catalogue"
    );
    assert!(
        store.get(&report_digest).is_none(),
        "get doit cléer sur descriptor_digest, pas report_digest"
    );
    let _: AdmissionRecord = got;
}

#[test]
fn t7_manifesto_register_does_not_write_valid() {
    let (store, _input, descriptor) = passing_input(false);
    register_manifesto_release(&store, &descriptor);
    manifesto_register_does_not_admit(&store, &descriptor);
    assert!(
        store.get(&descriptor).is_none(),
        "register Manifesto ne doit pas écrire de ligne Valid"
    );
}

#[test]
fn t7_harness_publisher_plugin_cannot_insert_valid() {
    let src = operator_admission_src();
    assert!(
        !contains_ident(&src, "insert_raw"),
        "pas de backdoor insert_raw dans admission.rs"
    );
    assert!(
        !src.contains("pub fn insert"),
        "pas d'insert public qui contourne admit"
    );
    assert!(
        !contains_ident(&src, "VERIFIED"),
        "admission.rs ne doit pas stocker VERIFIED"
    );
    assert!(
        !contains_ident(HARNESS_SRC, "VALID"),
        "harness P0 ne doit pas émettre VALID"
    );
    assert!(
        !contains_ident(HARNESS_SRC, "VERIFIED"),
        "harness P0 ne doit pas émettre VERIFIED"
    );
    assert!(
        !HARNESS_SRC.contains("AdmissionRecord"),
        "harness ne construit pas AdmissionRecord"
    );
    assert!(
        !HARNESS_SRC.contains("InMemoryAdmissionStore"),
        "harness n'est pas le store d'admission"
    );
}

#[test]
fn t7_claimed_verified_with_failed_signature_or_conformance_refuses() {
    let (descriptor, report_ok, _) = passing_report_and_descriptor();
    let fail = evaluate_conformance("not a manifesto");
    let mut store = InMemoryAdmissionStore::new();

    let sig = store.admit(AdmitInput {
        descriptor_digest: descriptor.clone(),
        observed_descriptor: descriptor.clone(),
        policy_id: POLICY_ID.to_owned(),
        report_digest: report_ok,
        conformance_passed: true,
        signature_verified: false,
        claimed_verified: true,
    });
    assert_eq!(sig, Err(AdmitRefuse::UnexpectedSignature));
    assert!(store.get(&descriptor).is_none());

    let conf = store.admit(AdmitInput {
        descriptor_digest: descriptor.clone(),
        observed_descriptor: descriptor.clone(),
        policy_id: POLICY_ID.to_owned(),
        report_digest: fail.report_digest,
        conformance_passed: false,
        signature_verified: true,
        claimed_verified: true,
    });
    assert_eq!(conf, Err(AdmitRefuse::ManifestNonconformant));
    assert!(store.get(&descriptor).is_none());
}

#[test]
fn t7_missing_policy_id_refuses() {
    let (mut store, mut input, descriptor) = passing_input(false);
    input.policy_id.clear();
    let refused = store.admit(input);
    assert_eq!(refused, Err(AdmitRefuse::MissingRuntimePolicy));
    assert!(store.get(&descriptor).is_none());
}

#[test]
fn t7_wrong_policy_id_refuses() {
    let (mut store, mut input, descriptor) = passing_input(true);
    input.policy_id = "community-skip/v0".to_owned();
    let refused = store.admit(input);
    assert_eq!(refused, Err(AdmitRefuse::MissingRuntimePolicy));
    assert!(store.get(&descriptor).is_none());
}

#[test]
fn t7_admit_refuse_variants_stable_for_t8() {
    let variants = [
        AdmitRefuse::TamperedDigest,
        AdmitRefuse::ManifestNonconformant,
        AdmitRefuse::UnexpectedSignature,
        AdmitRefuse::MissingRuntimePolicy,
    ];
    assert_eq!(variants.len(), 4);
    let (descriptor, report_digest, _) = passing_report_and_descriptor();
    assert_eq!(
        AdmitRefuse::if_digest_mismatch(&descriptor, &descriptor),
        None
    );
    assert_eq!(
        AdmitRefuse::if_digest_mismatch(&descriptor, &report_digest),
        Some(AdmitRefuse::TamperedDigest)
    );
}

#[test]
fn t7_manifesto_src_has_no_admission_store() {
    let manifesto = repo_root().join("Manifesto");
    let mut files = Vec::new();
    for crate_name in MANIFESTO_SRC_CRATES {
        let src = manifesto.join(crate_name).join("src");
        assert!(src.is_dir(), "Manifesto/{crate_name}/src doit exister");
        collect_rs(&src, &mut files);
    }
    let mut hits = Vec::new();
    for file in &files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("source manquante {}: {e}", file.display()));
        for needle in ["AdmissionRecord", "InMemoryAdmissionStore"] {
            if content.contains(needle) {
                hits.push(format!("{}: {needle}", file.display()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "tokens admission interdits sous Manifesto/*/src: {hits:?}"
    );
}

#[test]
fn t7_operator_src_has_no_http_listener() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&root, &mut files);
    let mut hits = Vec::new();
    for file in &files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("source manquante {}: {e}", file.display()));
        for needle in HTTP_LISTEN_TOKENS {
            if content.contains(needle) {
                hits.push(format!("{}: {needle}", file.display()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "tokens HTTP interdits dans apparatus-operator/src: {hits:?}"
    );
}
