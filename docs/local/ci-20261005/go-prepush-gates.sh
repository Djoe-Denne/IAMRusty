#!/usr/bin/env bash
# go-prepush-gates.sh — exact pre-push gate runner (coordination artifact, NOT product code).
# Command source of truth: docs/local/ci-20261005/ci-checklist.md; tokens identical to
# .github/workflows/ci.yml @ HEAD 87a6 (build matrix, both SonarCloud clippy commands, fmt).
# The authoring worker does NOT execute this script. The parent runs it later under the
# exclusive Docker Cargo lease; the runtime gater records the same compiler (1.94) and
# fmt/clippy components. Test/fixture environment is inherited only — nothing is embedded,
# dumped, or echoed here. No secrets are read or printed by this script.

set -eu

cd /workspace

export CARGO_TARGET_DIR=/linux-target
export RUN_ENV=test
export RUST_BACKTRACE=1

LOGDIR=/evidence/go-prepush-gates-20261005
mkdir -p "$LOGDIR"

TOTAL_START=$SECONDS

# Compact per-gate record: PASS/FAIL, exit code, elapsed, log path. No env dump, no secrets.
gate() {
  local label=$1; shift
  local rc=0
  local start=$SECONDS
  "$@" >"$LOGDIR/$label.log" 2>&1 || rc=$?
  local elapsed=$((SECONDS - start))
  if [ "$rc" -eq 0 ]; then
    printf '[GATE] %-18s PASS exit=0 elapsed=%ss log=%s/%s.log\n' "$label" "$elapsed" "$LOGDIR" "$label"
  else
    printf '[GATE] %-18s FAIL exit=%s elapsed=%ss log=%s/%s.log\n' "$label" "$rc" "$elapsed" "$LOGDIR" "$label"
    exit "$rc"
  fi
}

tokens() {
  printf '[TOKENS] %s\n' "$*"
}

# --- Exact CI build matrix (7 rows, CI order; no --jobs, no extra features, no cargo check) ---
iam_pkgs=(--package iam-service --package iam-domain --package iam-application --package iam-infra --package iam-http_server --package iam-setup --package iam-configuration)
telegraph_pkgs=(--package telegraph-service --package telegraph-domain --package telegraph-application --package telegraph-infra --package telegraph-http_server --package telegraph-setup --package telegraph-configuration)
hive_pkgs=(--package hive-service --package hive-domain --package hive-application --package hive-infra --package hive-http --package hive-setup --package hive-configuration)
manifesto_pkgs=(--package manifesto-service --package manifesto-domain --package manifesto-application --package manifesto-infra --package manifesto-http_server --package manifesto-setup --package manifesto-configuration)

tokens iam cargo build --locked --all-targets "${iam_pkgs[@]}"
gate iam cargo build --locked --all-targets "${iam_pkgs[@]}"

tokens telegraph cargo build --locked --all-targets "${telegraph_pkgs[@]}"
gate telegraph cargo build --locked --all-targets "${telegraph_pkgs[@]}"

tokens hive cargo build --locked --all-targets "${hive_pkgs[@]}"
gate hive cargo build --locked --all-targets "${hive_pkgs[@]}"

tokens manifesto cargo build --locked --all-targets "${manifesto_pkgs[@]}"
gate manifesto cargo build --locked --all-targets "${manifesto_pkgs[@]}"

tokens monolith cargo build --locked --all-targets --package oodhive-monolith
gate monolith cargo build --locked --all-targets --package oodhive-monolith

tokens sentinel-sync cargo build --locked --all-targets --package sentinel-sync
gate sentinel-sync cargo build --locked --all-targets --package sentinel-sync

tokens ext-authz cargo build --locked --all-targets --package ext-authz
gate ext-authz cargo build --locked --all-targets --package ext-authz

# --- Both SonarCloud clippy commands (ci.yml:496-526). Compiler artifacts stay in
# /linux-target (CARGO_TARGET_DIR); target/sonar/*.jsonl are workspace report files only. ---
mkdir -p target/sonar

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

clippy_production() {
  # stdout = Sonar report file (path unaltered); stderr flows to the gate log.
  cargo clippy --workspace --locked --all-features --lib --bins --examples --message-format=json -- \
    "${common_clippy[@]}" \
    -W clippy::unwrap_used \
    -W clippy::expect_used \
    -W clippy::panic \
    > target/sonar/clippy-production.jsonl
}

clippy_tests() {
  cargo clippy --workspace --locked --all-features --tests --benches --message-format=json -- \
    "${common_clippy[@]}" \
    -A clippy::unwrap_used \
    -A clippy::expect_used \
    -A clippy::panic \
    > target/sonar/clippy-tests.jsonl
}

tokens clippy-production cargo clippy --workspace --locked --all-features --lib --bins --examples --message-format=json -- "${common_clippy[@]}" -W clippy::unwrap_used -W clippy::expect_used -W clippy::panic
gate clippy-production clippy_production

tokens clippy-tests cargo clippy --workspace --locked --all-features --tests --benches --message-format=json -- "${common_clippy[@]}" -A clippy::unwrap_used -A clippy::expect_used -A clippy::panic
gate clippy-tests clippy_tests

# --- fmt gate (LAST), exact CI token ---
tokens fmt cargo fmt --all -- --check
gate fmt cargo fmt --all -- --check

TOTAL_ELAPSED=$((SECONDS - TOTAL_START))
printf '[DONE] all gates PASS total_elapsed=%ss\n' "$TOTAL_ELAPSED"
printf '[DONE] logs=%s reports=target/sonar/clippy-production.jsonl target/sonar/clippy-tests.jsonl\n' "$LOGDIR"
