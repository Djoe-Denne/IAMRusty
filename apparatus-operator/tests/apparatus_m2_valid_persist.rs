//! M2 — `VALID` persisté hors `InMemoryAdmissionStore` et hors CR Kind.
//!
//! Store produit : fichier JSON (`PersistentAdmissionStore`). Identité catalogue
//! = digest descripteur 0002 du manifeste disque `apparatus-reference-kv`
//! (helper M1, sans pin zot). Pas de feature Kind/zot.

use std::fs;
use std::path::{Path, PathBuf};

use apparatus_operator::admission::{
    admission_store_path_from_env, AdmissionStatus, AdmissionStore, AdmitInput, AdmitRefuse,
    PersistentAdmissionStore, ADMISSION_STORE_PATH_ENV,
};
use apparatus_operator::{
    digest_manifest, evaluate_conformance, parse_manifest, ReleaseDigest, POLICY_ID,
};

const REFERENCE_KV_ID: &str = "io.aiforall.reference-kv";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn reference_kv_manifest() -> PathBuf {
    repo_root()
        .join("apparatus-reference-kv")
        .join("apparatus.toml")
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
        "apparatus-m2-{label}-{}-{nanos}.json",
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

#[test]
fn m2_admit_get_returns_valid_for_m1_digest() {
    let tmp = TempStorePath::new("admit");
    let (input, descriptor, report_digest) = passing_input();
    let mut store = PersistentAdmissionStore::open(&tmp.path).expect("open store");
    let record = store.admit(input).expect("admit M1");
    assert_eq!(record.status, AdmissionStatus::Valid);
    assert_eq!(record.policy_version, POLICY_ID);
    assert_eq!(record.report_digest, report_digest);
    assert_eq!(record.descriptor_digest, descriptor);
    assert_ne!(
        report_digest, descriptor,
        "reportDigest n'est pas l'identité catalogue"
    );

    let got = store
        .get(&descriptor)
        .expect("lecture produit PersistentAdmissionStore::get");
    assert_eq!(got.status, AdmissionStatus::Valid);
    assert_eq!(got.policy_version, POLICY_ID);
    assert_eq!(got.report_digest, report_digest);
    assert!(
        store.get(&report_digest).is_none(),
        "get clé sur descriptor_digest, pas report_digest"
    );
}

#[test]
fn m2_reopen_same_file_still_valid() {
    let tmp = TempStorePath::new("reopen");
    let (input, descriptor, report_digest) = passing_input();
    {
        let mut store = PersistentAdmissionStore::open(&tmp.path).expect("open store");
        store.admit(input).expect("admit M1");
    }
    let reopened = PersistentAdmissionStore::open(&tmp.path).expect("ré-open même path");
    let got = reopened
        .get(&descriptor)
        .expect("VALID survit à une nouvelle instance");
    assert_eq!(got.status, AdmissionStatus::Valid);
    assert_eq!(got.policy_version, POLICY_ID);
    assert_eq!(got.report_digest, report_digest);
}

#[test]
fn m2_refuse_writes_no_valid_line() {
    let tmp = TempStorePath::new("refuse");
    let (mut input, descriptor, _) = passing_input();
    input.conformance_passed = false;
    input.signature_verified = false;
    let mut store = PersistentAdmissionStore::open(&tmp.path).expect("open store");
    let refused = store.admit(input);
    assert!(
        matches!(
            refused,
            Err(AdmitRefuse::ManifestNonconformant | AdmitRefuse::UnexpectedSignature)
        ),
        "refus Adm-A attendu, obtenu {refused:?}"
    );
    assert!(
        store.get(&descriptor).is_none(),
        "aucune ligne VALID en mémoire après refus"
    );
    drop(store);
    let reopened = PersistentAdmissionStore::open(&tmp.path).expect("ré-open après refus");
    assert!(
        reopened.get(&descriptor).is_none(),
        "aucune ligne VALID dans le fichier après refus"
    );
}

#[test]
fn m2_claimed_verified_failed_guard_writes_no_valid() {
    let tmp = TempStorePath::new("claimed");
    let (descriptor, report_ok, _) = m1_descriptor_and_report();
    let mut store = PersistentAdmissionStore::open(&tmp.path).expect("open store");

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

    let fail = evaluate_conformance("not a manifesto");
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
    drop(store);

    let reopened = PersistentAdmissionStore::open(&tmp.path).expect("ré-open claimed");
    assert!(
        reopened.get(&descriptor).is_none(),
        "claimed_verified ne doit pas écrire VALID dans le fichier"
    );
}

#[test]
fn m2_claimed_verified_success_still_persists_valid() {
    let tmp = TempStorePath::new("claimed-ok");
    let (mut input, descriptor, report_digest) = passing_input();
    input.claimed_verified = true;
    {
        let mut store = PersistentAdmissionStore::open(&tmp.path).expect("open store");
        let record = store
            .admit(input)
            .expect("admit claimed=true avec gardes OK");
        assert_eq!(record.status, AdmissionStatus::Valid);
        assert_eq!(record.policy_version, POLICY_ID);
        assert_eq!(record.report_digest, report_digest);
    }
    let reopened = PersistentAdmissionStore::open(&tmp.path).expect("ré-open claimed-ok");
    let got = reopened
        .get(&descriptor)
        .expect("claimed=true ne bloque pas un VALID produit");
    assert_eq!(got.status, AdmissionStatus::Valid);
    assert_eq!(got.policy_version, POLICY_ID);
    assert_eq!(got.report_digest, report_digest);
}

#[test]
fn m2_persisted_json_matches_crd_shape() {
    let tmp = TempStorePath::new("crd-shape");
    let (input, descriptor, report_digest) = passing_input();
    {
        let mut store = PersistentAdmissionStore::open(&tmp.path).expect("open store");
        store.admit(input).expect("admit");
    }
    let raw = fs::read_to_string(&tmp.path).expect("lecture brute du JSON produit");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("JSON");
    let rec = &value["records"][0];
    assert_eq!(
        rec["descriptorDigest"].as_str(),
        Some(descriptor.as_str()),
        "descriptorDigest CR"
    );
    assert_eq!(
        rec["policyVersion"].as_str(),
        Some(POLICY_ID),
        "policyVersion CR"
    );
    assert_eq!(
        rec["reportDigest"].as_str(),
        Some(report_digest.as_str()),
        "reportDigest CR"
    );
    assert_eq!(rec["status"]["phase"].as_str(), Some("VALID"));
}

#[test]
fn m2_planted_valid_foreign_policy_is_skipped_on_open() {
    let tmp = TempStorePath::new("plant");
    let (_, descriptor, report_digest) = passing_input();
    let planted = format!(
        r#"{{"records":[{{"descriptorDigest":"{}","policyVersion":"foreign-policy/v0","reportDigest":"{}","status":{{"phase":"VALID"}}}}]}}"#,
        descriptor.as_str(),
        report_digest.as_str()
    );
    fs::write(&tmp.path, planted).expect("planter JSON VALID hors POLICY_ID");
    let store = PersistentAdmissionStore::open(&tmp.path).expect("open du JSON planté");
    assert!(
        store.get(&descriptor).is_none(),
        "phase=VALID + policyVersion ≠ POLICY_ID ne doit pas produire un VALID"
    );
}

#[test]
fn m2_admission_store_path_from_env_fails_when_unset() {
    let current = std::env::var_os(ADMISSION_STORE_PATH_ENV);
    assert!(
        current.as_ref().is_none_or(|value| value.is_empty()),
        "unsafe_code=forbid : ce test exige l'env absente ou vide, pas de set_var"
    );
    let result = admission_store_path_from_env();
    assert!(
        result.is_err(),
        "env absente/vide → fail-closed, obtenu {result:?}"
    );
}
