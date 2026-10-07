# Exact CI gates (observed 2026-10-05)

Source: `.github/workflows/ci.yml`; metadata only, not a workflow modification.
IAM tests/key work is suspended by the user. No push until the mandatory full gates can be replayed on the final combined diff. User-owned IAM/workflow edits must not be staged as this worker's changes.

## Build matrix

Run one Cargo at a time in the session-owned Linux compiler; no reduced `cargo check` substitute.

```sh
cargo build --locked --all-targets --package iam-service --package iam-domain --package iam-application --package iam-infra --package iam-http_server --package iam-setup --package iam-configuration
cargo build --locked --all-targets --package telegraph-service --package telegraph-domain --package telegraph-application --package telegraph-infra --package telegraph-http_server --package telegraph-setup --package telegraph-configuration
cargo build --locked --all-targets --package hive-service --package hive-domain --package hive-application --package hive-infra --package hive-http --package hive-setup --package hive-configuration
cargo build --locked --all-targets --package manifesto-service --package manifesto-domain --package manifesto-application --package manifesto-infra --package manifesto-http_server --package manifesto-setup --package manifesto-configuration
cargo build --locked --all-targets --package oodhive-monolith
cargo build --locked --all-targets --package sentinel-sync
cargo build --locked --all-targets --package ext-authz
```

## Both SonarCloud commands (ci.yml:488-518)

```sh
common_clippy=(
  -W clippy::all
  -W clippy::pedantic
  -W clippy::nursery
  -W clippy::cargo
  -W clippy::todo
  -W clippy::unimplemented
  -A clippy::cargo_common_metadata
  -A clippy::multiple_crate_versions
)
cargo clippy --workspace --locked --all-features --lib --bins --examples --message-format=json -- \
  "${common_clippy[@]}" -W clippy::unwrap_used -W clippy::expect_used -W clippy::panic \
  > target/sonar/clippy-production.jsonl
cargo clippy --workspace --locked --all-features --tests --benches --message-format=json -- \
  "${common_clippy[@]}" -A clippy::unwrap_used -A clippy::expect_used -A clippy::panic \
  > target/sonar/clippy-tests.jsonl
```

These are deferred while IAM source is changing concurrently. The named Linux target cache remains separate from the host target directory.

## Fmt and Coverage

```sh
cargo fmt --all -- --check
cargo llvm-cov <matrix.package_args> --locked --no-report --no-fail-fast --tests
cargo llvm-cov <matrix.package_args> --locked --no-report --no-fail-fast --lib --bins
```

Coverage preserves the test exit status around `cargo llvm-cov report --lcov --output-path <matrix.output_path>`.
Global runtime test environment includes `RUN_ENV=test`, `RUST_BACKTRACE=1`.
Integration fixture MODE/RUN_ID/LEDGER_DIR are scoped to BOTH preparation and integration STEPS, not the job. Use unique local run IDs and ledgers. Workflow currently contains the user-owned RSA8192 environment-variable reference; do not read its secret value.
No extra CI teardown step is present after integration upload; the fixture harness owns normal teardown. Locally, verify created-ID ledgers and stop any owned fixtures left running.
