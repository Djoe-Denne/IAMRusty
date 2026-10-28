//! G-7 — §13.1 wire profile. Oracles come from the frozen contract, not C's codec.

use apparatus_contracts::runtime::{
    parse_runtime_json, RuntimeBindingSnapshotV1, RuntimeCompleteEffectRequestV1, RuntimeCounterV1,
    RuntimeEffectHashInputV1, RuntimeEnrollmentRequestV1, RuntimeErrorCodeV1,
    RuntimeInvokeRequestV1, RuntimeOperationHashInputV1, RuntimeUnixMsV1,
};
use serde_json::Value;

fn shape_vectors() -> Value {
    serde_json::from_str(include_str!("fixtures/runtime_shapes_v1.json"))
        .expect("G-7 section 13.2 shapes")
}

fn admission_claims() -> Value {
    serde_json::from_str::<Value>(include_str!(
        "fixtures/runtime_admission_attestation_v1.json"
    ))
    .expect("G-7 unsigned admission claim fixture")["claims"]
        .clone()
}

#[test]
fn g7_admission_prepare_and_open_claim_shapes_preserve_full_challenge_without_granting_valid() {
    use apparatus_contracts::runtime::{
        RuntimeAdmissionAttestationV1, RuntimeAdmissionChallengeV1,
    };

    let prepare = admission_claims();
    assert_exact_wire::<RuntimeAdmissionChallengeV1>(&prepare["challenge"]);
    assert_exact_wire::<RuntimeAdmissionAttestationV1>(&prepare);
    let mut open = prepare;
    open["challenge"]["action"] = serde_json::json!("open");
    open["challenge"]["challenge_id"] = serde_json::json!("00000000-0000-4000-8000-000000000035");
    open["jti"] = open["challenge"]["challenge_id"].clone();
    open["challenge"]["service_uid"] = serde_json::json!("admitted-service-open-uid");
    open["challenge"]["revision_uid"] = serde_json::json!("admitted-revision-open-uid");
    assert_exact_wire::<RuntimeAdmissionChallengeV1>(&open["challenge"]);
    assert_exact_wire::<RuntimeAdmissionAttestationV1>(&open);
    // Decoding these unsigned bytes is not a successful Prepare/Open or VALID.
}

#[test]
fn g7_admission_challenge_requires_every_field_including_explicit_null_uids() {
    use apparatus_contracts::runtime::RuntimeAdmissionChallengeV1;

    let challenge = admission_claims()["challenge"].clone();
    for key in challenge.as_object().expect("challenge object").keys() {
        let mut missing = challenge.clone();
        missing
            .as_object_mut()
            .expect("challenge object")
            .remove(key);
        assert!(
            parse_runtime_json::<RuntimeAdmissionChallengeV1>(
                &serde_json::to_vec(&missing).expect("missing challenge field JSON")
            )
            .is_err(),
            "G-7 challenge field omitted instead of explicit value/null: {key}"
        );
    }
}

#[test]
fn g7_admission_claims_require_every_binding_and_reject_runner_assertion_flags() {
    use apparatus_contracts::runtime::RuntimeAdmissionAttestationV1;

    let claims = admission_claims();
    for key in claims.as_object().expect("admission claims object").keys() {
        let mut missing = claims.clone();
        missing.as_object_mut().expect("claims object").remove(key);
        assert_rejected_wire::<RuntimeAdmissionAttestationV1>(&missing);
    }
    for flag in ["valid", "signature_verified", "d_ack"] {
        let mut extra = claims.clone();
        extra[flag] = Value::Bool(true);
        assert_rejected_wire::<RuntimeAdmissionAttestationV1>(&extra);
    }
    let mut unknown_nested = claims;
    unknown_nested["challenge"]["force"] = Value::Bool(true);
    assert_rejected_wire::<RuntimeAdmissionAttestationV1>(&unknown_nested);
}

