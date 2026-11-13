//! QA lot B / §13.5: real offline verification, not TLS or SQL authority.
//! Independent Node RSA/JCS vectors are shared with the service regression suite.

use super::{
    PinnedRuntimeAdmissionKey, PinnedRuntimeAdmissionProofVerifier, RuntimeAdmissionProofVerifier,
    VerifiedRuntimeAdmissionV1,
};
use apparatus_contracts::runtime::{
    RuntimeAdmissionChallengeV1, RuntimeErrorCodeV1, RuntimeUnixMsV1, MAX_ADMISSION_PROOF_BYTES,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rsa::{traits::PublicKeyParts, BigUint, RsaPublicKey};
use serde_json::Value;
use std::{
    future::Future as _,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Wake, Waker},
};
use uuid::Uuid;

fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/runtime_admission_jws_v1.json"
    ))
    .expect("independent Node RSA/JCS corpus")
}

fn pin(fixture: &Value) -> PinnedRuntimeAdmissionKey {
    let integer = |field: &str| {
        BigUint::from_bytes_be(
            &URL_SAFE_NO_PAD
                .decode(fixture["public_jwk"][field].as_str().expect("JWK integer"))
                .expect("public base64url integer"),
        )
    };
    PinnedRuntimeAdmissionKey {
        authority_id: serde_json::from_value(fixture["authority_id"].clone())
            .expect("authority UUID"),
        installation_id: serde_json::from_value(fixture["installation_id"].clone())
            .expect("installation UUID"),
        public_key: RsaPublicKey::new(integer("n"), integer("e")).expect("public RSA key"),
    }
}

fn repositories(fixture: &Value) -> Vec<String> {
    serde_json::from_value(fixture["allowed_repositories"].clone()).expect("approved repositories")
}

fn verifier(fixture: &Value) -> PinnedRuntimeAdmissionProofVerifier {
    PinnedRuntimeAdmissionProofVerifier::new(vec![pin(fixture)], repositories(fixture))
        .expect("explicit pinned public trust configuration")
}

fn prepare_case(fixture: &Value) -> &Value {
    fixture["cases"]
        .as_array()
        .expect("crypto cases")
        .iter()
        .find(|case| case["name"] == "prepare_independent_rs256")
        .expect("independently signed Prepare witness")
}

fn challenge(case: &Value) -> RuntimeAdmissionChallengeV1 {
    serde_json::from_value(case["expected_challenge"].clone()).expect("writer challenge")
}

fn now(case: &Value) -> RuntimeUnixMsV1 {
    serde_json::from_value(case["db_now_unix_ms"].clone()).expect("fixed verification clock")
}

fn compact(case: &Value) -> &str {
    case["compact_jws"]
        .as_str()
        .expect("independent signed JWS")
}

fn verified(
    verifier: &PinnedRuntimeAdmissionProofVerifier,
    case: &Value,
) -> VerifiedRuntimeAdmissionV1 {
    let proof = verifier
        .verify(&challenge(case), compact(case), now(case))
        .expect("real RS256 verification must accept the independent witness");
    assert_eq!(
        serde_json::to_value(&proof.claims().challenge).expect("verified challenge wire value"),
        case["expected_challenge"]
    );
    proof
}

fn revoke_uncontended(verifier: &PinnedRuntimeAdmissionProofVerifier, kid: &str) {
    let mut revocation = Box::pin(verifier.revoke_key(kid));
    assert!(
        revocation
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_ready(),
        "key withdrawal with no verified capability held must finish"
    );
}

