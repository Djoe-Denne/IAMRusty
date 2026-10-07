# Manifesto / Hive validation — 2026-10-05

Parent-authorized non-IAM Cargo lease. No IAM tests, keys, commits or pushes.
Compiler retained under parent lease: `493026f9bfc97cf2a64e768f4d7ab198875c4b72d62f8f7bb84fe0fe320ad805`.
Before execution: compiler `docker top` = `sleep infinity` only; rustfmt and clippy installed.
Mounts verified: `/workspace`, `/linux-target` (aiforall-iam-fixes-target-linux), SDK registry/git caches, `/evidence`, Docker socket.
Before inventory running unrelated IDs (not owned): `cdcb3afd7776b6c60c7bc930535fd2ea9099796986ffe527d26b18bc0fa7ef76`, `89aea5f8c01caaf1fde09919c9925e0c2f235862736814658bb25417759f4a83`.
Fixture run ID: `mh-20261005-sol-01`; fixture ledger `/evidence/mh-fixtures-sol-01` (separate from parent ledger).
Status: final non-IAM validation PASS at HEAD `87a6a10` dirty. Historical phase-2 failures below were resolved by the parent-authorized four lexical substitutions; see final validation section. No commit/push by this worker.
Evidence under parent `/evidence` mount: `mh-repro-model.log`, `mh-repro-root.log`, `mh-repro-lazaret.log`, `mh-repro-hive.log`.
Completed pre-edit repro: model, HTTP lib and Lazaret scope selectors each FAIL (0 passed / 1 failed); exact expected NotFound/absent errors reproduced.
Hive exact selector: FAIL (0 passed / 1 failed), original verifier assertion at line 91, 7.66 s.
Repro fixture IDs from harness created events: OpenFGA `41a94932d96daa1edbc4d4ad1b67550a49ab4727b156dfef4398637b497a36ee`; PostgreSQL `6f560a46a804ee02d7532ff3471f24a31049eaf4f521849f3fac15c17348e61e`. Both remained running after harness exit; explicitly stopped gracefully by exact ID before next execution. Compiler top rechecked idle.

## Changes and contract preservation

Manifesto: ten `workspace_root` helpers now ascend `../..`: P1 T7, P2 T7, P3 T1 absence/T2 gate/T3 identity/T4 gateway/T5 consent/T6 KV/T7 invoke, P4 T1 absence. P1 T5 ACL and P2 T3 persist model guards now target `../../ops/openfga/model.fga`. Corrected root comment; no traversal, lists, allowlists or assertion changes.
Hive: `tests/organization_signer_permissions.rs` asserts exact configured JWKS URL, ordered `[RS256, HS256]`, test-secret constant, issuer/audience, and no trusted mesh. All HTTP/org/revocation/outbound RPC assertions unchanged. Helper and production configuration unchanged.
Protected files verified byte-identical before/after: `tests/fixtures/db.rs` SHA256 `5065edeb8732a54213520dce68e1bec846d885d466870b8b667f88aac9bf136b`; `tests/members_api_tests.rs` SHA256 `b632150534b5e3766290b14d7f2c68758f4515b6e412df6b3366dbd230529213`. Baseline dirty AGENTS unchanged SHA256 `c5f8fe4377d7b7f1aee0f1c7205b9abf31c186cd3385bab9743f779c11ceef34`. No IAM/key/submodule edits.

## Commands and results

Exact sequential validation commands/selectors are in ignored `mh-validate.sh` alongside this file; compiler workdir `/workspace`, Linux target `/linux-target`.
Pre-edit commands: `cargo test --locked -p manifesto-service --test apparatus_p1_t5_acl t5_model_has_no_new_apparatus_type -- --exact`; same pattern P1 T7 `t7_component_routes_unchanged_five_registrations`, P3 T2 `t2_p4_tokens_forbidden_in_lazaret_src`; Hive `cargo test --locked -p hive-service --test organization_signer_permissions configure_requires_admin_on_the_exact_organization_before_any_iam_rpc -- --exact`. All exit 101 with expected causes.
Post-edit: 43 Manifesto pure guards executed, **38 PASS / 5 FAIL**. Both directly corrected model selectors pass. P4 gate integrity passes (P4 target 10/10). No remaining filesystem NotFound. Hive exact real-harness test **1 PASS / 0 FAIL**, 7.10 s (pre-edit 7.66 s FAIL).
`cargo build --locked --all-targets --package manifesto-service --package manifesto-domain --package manifesto-application --package manifesto-infra --package manifesto-http_server --package manifesto-setup --package manifesto-configuration`: PASS, 3m34s.
`cargo build --locked --all-targets --package hive-service --package hive-domain --package hive-application --package hive-infra --package hive-http --package hive-setup --package hive-configuration`: PASS, 3m03s.
Targeted service `cargo clippy --locked -p manifesto-service -p hive-service --all-features --tests --benches` with CI common warning groups and test allowances: exit 0, 1m47s, 331 warning lines (not a warning-free gate).
`cargo fmt --all -- --check`: PASS, read-only. Formatting mutation restricted to three affected files with rustfmt `skip_children=true`.
Full workspace Sonar production and test commands in `.github/workflows/ci.yml:505–518` remain NOT RUN: `cargo clippy --workspace --locked --all-features --lib --bins --examples --message-format=json -- "${common_clippy[@]}" -W clippy::unwrap_used -W clippy::expect_used -W clippy::panic`; test counterpart `--tests --benches` and `-A` for those three lints. Deferred because IAM is suspended. Full coverage matrices not rerun; no equivalent claim.