#[test]
fn g7_admission_challenge_action_and_schema_are_closed() {
    use apparatus_contracts::runtime::{
        RuntimeAdmissionAttestationV1, RuntimeAdmissionChallengeV1,
    };

    let challenge = admission_claims()["challenge"].clone();
    for action in ["activate", "fence", "Prepare", "OPEN", "valid"] {
        let mut changed = challenge.clone();
        changed["action"] = Value::String(action.to_owned());
        assert_rejected_wire::<RuntimeAdmissionChallengeV1>(&changed);
    }
    let mut version = challenge;
    version["schema_version"] = serde_json::json!(2);
    assert_rejected_wire::<RuntimeAdmissionChallengeV1>(&version);
    let mut claims = admission_claims();
    claims["schema_version"] = serde_json::json!(2);
    assert_rejected_wire::<RuntimeAdmissionAttestationV1>(&claims);
}

#[test]
fn g7_admission_duplicate_challenge_and_policy_fields_are_rejected_before_value() {
    use apparatus_contracts::runtime::RuntimeAdmissionAttestationV1;

    let bytes = serde_json::to_string(&admission_claims()).expect("admission JSON");
    for (original, duplicated) in [
        (
            "\"policy_id\":\"apparatus-p4-conformance/v1\"",
            "\"policy_id\":\"apparatus-p4-conformance/v1\",\"policy_id\":\"untrusted-policy\"",
        ),
        (
            "\"challenge_id\":\"00000000-0000-4000-8000-000000000031\"",
            "\"challenge_id\":\"00000000-0000-4000-8000-000000000031\",\"challenge_id\":\"00000000-0000-4000-8000-000000000035\"",
        ),
    ] {
        assert!(bytes.contains(original), "G-7 raw fixture replacement must apply");
        let duplicated_json = bytes.replacen(original, duplicated, 1);
        assert!(
            parse_runtime_json::<RuntimeAdmissionAttestationV1>(duplicated_json.as_bytes()).is_err(),
            "G-7 duplicate signed-claim field accepted: {original}"
        );
    }
}

fn assert_exact_wire<T: serde::de::DeserializeOwned + serde::Serialize>(input: &Value) {
    let parsed: T = parse_runtime_json(&serde_json::to_vec(input).expect("wire JSON"))
        .expect("G-7 contract shape accepted");
    assert_eq!(serde_json::to_value(parsed).expect("wire JSON"), *input);
}

fn assert_rejected_wire<T: serde::de::DeserializeOwned>(input: &Value) {
    assert!(
        parse_runtime_json::<T>(&serde_json::to_vec(input).expect("negative wire JSON")).is_err(),
        "G-7 obsolete/ambiguous wire shape accepted: {input}"
    );
}

#[test]
fn g7_all_error_codes_keep_frozen_status_retry_role_and_closed_envelope() {
    use apparatus_contracts::runtime::{RuntimeEffectOutcomeV1, RuntimeServiceRoleV1};

    let fixture: Value = serde_json::from_str(include_str!("fixtures/runtime_errors_v1.json"))
        .expect("G-5/G-7 independent error mapping");
    let rows = fixture["rows"].as_array().expect("31 error mappings");
    assert_eq!(rows.len(), 31, "G-7 frozen error inventory");
    for row in rows {
        let code: RuntimeErrorCodeV1 = serde_json::from_value(row[0].clone()).expect("wire code");
        assert_eq!(
            u64::from(code.http_status()),
            row[1].as_u64().expect("HTTP status"),
            "G-7 {}",
            row[0]
        );
        for (role, retry_column) in [
            (RuntimeServiceRoleV1::Projector, 2),
            (RuntimeServiceRoleV1::Lazaret, 3),
        ] {
            let (outcome, outcome_wire) = if code == RuntimeErrorCodeV1::RuntimeEffectOutcomeUnknown
            {
                (RuntimeEffectOutcomeV1::MayHaveStarted, "may_have_started")
            } else {
                (RuntimeEffectOutcomeV1::NotStarted, "not_started")
            };
            let envelope = code.error(role, outcome);
            assert_eq!(
                serde_json::to_value(envelope).expect("error envelope"),
                serde_json::json!({
                    "schema_version": 1,
                    "code": row[0],
                    "retry_class": row[retry_column],
                    "effect_outcome": outcome_wire
                }),
                "G-7 error mapping for {} role={role:?}",
                row[0]
            );
        }
    }
}