// QA B: withdrawal must affect the specified key, not disable all configured trust.
#[test]
fn revocation_closes_the_real_signing_key_but_not_an_unrelated_kid() {
    let fixture = vectors();
    let case = prepare_case(&fixture);
    let verifier = verifier(&fixture);
    let kid = fixture["expected_kid"]
        .as_str()
        .expect("independent RFC7638 kid");
    let proof = verified(&verifier, case);
    assert_eq!(proof.kid(), kid);
    drop(proof);

    let other_kid = format!("ara-v1-{}", URL_SAFE_NO_PAD.encode([0_u8; 32]));
    assert_ne!(other_kid, kid);
    revoke_uncontended(&verifier, &other_kid);
    drop(verified(&verifier, case));

    for _ in 0..2 {
        revoke_uncontended(&verifier, kid);
        assert_eq!(
            verifier
                .verify(&challenge(case), compact(case), now(case))
                .err(),
            Some(RuntimeErrorCodeV1::RuntimeKeyUnavailable),
            "withdrawn key must not verify again, including repeated withdrawal"
        );
    }
}

#[derive(Default)]
struct RevocationWake(AtomicBool);

impl Wake for RevocationWake {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

// QA B: poll the actual async withdrawal; no sleep, scheduler race or fake lock.
#[test]
fn withdrawal_waits_for_every_real_verified_capability_and_wakes_after_last_drop() {
    let fixture = vectors();
    let case = prepare_case(&fixture);
    let verifier = verifier(&fixture);
    let first = verified(&verifier, case);
    let second = verified(&verifier, case);
    let kid = first.kid().to_owned();
    let mut revocation = Box::pin(verifier.revoke_key(&kid));
    let wake = Arc::new(RevocationWake::default());
    let waker = Waker::from(Arc::clone(&wake));
    let mut context = Context::from_waker(&waker);

    assert!(revocation.as_mut().poll(&mut context).is_pending());
    drop(first);
    assert!(
        revocation.as_mut().poll(&mut context).is_pending(),
        "one remaining verified capability must still serialize withdrawal"
    );
    wake.0.store(false, Ordering::SeqCst);
    drop(second);
    assert!(
        wake.0.load(Ordering::SeqCst),
        "last capability release must wake the real withdrawal future"
    );
    assert!(revocation.as_mut().poll(&mut context).is_ready());
    assert_eq!(
        verifier
            .verify(&challenge(case), compact(case), now(case))
            .err(),
        Some(RuntimeErrorCodeV1::RuntimeKeyUnavailable)
    );
}

// QA B / §13.5-A: invalid allowlist entries must close the whole configuration.
#[test]
fn malformed_repository_configuration_never_survives_beside_valid_repositories() {
    let fixture = vectors();
    for repository in [
        "",
        "registry.test",
        "Registry.test/apparatus/envelopes",
        "registry.test/apparatus/envelopes@sha256:abcd",
        "registry.test/apparatus/*",
        "registry.test/apparatus//envelopes",
        "registry.test/apparatus/envelopes/",
        "registry.test/apparatus:5000/envelopes",
        "registry.test/apparatus/envelopes:latest",
        "registry.test/apparatus/é",
    ] {
        let mut allowed = repositories(&fixture);
        allowed.push(repository.to_owned());
        assert_eq!(
            PinnedRuntimeAdmissionProofVerifier::new(vec![pin(&fixture)], allowed).err(),
            Some(RuntimeErrorCodeV1::RuntimeKeyUnavailable),
            "invalid repository {repository:?} must not be silently ignored"
        );
    }
    assert_eq!(
        PinnedRuntimeAdmissionProofVerifier::new(vec![pin(&fixture)], vec![]).err(),
        Some(RuntimeErrorCodeV1::RuntimeKeyUnavailable)
    );
}

// QA B / §13.5-B: trust entries require both UUIDs and the mandated RSA exponent.
#[test]
fn nil_authority_nil_installation_and_wrong_rsa_exponent_close_configuration() {
    let fixture = vectors();
    for fault in ["nil authority", "nil installation", "exponent three"] {
        let mut key = pin(&fixture);
        match fault {
            "nil authority" => key.authority_id = Uuid::nil(),
            "nil installation" => key.installation_id = Uuid::nil(),
            _ => {
                key.public_key = RsaPublicKey::new(key.public_key.n().clone(), BigUint::from(3_u8))
                    .expect("structurally valid RSA public key with non-profile exponent");
            }
        }
        assert_eq!(
            PinnedRuntimeAdmissionProofVerifier::new(vec![key], repositories(&fixture)).err(),
            Some(RuntimeErrorCodeV1::RuntimeKeyUnavailable),
            "{fault} is not an approved trust entry"
        );
    }
}

// QA B / §13.5-A: both references must match exact repositories, not prefixes.
#[test]
fn real_signed_proof_requires_both_exact_repositories_in_the_pin_configuration() {
    let fixture = vectors();
    let case = prepare_case(&fixture);
    drop(verified(&verifier(&fixture), case));
    for allowed in [
        vec!["registry.test/apparatus/envelopes"],
        vec!["registry.test/apparatus/reference-kv"],
        vec!["registry.test/apparatus"],
        vec![
            "registry.test/apparatus/envelopes/child",
            "registry.test/apparatus/reference-kv/child",
        ],
    ] {
        let verifier = PinnedRuntimeAdmissionProofVerifier::new(
            vec![pin(&fixture)],
            allowed.iter().map(ToString::to_string).collect(),
        )
        .expect("well-formed but insufficient repository configuration");
        assert_eq!(
            verifier
                .verify(&challenge(case), compact(case), now(case))
                .err(),
            Some(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted),
            "both independently signed references must be approved: {allowed:?}"
        );
    }
}

// QA B / §13.5 closed JWS: structural/encoding rejection complements signed cases.
#[test]
fn padded_or_non_url_segments_and_extra_parts_are_invalid() {
    let fixture = vectors();
    let case = prepare_case(&fixture);
    let verifier = verifier(&fixture);
    drop(verified(&verifier, case));
    let parts: Vec<_> = compact(case).split('.').collect();
    assert_eq!(parts.len(), 3, "independent compact JWS has three segments");
    let mut altered = vec![
        ("extra segment", format!("{}.AA", compact(case))),
        ("empty payload", format!("{}..{}", parts[0], parts[2])),
        ("non-ASCII", format!("é.{}.{}", parts[1], parts[2])),
    ];
    for index in 0..3 {
        let mut padded = parts.clone();
        let segment = format!("{}=", parts[index]);
        padded[index] = &segment;
        altered.push(("padded segment", padded.join(".")));
        let mut non_url = parts.clone();
        non_url[index] = "+";
        altered.push(("non-base64url segment", non_url.join(".")));
    }
    for (scenario, proof) in altered {
        assert_eq!(
            verifier.verify(&challenge(case), &proof, now(case)).err(),
            Some(RuntimeErrorCodeV1::RuntimeInvalidProof),
            "closed JWS encoding must reject {scenario}"
        );
    }
}

// QA B / §13.5-A: an unavailable key must not mask an oversized proof.
// These untrusted frames are negative probes, never authenticated witnesses.
#[test]
fn proof_byte_limit_precedes_unavailable_key_lookup_at_the_boundary() {
    let fixture = vectors();
    let case = prepare_case(&fixture);
    let verifier = verifier(&fixture);
    let parts: Vec<_> = compact(case).split('.').collect();
    let mut header: Value = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(parts[0])
            .expect("independent header encoding"),
    )
    .expect("independent protected header");
    header["kid"] = format!("ara-v1-{}", URL_SAFE_NO_PAD.encode([0_u8; 32])).into();
    let encoded_header =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).expect("negative probe header"));
    let overhead = encoded_header.len() + parts[2].len() + 2;
    for (length, expected) in [
        (
            MAX_ADMISSION_PROOF_BYTES,
            RuntimeErrorCodeV1::RuntimeKeyUnavailable,
        ),
        (
            MAX_ADMISSION_PROOF_BYTES + 1,
            RuntimeErrorCodeV1::RuntimeInvalidProof,
        ),
    ] {
        let probe = format!(
            "{encoded_header}.{}.{}",
            "A".repeat(length - overhead),
            parts[2]
        );
        assert_eq!(probe.len(), length, "exact byte boundary probe");
        assert_eq!(
            verifier.verify(&challenge(case), &probe, now(case)).err(),
            Some(expected),
            "untrusted {length}-byte frame must respect the 16 KiB gate"
        );
    }
}
