//! Apparatus P4 — T8 refus ADR-0005 (un protocole, pas de CR VALID, pas de schedule).
//!
//! Quatre gardes : digest altéré, manifeste non conforme, signature inattendue,
//! politique runtime manquante. `claimed_verified` ne contourne aucun refus.
//! Officiel et communautaire empruntent le même `admit()`. Pas d'HTTP, pas de kube.

use std::fs;
use std::path::{Path, PathBuf};

use apparatus_operator::admission::{
    AdmissionRecord, AdmissionStore, AdmitInput, AdmitRefuse, InMemoryAdmissionStore,
};
use apparatus_operator::{
    community_policy_id, digest_manifest, evaluate_conformance, official_policy_id, parse_manifest,
    would_schedule, ReleaseDigest, POLICY_ID,
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

const TRUSTED_NEEDLES: [&str; 4] = [
    "trusted_skip",
    "trusted_path",
    "trusted skip",
    "trusted path",
];

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

fn passing_report_and_descriptor() -> (ReleaseDigest, ReleaseDigest, bool) {
    let report = evaluate_conformance(VALID_MANIFEST_TOML);
    assert!(report.passed, "fixture T5 valide doit passer la suite");
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

fn assert_refused_without_schedule(
    store: &InMemoryAdmissionStore,
    digest: &ReleaseDigest,
    result: &Result<AdmissionRecord, AdmitRefuse>,
    expected: AdmitRefuse,
) {
    assert_eq!(result, &Err(expected));
    assert!(
        store.get(digest).is_none(),
        "refus {expected:?} ne doit pas écrire de CR Valid"
    );
    assert!(
        !would_schedule(store, digest),
        "refus {expected:?} ne doit pas programmer de workload"
    );
}

#[test]
fn t8_tampered_digest_refuses_no_cr_no_schedule() {
    let (mut store, mut input, descriptor) = passing_input(true);
    input.observed_descriptor = input.report_digest.clone();
    let refused = store.admit(input);
    assert_refused_without_schedule(&store, &descriptor, &refused, AdmitRefuse::TamperedDigest);
}

#[test]
fn t8_manifest_nonconformant_refuses_no_cr_no_schedule() {
    let (mut store, mut input, descriptor) = passing_input(true);
    input.conformance_passed = false;
    let refused = store.admit(input);
    assert_refused_without_schedule(
        &store,
        &descriptor,
        &refused,
        AdmitRefuse::ManifestNonconformant,
    );
}

#[test]
fn t8_unexpected_signature_refuses_no_cr_no_schedule() {
    assert_eq!(official_policy_id(), community_policy_id());
    let (mut store, mut input, descriptor) = passing_input(true);
    input.policy_id = community_policy_id().to_owned();
    input.signature_verified = false;
    assert!(input.claimed_verified, "revendication publisher posée");
    let refused = store.admit(input);
    assert_refused_without_schedule(
        &store,
        &descriptor,
        &refused,
        AdmitRefuse::UnexpectedSignature,
    );
}

#[test]
fn t8_missing_runtime_policy_refuses_no_cr_no_schedule() {
    let (mut store, mut input, descriptor) = passing_input(true);
    input.policy_id.clear();
    let refused = store.admit(input);
    assert_refused_without_schedule(
        &store,
        &descriptor,
        &refused,
        AdmitRefuse::MissingRuntimePolicy,
    );
}

#[test]
fn t8_claimed_verified_does_not_bypass_any_refusal() {
    let (descriptor, report_ok, _) = passing_report_and_descriptor();
    let fail = evaluate_conformance("not a manifesto");
    let mut store = InMemoryAdmissionStore::new();

    let cases = [
        (
            AdmitInput {
                descriptor_digest: descriptor.clone(),
                observed_descriptor: report_ok.clone(),
                policy_id: POLICY_ID.to_owned(),
                report_digest: report_ok.clone(),
                conformance_passed: true,
                signature_verified: true,
                claimed_verified: true,
            },
            AdmitRefuse::TamperedDigest,
        ),
        (
            AdmitInput {
                descriptor_digest: descriptor.clone(),
                observed_descriptor: descriptor.clone(),
                policy_id: POLICY_ID.to_owned(),
                report_digest: fail.report_digest,
                conformance_passed: false,
                signature_verified: true,
                claimed_verified: true,
            },
            AdmitRefuse::ManifestNonconformant,
        ),
        (
            AdmitInput {
                descriptor_digest: descriptor.clone(),
                observed_descriptor: descriptor.clone(),
                policy_id: POLICY_ID.to_owned(),
                report_digest: report_ok.clone(),
                conformance_passed: true,
                signature_verified: false,
                claimed_verified: true,
            },
            AdmitRefuse::UnexpectedSignature,
        ),
        (
            AdmitInput {
                descriptor_digest: descriptor.clone(),
                observed_descriptor: descriptor.clone(),
                policy_id: "community-skip/v0".to_owned(),
                report_digest: report_ok,
                conformance_passed: true,
                signature_verified: true,
                claimed_verified: true,
            },
            AdmitRefuse::MissingRuntimePolicy,
        ),
    ];

    for (input, expected) in cases {
        let refused = store.admit(input);
        assert_refused_without_schedule(&store, &descriptor, &refused, expected);
    }
}

#[test]
fn t8_operator_src_has_no_trusted_skip() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&root, &mut files);
    let mut hits = Vec::new();
    for file in &files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("source manquante {}: {e}", file.display()));
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            for needle in TRUSTED_NEEDLES {
                if line.contains(needle) {
                    hits.push(format!("{}:{}: {needle}", file.display(), idx + 1));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "chemin trusted interdit dans apparatus-operator/src: {hits:?}"
    );
}
