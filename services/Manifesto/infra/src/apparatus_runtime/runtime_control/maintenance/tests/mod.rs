//! Pure unit rules and configuration guards only (QA 20261113, lot A).
//! Oracles: apparatus-knative-runtime §13.1-A and §13.3-B/C; maintenance
//! action literals follow the closed `RuntimeAdmissionActionV1` contract.
//! Constructed facts are DATA, not authenticated D/operator provenance.
//! No TLS/channel authentication, SQL persistence or transaction is proven here.
//! Kept under a test-only directory, outside the production coverage denominator.

use super::{
    action_string, check_ack_time, plan_json, require_command, RuntimeMaintenanceAct,
    RuntimeMaintenanceFacts, RuntimeMaintenanceProfile, SqlRuntimeMaintenance,
};
use apparatus_contracts::runtime::{
    RuntimeAdmissionActionV1, RuntimeBindingCutoverPlanV1, RuntimeControlErrorV1, RuntimeCounterV1,
    RuntimeDigestV1, RuntimeErrorCodeV1, RuntimeLegacyFenceInstallationAckV1, RuntimeUnixMsV1,
};
use sea_orm::DatabaseConnection;
use serde_json::Value;
use uuid::Uuid;

fn digest(hex_digit: char) -> RuntimeDigestV1 {
    RuntimeDigestV1::new(format!("sha256:{}", hex_digit.to_string().repeat(64)))
        .expect("fixture uses a lowercase SHA-256 digest")
}

fn cutover_plan() -> RuntimeBindingCutoverPlanV1 {
    RuntimeBindingCutoverPlanV1 {
        schema_version: 1,
        migration_id: Uuid::from_u128(0x11111111_1111_4111_8111_111111111111),
        project_id: Uuid::from_u128(0x22222222_2222_4222_8222_222222222222),
        binding_id: Uuid::from_u128(0x33333333_3333_4333_8333_333333333333),
        expected_generation: RuntimeCounterV1::new(9_007_199_254_740_993)
            .expect("counter beyond JS exact integer range remains valid"),
        expected_grant_revision: RuntimeCounterV1::new(17).expect("valid grant revision"),
        release: digest('a'),
        profile: "knative-private-runtime-v1".to_owned(),
        profile_lock_sha256: digest('b'),
        inventory_sha256: digest('c'),
    }
}

fn assert_error_code<T>(
    result: Result<T, RuntimeControlErrorV1>,
    expected: RuntimeErrorCodeV1,
    scenario: &str,
) {
    assert_eq!(
        result.err().map(|error| error.code),
        Some(expected),
        "{scenario}: expected the exact closed error code, not success or another refusal"
    );
}

#[test]
fn matching_command_version_and_cutover_ids_pass_the_pure_consistency_rule() {
    let plan = cutover_plan();
    assert_eq!(
        require_command(
            &plan,
            1,
            plan.migration_id,
            plan.project_id,
            plan.binding_id
        )
        .map_err(|error| error.code),
        Ok(()),
        "a command referring to exactly this plan must pass identity consistency"
    );
}

#[test]
fn each_command_version_or_id_mismatch_is_invalid_request() {
    let plan = cutover_plan();
    let other = Uuid::from_u128(0x44444444_4444_4444_8444_444444444444);
    let cases = [
        (
            "zero version",
            0,
            plan.migration_id,
            plan.project_id,
            plan.binding_id,
        ),
        (
            "unknown version",
            2,
            plan.migration_id,
            plan.project_id,
            plan.binding_id,
        ),
        (
            "maximum version",
            u16::MAX,
            plan.migration_id,
            plan.project_id,
            plan.binding_id,
        ),
        (
            "other migration",
            1,
            other,
            plan.project_id,
            plan.binding_id,
        ),
        (
            "nil migration",
            1,
            Uuid::nil(),
            plan.project_id,
            plan.binding_id,
        ),
        (
            "other project",
            1,
            plan.migration_id,
            other,
            plan.binding_id,
        ),
        (
            "nil project",
            1,
            plan.migration_id,
            Uuid::nil(),
            plan.binding_id,
        ),
        (
            "other binding",
            1,
            plan.migration_id,
            plan.project_id,
            other,
        ),
        (
            "nil binding",
            1,
            plan.migration_id,
            plan.project_id,
            Uuid::nil(),
        ),
    ];
    for (scenario, version, migration, project, binding) in cases {
        assert_error_code(
            require_command(&plan, version, migration, project, binding),
            RuntimeErrorCodeV1::RuntimeInvalidRequest,
            scenario,
        );
    }
}

fn ack_data(installed_at: i64) -> RuntimeMaintenanceFacts {
    let plan = cutover_plan();
    RuntimeMaintenanceFacts {
        operator_id: Uuid::from_u128(0x55555555_5555_4555_8555_555555555555),
        installation_ack: RuntimeLegacyFenceInstallationAckV1 {
            migration_id: plan.migration_id,
            project_id: plan.project_id,
            binding_id: plan.binding_id,
            inventory_sha256: plan.inventory_sha256,
            installed_at_unix_ms: RuntimeUnixMsV1::new(installed_at)
                .expect("fixture acknowledgement time is nonnegative"),
            observed_writer_cas: None,
        },
        drain_evidence_sha256: None,
    }
}

