//! Apparatus P4 — T5 runner de conformance plateforme (pas le harness P0).
//!
//! Rapport versionné `{policy_id, report_digest}` sur artifacts candidats.
//! Une suite qui passe n'est pas une admission et n'est pas installable.
//! Le harness P0 ne construit pas ce rapport.

use std::fs;
use std::path::Path;

use apparatus_operator::conformance::{
    community_policy_id, evaluate_conformance, official_policy_id, report_grants_admission,
    ConformanceReport, POLICY_ID,
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

fn assert_report_digest_format(report: &ConformanceReport) {
    let digest = report.report_digest.as_str();
    assert!(
        digest.starts_with("sha256:"),
        "report_digest doit commencer par sha256: : {digest}"
    );
    let hex = digest
        .strip_prefix("sha256:")
        .expect("préfixe sha256: déjà vérifié");
    assert_eq!(
        hex.len(),
        64,
        "report_digest hex len={}: {digest}",
        hex.len()
    );
    assert!(
        hex.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
        "report_digest hex doit être minuscule : {digest}"
    );
}

#[test]
fn t5_report_fields_on_pass() {
    let report = evaluate_conformance(VALID_MANIFEST_TOML);
    assert_eq!(report.policy_id, POLICY_ID);
    assert_report_digest_format(&report);
    assert!(report.passed, "manifeste P0 valide doit passer la suite");
}

#[test]
fn t5_report_fields_on_fail() {
    let toml = format!("build = \"make\"\n{VALID_MANIFEST_TOML}");
    let report = evaluate_conformance(&toml);
    assert_eq!(report.policy_id, POLICY_ID);
    assert_report_digest_format(&report);
    assert!(!report.passed, "build= libre doit échouer la suite");
}

#[test]
fn t5_policy_id_stable() {
    let first = evaluate_conformance(VALID_MANIFEST_TOML);
    let second = evaluate_conformance(VALID_MANIFEST_TOML);
    assert_eq!(first.policy_id, POLICY_ID);
    assert_eq!(second.policy_id, POLICY_ID);
    assert_eq!(first.policy_id, second.policy_id);
    assert_eq!(first.report_digest, second.report_digest);
}

#[test]
fn t5_mutation_after_fail_changes_digest() {
    let failed = format!("build = \"make\"\n{VALID_MANIFEST_TOML}");
    let mutated = format!("build = \"cmake\"\n{VALID_MANIFEST_TOML}");
    let first = evaluate_conformance(&failed);
    let second = evaluate_conformance(&mutated);
    assert!(!first.passed);
    assert!(!second.passed);
    assert_eq!(first.policy_id, second.policy_id);
    assert_ne!(
        first.report_digest, second.report_digest,
        "muter le candidat après un check raté doit changer report_digest"
    );
}

#[test]
fn t5_passing_conformance_is_not_admission() {
    let pass = evaluate_conformance(VALID_MANIFEST_TOML);
    let fail = evaluate_conformance("not a manifesto");
    assert!(pass.passed);
    assert!(!fail.passed);
    assert!(
        !report_grants_admission(&pass),
        "conformance verte ≠ admission"
    );
    assert!(!report_grants_admission(&fail));
    let src = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("conformance.rs"),
    )
    .expect("src/conformance.rs");
    assert!(
        !contains_ident(&src, "VALID"),
        "conformance.rs ne doit pas porter VALID"
    );
    assert!(
        !contains_ident(&src, "VERIFIED"),
        "conformance.rs ne doit pas porter VERIFIED"
    );
}

#[test]
fn t5_official_and_community_share_policy_id() {
    assert_eq!(official_policy_id(), POLICY_ID);
    assert_eq!(community_policy_id(), POLICY_ID);
    assert_eq!(official_policy_id(), community_policy_id());
    let official = evaluate_conformance(VALID_MANIFEST_TOML);
    let community = evaluate_conformance(VALID_MANIFEST_TOML);
    assert_eq!(official.policy_id, community.policy_id);
    assert_eq!(official.report_digest, community.report_digest);
}

#[test]
fn t5_harness_does_not_emit_conformance_report() {
    assert!(
        !contains_ident(HARNESS_SRC, "VALID"),
        "harness P0 ne doit pas contenir VALID"
    );
    assert!(
        !contains_ident(HARNESS_SRC, "VERIFIED"),
        "harness P0 ne doit pas contenir VERIFIED"
    );
    assert!(
        !HARNESS_SRC.contains("ConformanceReport"),
        "harness P0 ne doit pas construire ConformanceReport"
    );
}

#[test]
fn t5_k8s_dir_still_absent() {
    let k8s = Path::new(env!("CARGO_MANIFEST_DIR")).join("k8s");
    assert!(
        k8s.is_dir(),
        "apparatus-operator/k8s/ doit exister avec les manifests operator (ns, SA, RBAC, CRD, NetworkPolicy)"
    );
    let mut yaml = String::new();
    let entries = fs::read_dir(&k8s).expect("k8s/ lisible");
    let mut yaml_files = 0usize;
    for entry in entries {
        let path = entry.expect("entrée k8s lisible").path();
        let is_yaml = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"));
        if !is_yaml {
            continue;
        }
        yaml_files += 1;
        let content =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(
            !content.contains("kind.x-k8s.io"),
            "config Kind interdite sous k8s/: {}",
            path.display()
        );
        yaml.push_str(&content);
        yaml.push('\n');
    }
    assert!(
        yaml_files > 0,
        "apparatus-operator/k8s/ doit contenir des manifests YAML operator"
    );
    for needle in [
        "apparatus-system",
        "apparatus-plugins",
        "apparatus-build",
        "apparatus-admit",
        "apparatus-controller",
        "apparatus-gateway",
        "AdmissionRecord",
        "NetworkPolicy",
        "automountServiceAccountToken",
        "kind: Namespace",
        "kind: ServiceAccount",
        "kind: CustomResourceDefinition",
    ] {
        assert!(
            yaml.contains(needle),
            "manifests operator k8s/ doivent contenir {needle}"
        );
    }
}