#[test]
fn g7_authority_error_rejects_unknown_code_version_retry_and_free_details() {
    use apparatus_contracts::runtime::RuntimeControlErrorV1;

    let input = serde_json::json!({
        "schema_version": 1, "code": "RUNTIME_EFFECT_OUTCOME_UNKNOWN",
        "retry_class": "never", "effect_outcome": "may_have_started"
    });
    assert_exact_wire::<RuntimeControlErrorV1>(&input);
    for (field, replacement) in [
        ("schema_version", serde_json::json!(2)),
        ("code", serde_json::json!("RUNTIME_ALLOW_AFTER_RETRY")),
        ("retry_class", serde_json::json!("retry_operation")),
        ("effect_outcome", serde_json::json!("definitely_cancelled")),
        (
            "detail",
            serde_json::json!("http://private-authority.invalid/secret"),
        ),
    ] {
        let mut changed = input.clone();
        changed[field] = replacement;
        assert_rejected_wire::<RuntimeControlErrorV1>(&changed);
    }
}

#[test]
fn g7_dispatch_requires_target_binding_id_without_legacy_alias() {
    let input = shape_vectors()["invoke"].clone();
    assert_exact_wire::<RuntimeInvokeRequestV1>(&input);
    let mut both = input.clone();
    both["binding_id"] = input["target_binding_id"].clone();
    assert_rejected_wire::<RuntimeInvokeRequestV1>(&both);
    let mut legacy = both;
    legacy
        .as_object_mut()
        .expect("invoke object")
        .remove("target_binding_id");
    assert_rejected_wire::<RuntimeInvokeRequestV1>(&legacy);
    let mut missing = input;
    missing
        .as_object_mut()
        .expect("invoke object")
        .remove("target_binding_id");
    assert_rejected_wire::<RuntimeInvokeRequestV1>(&missing);
}

#[test]
fn g7_enrollment_cannot_supply_instance_id_and_requires_pod_uid() {
    let input = shape_vectors()["enrollment"].clone();
    // These are lexical fixtures, not valid CSR/token or enrollment proof.
    assert_exact_wire::<RuntimeEnrollmentRequestV1>(&input);
    let mut chosen_identity = input.clone();
    chosen_identity["instance_id"] = serde_json::json!("00000000-0000-4000-8000-000000000012");
    assert_rejected_wire::<RuntimeEnrollmentRequestV1>(&chosen_identity);
    chosen_identity["instance_id"] = Value::Null;
    assert_rejected_wire::<RuntimeEnrollmentRequestV1>(&chosen_identity);
    let mut missing = input;
    missing
        .as_object_mut()
        .expect("enrollment object")
        .remove("pod_uid");
    assert_rejected_wire::<RuntimeEnrollmentRequestV1>(&missing);
}

#[test]
fn g7_completion_states_are_not_error_outcomes_or_free_form_results() {
    let mut input = shape_vectors()["complete"].clone();
    for state in ["completed", "indeterminate"] {
        input["completion_state"] = Value::String(state.to_owned());
        assert_exact_wire::<RuntimeCompleteEffectRequestV1>(&input);
    }
    for state in ["not_started", "may_have_started", "success", "queued"] {
        input["completion_state"] = Value::String(state.to_owned());
        assert_rejected_wire::<RuntimeCompleteEffectRequestV1>(&input);
    }
    input
        .as_object_mut()
        .expect("complete object")
        .remove("completion_state");
    input["effect_outcome"] = Value::String("may_have_started".to_owned());
    assert_rejected_wire::<RuntimeCompleteEffectRequestV1>(&input);
}

