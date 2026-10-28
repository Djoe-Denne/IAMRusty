//! G-3/G-7 §13.5: offline crypto verification against independently signed JWS.
//! No SQL consumption, Adm-A single-writer or maintenance authority is implied.

use apparatus_contracts::runtime::{
    RuntimeAdmissionChallengeV1, RuntimeErrorCodeV1, RuntimeUnixMsV1,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use manifesto_infra::apparatus_runtime::admission_proof::{
    PinnedRuntimeAdmissionKey, PinnedRuntimeAdmissionProofVerifier,
    RejectRuntimeAdmissionProofVerifier, RuntimeAdmissionProofVerifier,
};
use rsa::{BigUint, RsaPublicKey};
use serde_json::Value;

fn vectors() -> Value {
    serde_json::from_str(include_str!("fixtures/runtime_admission_jws_v1.json"))
        .expect("G-7 independent Node RSA/JCS vectors")
}

fn public_key(jwk: &Value) -> RsaPublicKey {
    let integer = |field: &str| {
        BigUint::from_bytes_be(
            &URL_SAFE_NO_PAD
                .decode(jwk[field].as_str().expect("public JWK integer"))
                .expect("canonical base64url integer"),
        )
    };
    RsaPublicKey::new(integer("n"), integer("e")).expect("RSA test public key")
}

fn pin(fixture: &Value, jwk: &Value) -> PinnedRuntimeAdmissionKey {
    PinnedRuntimeAdmissionKey {
        authority_id: serde_json::from_value(fixture["authority_id"].clone())
            .expect("authority UUID"),
        installation_id: serde_json::from_value(fixture["installation_id"].clone())
            .expect("installation UUID"),
        public_key: public_key(jwk),
    }
}

fn verifier(fixture: &Value) -> PinnedRuntimeAdmissionProofVerifier {
    PinnedRuntimeAdmissionProofVerifier::new(
        vec![pin(fixture, &fixture["public_jwk"])],
        serde_json::from_value(fixture["allowed_repositories"].clone())
            .expect("approved repositories"),
    )
    .expect("G-7 explicitly pinned test authority")
}

fn challenge(case: &Value) -> RuntimeAdmissionChallengeV1 {
    serde_json::from_value(case["expected_challenge"].clone())
        .expect("independent writer challenge")
}

fn now(case: &Value) -> RuntimeUnixMsV1 {
    serde_json::from_value(case["db_now_unix_ms"].clone()).expect("fixed DB test clock")
}

fn assert_negative_cases(select: impl Fn(&str) -> bool) {
    let fixture = vectors();
    let verifier = verifier(&fixture);
    let mut tested = 0;
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().expect("crypto cases") {
        let name = case["name"].as_str().expect("scenario");
        if case["expected"] == "valid" || !select(name) {
            continue;
        }
        tested += 1;
        let expected: RuntimeErrorCodeV1 =
            serde_json::from_value(case["expected"].clone()).expect("contract error");
        let result = verifier.verify(
            &challenge(case),
            case["compact_jws"].as_str().expect("signed fixture"),
            now(case),
        );
        match result {
            Ok(_) => mismatches.push(format!(
                "{name}: expected {expected:?}, got verified capability"
            )),
            Err(actual) if actual != expected => {
                mismatches.push(format!("{name}: expected {expected:?}, got {actual:?}"))
            }
            Err(_) => {}
        }
    }
    assert!(tested > 0, "G-7 selected crypto corpus must be nonempty");
    assert!(
        mismatches.is_empty(),
        "G-7 crypto contract mismatches: {}",
        mismatches.join("; ")
    );
}

#[test]
fn g7_independent_prepare_open_signatures_kid_and_fingerprint_verify() {
    let fixture = vectors();
    let verifier = verifier(&fixture);
    let mut tested = 0;
    for case in fixture["cases"].as_array().expect("crypto cases") {
        if case["expected"] != "valid" {
            continue;
        }
        tested += 1;
        let proof = verifier
            .verify(
                &challenge(case),
                case["compact_jws"].as_str().expect("signed fixture"),
                now(case),
            )
            .unwrap_or_else(|code| panic!("G-7 independent {} rejected: {code:?}", case["name"]));
        assert_eq!(
            proof.kid(),
            fixture["expected_kid"].as_str().expect("RFC7638 kid")
        );
        let fingerprint: String = proof
            .fingerprint()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            fingerprint,
            case["expected_fingerprint_hex"]
                .as_str()
                .expect("independent JWS hash")
        );
        assert_eq!(
            serde_json::to_value(&proof.claims().challenge).expect("challenge wire"),
            case["expected_challenge"]
        );
        assert_eq!(proof.claims().policy_id, "apparatus-p4-conformance/v1");
        assert_eq!(
            proof.claims().report_sha256.as_str(),
            "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        );
    }
    assert_eq!(
        tested, 3,
        "G-7 Prepare, last valid millisecond and distinct Open"
    );
}

#[test]
fn g7_admission_expiry_future_clock_and_prepare_to_open_substitution_refuse() {
    assert_negative_cases(|name| {
        name.starts_with("exact_")
            || name == "future_observation"
            || name == "prepare_proof_cannot_serve_open"
    });
}

#[test]
fn g7_admission_signature_header_and_unknown_kid_refuse_without_key_discovery() {
    assert_negative_cases(|name| name == "signature_bit_flipped" || name.starts_with("header_"));
}

#[test]
fn g7_admission_signed_claim_context_cas_canonicality_and_repository_substitutions_refuse() {
    assert_negative_cases(|name| {
        name.starts_with("claims_")
            || name.starts_with("substituted_")
            || name.starts_with("valid_signature_")
            || name.starts_with("unapproved_")
            || name.starts_with("tagged_")
    });
}

#[test]
fn g7_admission_default_rejects_and_weak_duplicate_or_missing_trust_never_selects_first() {
    let fixture = vectors();
    let case = &fixture["cases"][0];
    assert_eq!(
        RejectRuntimeAdmissionProofVerifier
            .verify(
                &challenge(case),
                case["compact_jws"].as_str().expect("valid proof"),
                now(case)
            )
            .err(),
        Some(RuntimeErrorCodeV1::RuntimeKeyUnavailable)
    );
    for keys in [
        vec![],
        vec![pin(&fixture, &fixture["weak_public_jwk"])],
        vec![
            pin(&fixture, &fixture["public_jwk"]),
            pin(&fixture, &fixture["public_jwk"]),
        ],
    ] {
        assert_eq!(
            PinnedRuntimeAdmissionProofVerifier::new(
                keys,
                vec!["registry.test/apparatus/envelopes".to_owned()]
            )
            .err(),
            Some(RuntimeErrorCodeV1::RuntimeKeyUnavailable)
        );
    }
}