#[test]
fn acknowledgement_data_at_or_before_now_passes_only_the_timestamp_rule() {
    for (installed_at, now) in [(0, 0), (999, 1_000), (1_000, 1_000), (i64::MAX, i64::MAX)] {
        assert_eq!(
            check_ack_time(&ack_data(installed_at), now).map_err(|error| error.code),
            Ok(()),
            "installed_at={installed_at}, now={now}: equality and past timestamps are not future; this does not authenticate the data"
        );
    }
}

#[test]
fn acknowledgement_data_one_millisecond_in_the_future_is_invalid_proof() {
    for (installed_at, now) in [(1, 0), (1_001, 1_000), (i64::MAX, i64::MAX - 1)] {
        assert_error_code(
            check_ack_time(&ack_data(installed_at), now),
            RuntimeErrorCodeV1::RuntimeInvalidProof,
            &format!("future acknowledgement data: installed_at={installed_at}, now={now}"),
        );
    }
}

#[test]
fn complete_cutover_plan_json_matches_the_literal_journal_format() {
    // Independent literal oracle: do not serialize the plan to build expected.
    // Distinct IDs/digests/counters detect swaps; the large counter must be a string.
    let expected: Value = serde_json::from_str(
        r#"{
            "schema_version": 1,
            "migration_id": "11111111-1111-4111-8111-111111111111",
            "project_id": "22222222-2222-4222-8222-222222222222",
            "binding_id": "33333333-3333-4333-8333-333333333333",
            "expected_generation": "9007199254740993",
            "expected_grant_revision": "17",
            "release": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "profile": "knative-private-runtime-v1",
            "profile_lock_sha256": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "inventory_sha256": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        }"#,
    )
    .expect("literal V1 journal plan JSON is valid");
    assert_eq!(plan_json(&cutover_plan()), expected);
}

#[test]
fn maintenance_actions_use_the_closed_admission_action_literals() {
    // These are protocol literals, not an oracle generated by the same mapper.
    assert_eq!(action_string(RuntimeAdmissionActionV1::Prepare), "prepare");
    assert_eq!(action_string(RuntimeAdmissionActionV1::Open), "open");
}

fn configured_profile_without_channel() -> SqlRuntimeMaintenance {
    let mut maintenance = SqlRuntimeMaintenance::disabled(DatabaseConnection::Disconnected);
    maintenance.profile = Some(RuntimeMaintenanceProfile {
        authority_id: Uuid::from_u128(0x66666666_6666_4666_8666_666666666666),
        installation_id: Uuid::from_u128(0x77777777_7777_4777_8777_777777777777),
        profile: "knative-private-runtime-v1".to_owned(),
        profile_lock_sha256: digest('b'),
        endpoint_scheme: "https".to_owned(),
        endpoint_port: 443,
    });
    maintenance
}

#[tokio::test]
async fn disabled_configuration_refuses_every_maintenance_act_without_sql() {
    let maintenance = SqlRuntimeMaintenance::disabled(DatabaseConnection::Disconnected);
    let plan = cutover_plan();
    for (scenario, act) in [
        ("inspect", RuntimeMaintenanceAct::Inspect),
        ("fence", RuntimeMaintenanceAct::Fence),
        (
            "challenge prepare",
            RuntimeMaintenanceAct::Challenge(RuntimeAdmissionActionV1::Prepare),
        ),
        (
            "challenge open",
            RuntimeMaintenanceAct::Challenge(RuntimeAdmissionActionV1::Open),
        ),
        ("prepare", RuntimeMaintenanceAct::Prepare),
        ("resume prepare", RuntimeMaintenanceAct::ResumePrepare),
        ("open", RuntimeMaintenanceAct::Open),
        ("block release", RuntimeMaintenanceAct::BlockRelease),
    ] {
        assert_error_code(
            maintenance.authorize(act, &plan).await,
            RuntimeErrorCodeV1::RuntimeKeyUnavailable,
            scenario,
        );
    }
}

#[tokio::test]
async fn configuration_guard_rejects_each_plan_mismatch_before_the_absent_channel() {
    let maintenance = configured_profile_without_channel();
    let valid_plan = cutover_plan();
    let mut cases = Vec::new();
    let mut plan = valid_plan.clone();
    plan.schema_version = 2;
    cases.push(("unknown plan version", plan));
    let mut plan = valid_plan.clone();
    plan.migration_id = Uuid::nil();
    cases.push(("nil migration", plan));
    let mut plan = valid_plan.clone();
    plan.profile = "unapproved-profile".to_owned();
    cases.push(("different profile", plan));
    let mut plan = valid_plan;
    plan.profile_lock_sha256 = digest('d');
    cases.push(("different profile lock", plan));

    for (scenario, plan) in cases {
        assert_error_code(
            maintenance
                .authorize(RuntimeMaintenanceAct::Fence, &plan)
                .await,
            RuntimeErrorCodeV1::RuntimeInvalidRequest,
            scenario,
        );
    }
}

#[tokio::test]
async fn matching_plan_cannot_replace_the_absent_maintenance_channel_configuration() {
    let maintenance = configured_profile_without_channel();
    assert_error_code(
        maintenance
            .authorize(RuntimeMaintenanceAct::Fence, &cutover_plan())
            .await,
        RuntimeErrorCodeV1::RuntimeAuthorityUnavailable,
        "matching plan data with no maintenance channel",
    );
}
