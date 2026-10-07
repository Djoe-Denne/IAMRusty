# IAM fixes — active infrastructure handoff

Baseline: user-owned 87a6a10, preserved. No commit/push or SDK edits.

## Verified targeted fixes

- Binding and PEM CRLF tests accidentally transformed the committed CRLF fixture into CRCRLF. Normalize CRLF to LF before testing LF and CRLF; all mismatch/mutant negatives retained.
- Rotation test embedded a truncated PEM. Replace with a real seeded RSA2048 public key; admission/mappings unchanged. rsa dev dependency declared; Cargo.lock delta is only iam-http_server's rsa edge.
- Validation imported tracing::log macros, bypassing the test's tracing subscriber. Use tracing macros; same safe messages and sentinel coverage retained.
- RSA8192 loader preserves user pregeneration. Explicit size assertion plus CI-required missing/empty guards and four pure loader tests. Both IAM coverage steps reject an empty secret; no secret values read/printed and no RSA8192 generation.

Targeted commands: /evidence/iam-go-targeted.sh. Logs fixed-{binding,rotate,redaction,pem-crlf,loader}.log in the parent temp directory. All exit 0: four original tests 1 passed each; loader 4 passed. Elapsed respectively 1.99s,20.09s,1.96s,24.86s,2.23s. Initial --locked resolution stopped before compilation; minimal offline resolver added only the rsa edge, then all --locked tests passed.

## Active job / blocker

Remaining IAM integration reproduction was launched before touching those sources: /evidence/iam-go-repro-it.sh, shell job sh_10d29b274001wiTM3WG1KMwS44. Logs iam-go-repro-it.log and iam-go-repro-it-summary.log. This bounded operation is NOT yet verified complete; parent inherits its exclusive Cargo lease until actual completion/idle proof.

Context Mode subsequently returned ENOSPC while preparing a metadata-only local-material availability check. That check did not return; local RSA8192 availability remains UNKNOWN. A bounded disk/process observation also timed out at 15s. No fallback around Context Mode, deletion, unknown cleanup, global shutdown or abrupt kill attempted. Parent container-runtime-debugger arbitration needed.

Compiler 493026f9bfc97cf2a64e768f4d7ab198875c4b72d62f8f7bb84fe0fe320ad805 was verified exited, explicitly restarted under parent lease, then verified running with expected mounts/image. No active cargo/rustc before first Cargo; llvm-cov 0.9.1/rustfmt/clippy present. Own ledger: C:/Users/djden/AppData/Local/Temp/opencode/ci-20261005/iam-go-ledger.json. Fixture run context: MODE=local, RUN_ID=iam-go-repro-20261005, LEDGER_DIR=/evidence/iam-go-repro-fixtures. Fixture IDs/final states still require completion-ledger reconciliation; do not infer ownership from names. Compiler retained for parent, protected/foreign Kind nodes untouched.

## Incomplete

Remaining IAM binary reproductions/fixes; actual provisioned RSA8192 positives; full IAM build/lib+bins/coverage, fmt and final fixture inventory. Common fixture investigation (read-only) identified config-only unbound RSA token helpers and fresh-config reuse when building another app against a live generated epoch; no fixes to those files yet.