## Residual failures requiring parent scope/contract decision

- P1 `t7_no_p2_runtime_tokens_in_manifesto_prod`, P2 `t7_p2_tokens_only_on_allowlist_l`: `services/Manifesto/infra/src/event/consumer.rs:19` comment contains `poll`.
- P3 `t2_p4_tokens_forbidden_in_lazaret_src`, `t3_lazaret_src_forbids_p4_tokens`: `services/Lazaret/application/src/invoke.rs:78` and `configuration/src/lib.rs:241` comments contain `Kubernetes`.
- P3 `t3_manifesto_src_has_no_gateway_or_lazaret_or_session_issuance`: `services/Manifesto/application/src/usecase/component.rs:593` embedded test URL contains `/lazaret`.
These were masked by incorrect roots. No exclusions, assertion relaxation or unrelated source edits applied. Final validation command exits 1 because of these five guards.

## Final fixture inventory / lease handback

Post-fix run ID `mh-20261005-sol-02`, ledger `/evidence/mh-fixtures-sol-02`.
OpenFGA `6bd37026460f40bc246699bb1215825431ebd1904ef35d5997db3824e6ec3d38`; PostgreSQL `c9aab227738202c297547df6070cf030676fb2b6058d10fb9f152570d0857b44`. Both remained running after PASS and were stopped gracefully by exact created ID.
Final `docker ps -a` verifies all FOUR worker-owned fixture IDs **exited**; no newly retained runtime, no removal/prune. Compiler retained running under parent lease; final `docker top` only `sleep infinity`, **no Cargo/rustc**. Other baseline containers/Kind untouched; Desktop/WSL retained for parent, RAM not measured.
Diff self-review: changes limited to twelve Manifesto tests + one Hive test; no authorization assertions removed. Parent independent reviews and residual decisions pending.

## Final validation after four approved substitutions (HEAD 87a6a10 dirty)

Read architecte contract and both incremental PASS briefings (tests briefing actual filename `2026-10-05T1653Z-tests-lexical-residue-87a6a10.md`). No source edits in this final package; only this evidence update.

Compiler checked idle before execution. Reused `mh-validate.sh` lines 1–25, streaming via `head -n 25 ... | sed -e s@/evidence/mh-@/evidence/mh-final-@g -e s@mh-20261005-sol-02@mh-20261005-sol-03@g | sh`; excluded Hive and Clippy commands. All exact existing guard selectors retained.

- **43 Manifesto guards PASS / 0 FAIL**: eight pure targets 35 tests, two model selectors, six exact pure P3 T4/T5 selectors. Five previous lexical failures now pass; P4 gate-integrity target 10/10.
- `CARGO_TARGET_DIR=/linux-target cargo test --locked -p manifesto-application --lib attach_from_catalog_tests`: **6 PASS / 0 FAIL**, zero filtered/ignored; compilation 56.50 s.
- Total **49 PASS / 0 FAIL**.
- Exact Manifesto CI matrix `cargo build --locked --all-targets --package manifesto-service --package manifesto-domain --package manifesto-application --package manifesto-infra --package manifesto-http_server --package manifesto-setup --package manifesto-configuration`: **PASS**, 1m58s.
- Docker `cargo fmt --all -- --check`: **PASS**, no formatting mutations.
- Scoped `git diff --check -- services/Manifesto services/Lazaret/application/src/invoke.rs services/Lazaret/configuration/src/lib.rs`: **PASS on host**. Compiler attempt exited 127 (`git` absent); no installation or silent equivalence claim.

Logs: parent evidence directory `C:/Users/djden/AppData/Local/Temp/opencode/ci-20261005/mh-final-*.log` (`/evidence` mount).
All commands completed. No fixtures created by these pure tests; final running inventory equals initial compiler plus two pre-existing Kind nodes, untouched. Compiler final `docker top` only `sleep infinity`: **idle, no Cargo/rustc**; retained parent lease, exclusive Cargo slot returned. Parent owns compiler stop. No IAM tests/keys/secrets, SDK/protected Hive edits, commits or pushes. Full IAM/Sonar/coverage-matrix gates remain deferred; this is not full-CI equivalence.