#[test]
fn g7_binding_wire_distinguishes_unread_registry_from_empty_registry_and_omission() {
    let empty = shape_vectors()["binding_empty_instances"].clone();
    assert_exact_wire::<RuntimeBindingSnapshotV1>(&empty);
    let mut unread = empty.clone();
    unread["instances"] = Value::Null;
    // null is legal for list_bindings only. GET-context validation is a separate
    // adapter/HTTP oracle; DTO decoding must not equate it with items=[].
    assert_exact_wire::<RuntimeBindingSnapshotV1>(&unread);
    let mut omitted = empty;
    omitted
        .as_object_mut()
        .expect("snapshot object")
        .remove("instances");
    assert_rejected_wire::<RuntimeBindingSnapshotV1>(&omitted);
}

#[test]
fn g7_binding_wire_preserves_two_replicas_and_legacy_fence_without_fabricating_pod() {
    let vectors = shape_vectors();
    let mut binding = vectors["binding_empty_instances"].clone();
    let first = vectors["current_pod_instance"].clone();
    let mut second = first.clone();
    second["instance_id"] = serde_json::json!("00000000-0000-4000-8000-000000000013");
    second["pod"]["pod_uid"] = serde_json::json!("runtime-pod-beta");
    second["cas"]["projection_version"] = serde_json::json!("13");
    second["observed_at_unix_ms"] = serde_json::json!("1700000001000");
    second["valid_until_unix_ms"] = serde_json::json!("1700000006000");
    binding["instances"]["items"] = serde_json::json!([first, second, vectors["legacy_tombstone"]]);
    // A sibling refresh changes its row projection_version, not the first row's
    // identity/history. This checks wire preservation only, never allow/freshness.
    assert_exact_wire::<RuntimeBindingSnapshotV1>(&binding);
}

fn lexical_vectors() -> Value {
    serde_json::from_str(include_str!("fixtures/runtime_lexical_v1.json"))
        .expect("G-7 lexical fixture JSON")
}

fn operation_input() -> Value {
    let vector: Value =
        serde_json::from_str(include_str!("fixtures/runtime_operation_hash_v1.json"))
            .expect("G-7 operation fixture JSON");
    vector["input"].clone()
}

fn operation_hash(input: &Value) -> String {
    let parsed: RuntimeOperationHashInputV1 =
        parse_runtime_json(&serde_json::to_vec(input).expect("operation JSON"))
            .expect("G-7 valid operation preimage");
    parsed.sha256().expect("operation hash").as_str().to_owned()
}

#[test]
fn g7_operation_hash_matches_independent_contract_golden_vector() {
    let vector: Value =
        serde_json::from_str(include_str!("fixtures/runtime_operation_hash_v1.json"))
            .expect("G-7 golden vector");
    assert_eq!(operation_hash(&vector["input"]), vector["expected_sha256"]);
}

#[test]
fn g7_hash_uses_utf16_key_order_not_utf8_byte_order() {
    // RFC 8785 sorts the supplementary key's surrogate pair before U+E000.
    // This digest was computed independently with ECMAScript key sorting and
    // Node SHA-256, not copied from C's serializer/hash implementation.
    let mut input = operation_input();
    input["payload"] = serde_json::json!({"\u{1f600}": "supplementary", "\u{e000}": "bmp-private"});
    assert_eq!(
        operation_hash(&input),
        "sha256:dce3be4fbfb28a740a6777df0013d078b2d7a1638949e963ff3a70dc889b4d90"
    );
}

