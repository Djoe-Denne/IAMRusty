#!/bin/sh
export CARGO_TARGET_DIR=/linux-target
export RUSTYCOG_TEST_RUNNER_MODE=local
export RUSTYCOG_TEST_RUN_ID=mh-20261005-sol-02
export RUSTYCOG_TEST_LEDGER_DIR=/evidence/mh-fixtures-sol-02
mkdir -p "$RUSTYCOG_TEST_LEDGER_DIR"
run() {
    label=$1
    shift
    "$@" > "/evidence/mh-$label.log" 2>&1
    status=$?
    echo "$label:$status"
    [ "$status" -eq 0 ] || failed=1
}
failed=0
run pure cargo test --locked --no-fail-fast -p manifesto-service --test apparatus_p1_t7_gate --test apparatus_p2_t7_gate --test apparatus_p3_t1_absence --test apparatus_p3_t2_gate --test apparatus_p3_t3_identity --test apparatus_p3_t6_kv --test apparatus_p3_t7_invoke --test apparatus_p4_t1_absence
run model-p1 cargo test --locked -p manifesto-service --test apparatus_p1_t5_acl t5_model_has_no_new_apparatus_type -- --exact
run model-p2 cargo test --locked -p manifesto-service --test apparatus_p2_t3_persist t3_openfga_model_has_no_apparatus_type -- --exact
for selector in t4_handler_does_not_call_world_read t4_openfga_model_has_no_apparatus t4_new_prod_files_forbid_skip_and_runtime_tokens; do
    run "$selector" cargo test --locked -p manifesto-service --test apparatus_p3_t4_gateway "$selector" -- --exact
done
for selector in t5_openfga_model_has_no_apparatus t5_handler_does_not_call_world_read t5_prod_files_forbid_gateway_lazaret_and_skip; do
    run "$selector" cargo test --locked -p manifesto-service --test apparatus_p3_t5_consent "$selector" -- --exact
done
run build-manifesto cargo build --locked --all-targets --package manifesto-service --package manifesto-domain --package manifesto-application --package manifesto-infra --package manifesto-http_server --package manifesto-setup --package manifesto-configuration
run build-hive cargo build --locked --all-targets --package hive-service --package hive-domain --package hive-application --package hive-infra --package hive-http --package hive-setup --package hive-configuration
run clippy cargo clippy --locked -p manifesto-service -p hive-service --all-features --tests --benches -- -W clippy::pedantic -W clippy::nursery -W clippy::cargo -W clippy::todo -W clippy::unimplemented -A clippy::cargo_common_metadata -A clippy::multiple_crate_versions -A clippy::unwrap_used -A clippy::expect_used -A clippy::panic
run hive cargo test --locked -p hive-service --test organization_signer_permissions configure_requires_admin_on_the_exact_organization_before_any_iam_rpc -- --exact
echo validation-complete
exit "$failed"