#[test]
fn g7_mutating_each_authority_or_operation_field_changes_hash() {
    let input = operation_input();
    let baseline = operation_hash(&input);
    for (pointer, replacement) in [
        ("/authorization_id", "00000000-0000-4000-8000-000000000009"),
        ("/project_id", "00000000-0000-4000-8000-000000000009"),
        ("/source/instance", "00000000-0000-4000-8000-000000000009"),
        ("/source/binding", "00000000-0000-4000-8000-000000000009"),
        (
            "/source/release",
            "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        ),
        ("/source/generation", "2"),
        ("/source/grant_revision", "9007199254740994"),
        ("/source_revocation_epoch", "3"),
        ("/target/binding_id", "00000000-0000-4000-8000-000000000009"),
        ("/target/generation", "4"),
        (
            "/target/release",
            "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        ),
        ("/target/grant_revision", "5"),
        ("/target/revocation_epoch", "6"),
        ("/operation_id", "00000000-0000-4000-8000-000000000009"),
        ("/operation", "kv.put"),
        ("/payload/key", "another-binding-state"),
        ("/deadline_unix_ms", "1700000030001"),
    ] {
        let mut changed = input.clone();
        *changed
            .pointer_mut(pointer)
            .expect("G-7 covered field exists") = Value::String(replacement.to_owned());
        assert_ne!(
            operation_hash(&changed),
            baseline,
            "G-7 field omitted from hash: {pointer}"
        );
    }
}

#[test]
fn g7_operation_hash_rejects_effect_domain_substitution() {
    let mut input = operation_input();
    input["domain"] = Value::String("urn:aiforall:apparatus:runtime:effect-hash:v1".to_owned());
    // Rejection may happen at decoding or hashing; the contract does not require
    // constructing an invalid-domain DTO before rejecting it.
    let result = parse_runtime_json::<RuntimeOperationHashInputV1>(
        &serde_json::to_vec(&input).expect("operation JSON"),
    )
    .and_then(|parsed| parsed.sha256());
    assert!(matches!(
        result,
        Err(RuntimeErrorCodeV1::RuntimeInvalidRequest)
    ));
}

#[test]
fn g7_effect_hash_matches_independent_vector_and_binds_execution_instance_and_invoke() {
    let vector: Value = serde_json::from_str(include_str!("fixtures/runtime_effect_hash_v1.json"))
        .expect("G-7 effect vector");
    let input = &vector["input"];
    let effect: RuntimeEffectHashInputV1 =
        parse_runtime_json(&serde_json::to_vec(input).expect("effect input JSON"))
            .expect("valid effect preimage");
    let baseline = effect.sha256().expect("effect hash");
    assert_eq!(
        baseline.as_str(),
        vector["expected_sha256"].as_str().expect("golden digest")
    );
    for (pointer, replacement) in [
        (
            "/operation_sha256",
            "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        ),
        ("/execution_id", "00000000-0000-4000-8000-000000000009"),
        ("/effect_id", "00000000-0000-4000-8000-000000000009"),
        ("/instance_id", "00000000-0000-4000-8000-000000000009"),
        ("/invoke/binding_id", "00000000-0000-4000-8000-000000000009"),
        (
            "/invoke/operation_id",
            "00000000-0000-4000-8000-000000000009",
        ),
        ("/invoke/operation", "kv.get"),
        ("/invoke/params/key", "another-binding-state"),
        ("/invoke/params/value", "substituted"),
    ] {
        let mut changed = input.clone();
        *changed
            .pointer_mut(pointer)
            .expect("G-7 effect field exists") = Value::String(replacement.to_owned());
        let parsed: RuntimeEffectHashInputV1 =
            parse_runtime_json(&serde_json::to_vec(&changed).expect("changed effect JSON"))
                .expect("valid changed effect preimage");
        assert_ne!(
            parsed.sha256().expect("changed hash"),
            baseline,
            "G-7 effect field omitted: {pointer}"
        );
    }
}

#[test]
fn g7_counter_wire_is_exact_decimal_string_including_above_binary64_precision() {
    let vectors = lexical_vectors();
    for value in vectors["valid_counters"]
        .as_array()
        .expect("valid counters")
    {
        let counter: RuntimeCounterV1 =
            serde_json::from_value(value.clone()).expect("G-7 valid counter");
        assert_eq!(serde_json::to_value(counter).expect("counter JSON"), *value);
        assert_eq!(counter.get().to_string(), value.as_str().expect("decimal"));
    }
    for value in vectors["invalid_counters"]
        .as_array()
        .expect("invalid counters")
    {
        assert!(
            serde_json::from_value::<RuntimeCounterV1>(value.clone()).is_err(),
            "G-7 noncanonical counter accepted: {value}"
        );
    }
}

#[test]
fn g7_counter_exhaustion_never_wraps_or_resets() {
    let max = RuntimeCounterV1::new(9_223_372_036_854_775_807).expect("maximum counter");
    assert_eq!(
        max.checked_next(),
        Err(RuntimeErrorCodeV1::RuntimeCounterExhausted)
    );
    assert_eq!(
        RuntimeCounterV1::new(9_223_372_036_854_775_808),
        Err(RuntimeErrorCodeV1::RuntimeCounterExhausted)
    );
    assert_eq!(
        RuntimeCounterV1::new(0)
            .expect("zero")
            .checked_next()
            .expect("one")
            .get(),
        1
    );
}

#[test]
fn g7_control_time_is_nonnegative_canonical_decimal_string() {
    let vectors = lexical_vectors();
    for value in vectors["valid_unix_ms"].as_array().expect("valid dates") {
        let time: RuntimeUnixMsV1 = serde_json::from_value(value.clone()).expect("valid time");
        assert_eq!(serde_json::to_value(time).expect("time JSON"), *value);
    }
    for value in vectors["invalid_unix_ms"]
        .as_array()
        .expect("invalid dates")
    {
        assert!(
            serde_json::from_value::<RuntimeUnixMsV1>(value.clone()).is_err(),
            "G-7 noncanonical control time accepted: {value}"
        );
    }
    assert_eq!(
        RuntimeUnixMsV1::new(-1),
        Err(RuntimeErrorCodeV1::RuntimeInvalidRequest)
    );
}

#[test]
fn g7_operation_envelope_preserves_every_contract_field_and_large_counter() {
    let input = operation_input();
    let bytes = serde_json::to_vec(&input).expect("operation input JSON");
    let parsed: RuntimeOperationHashInputV1 = parse_runtime_json(&bytes).expect("valid operation");
    assert_eq!(serde_json::to_value(parsed).expect("operation JSON"), input);
}

#[test]
fn g7_operation_envelope_rejects_missing_and_unknown_fields() {
    let input = operation_input();
    for field in input.as_object().expect("operation object").keys() {
        let mut missing = input.clone();
        missing
            .as_object_mut()
            .expect("operation object")
            .remove(field);
        assert!(
            parse_runtime_json::<RuntimeOperationHashInputV1>(
                &serde_json::to_vec(&missing).expect("missing field JSON")
            )
            .is_err(),
            "G-7 required operation field accepted absent: {field}"
        );
    }
    let mut extra = input;
    extra["trusted_skip_gateway"] = Value::Bool(true);
    assert!(
        parse_runtime_json::<RuntimeOperationHashInputV1>(
            &serde_json::to_vec(&extra).expect("unknown field JSON")
        )
        .is_err(),
        "G-7 unknown authority bypass field accepted"
    );
}

#[test]
fn g7_raw_payload_duplicates_unsafe_numbers_and_surrogates_are_rejected_before_value() {
    let input = serde_json::to_string(&operation_input()).expect("operation JSON");
    let original_payload = r#"{"key":"runtime-cold-state"}"#;
    assert!(
        input.contains(original_payload),
        "G-7 raw replacement must target payload"
    );
    for raw in lexical_vectors()["raw_payloads_rejected_before_value"]
        .as_array()
        .expect("raw cases")
    {
        let bytes = input
            .replace(original_payload, raw.as_str().expect("raw JSON"))
            .into_bytes();
        assert!(
            parse_runtime_json::<RuntimeOperationHashInputV1>(&bytes).is_err(),
            "G-7 forbidden raw payload accepted: {raw}"
        );
    }
}

#[test]
fn g7_unknown_operation_schema_version_has_protocol_error_not_allow() {
    let mut input = operation_input();
    input["schema_version"] = serde_json::json!(2);
    assert!(matches!(
        parse_runtime_json::<RuntimeOperationHashInputV1>(
            &serde_json::to_vec(&input).expect("v2 JSON")
        ),
        Err(RuntimeErrorCodeV1::RuntimeUnsupportedVersion)
    ));
}
