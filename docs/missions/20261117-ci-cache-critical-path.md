# CI compiler cache and critical-path implementation

## Status and boundary

Local implementation passed the bounded review described below. At the 2026-11-17 handoff it was **not committed or published**, under the user's explicit publication hold.

On 2026-11-18 the user authorized commit/push, the announced native-v2 publication migration, deletion of repository Actions caches and a two-consecutive-run hosted qualification. Follow-up status and evidence are tracked in [the hosted qualification report](20261118-ci-cache-hosted-qualification.md). The original local evidence below is historical, not proof of hosted success.

Baseline HEAD remains `f8ddff4f0e958b024d59b528e6e465a777cdcfd7` on `master`; pinned SDK remains `1db6c9de8708f6451d91a56f482b053026487c8f`. No task branch/worktree, application/SDK edit or local Docker/Kind/WSL runtime was created.

Decision and contract remain Proposed: [ADR-0507](../adr/0507-ci-service-local-dag-shared-compiler-cache.md), [execution contract](../contracts/ci-service-local-execution.md). Operational details: [guide](../guides/ci-cache-and-service-lanes.md).

## Implemented

- Shared content-addressed sccache results across all existing compiling CI lanes; no per-service `target/` archives.
- One registry-only Cargo download cache with one trusted master writer.
- Repository-owned digest-first installer, exact executable selection, verified-only statistics and masked GHA runtime capability bridge. Missing capabilities genuinely disable GHA, including its version override.
- Eight explicit native build jobs; each service's coverage depends only on its own build. E2E waits only for IAM/OAuthConnect/Ext Authz.
- Whole-target partitions: IAM4, Telegraph2, Hive2, Manifesto3, plus disjoint unit lanes. Real Cargo metadata audit:59 packages and193 unique baseline coverage targets across19 partitions.
- Explicit package ownership/registration and baseline coverage gaps. No implicit expansion of live-test features or claim of workspace-wide coverage.
- Source-bound outcome receipts and complete-set aggregation. Missing/failed/foreign/partial/duplicate/corrupted evidence cannot produce authoritative LCOV.
- Independent Clippy with unchanged two-pass policies; import-only Sonar without Cargo or empty-report fallbacks.
- Conservative selective-PR planning implemented but **default off**. Master/default PRs retain full declared coverage.
- Native bundle v2 explicitly attests the actual compiler wrapper and binds each role to its exact producer job; original source/SDK/workflow/attempt/artifact/ELF checks remain.

## Final evidence

- `actionlint1.7.12`: PASS for main CI, reusable coverage and referenced local action metadata. Archive checksum verified before installing the temporary native linter.
- New planner/cache/report regressions: **20 passed**, no skip; final log `pipeline-verified-tests.stderr.log`.
- Existing CI/E2E refusal/wiring suites: **133 total,132 passed,1 skipped**; final log `ci-seam-verified-tests.stderr.log`.
- Clippy argument/runner parity: **8 passed**; `clippy-parity-tests.stderr.log`.
- Real read-only Cargo metadata/partition audit: PASS,59 registered packages,193 unique target assignments.
- Real approved Linux sccache archive layout and archive/executable hash checks: PASS through the actual installer function. Linux executable version execution and PATH publication were deliberately not performed on Windows.
- `git diff --check`: PASS.
- Independent security re-review: PASS, installer-lifecycle S-1 HIGH closed.
- Independent correctness re-review: PASS, explicit-registration C-1 and YAML C-2 closed; final missing-capability override delta reviewed.

Per-command logs and disposable tool fixtures are under `.tmp/ci-cache-20261117/`, excluded from delivery. Commands were separately named and asynchronous; no application compilation or integration test runtime was started.

## Non-green broad-suite result

An additional broad Ops run executed188 tests:160 passed,27 skipped,1 failed. The failure was `test_minimal_native_pki.NativePkiTests.test_initial_issue_reuse_and_real_dependency_sans`, at its Windows owner/protected/private ACL predicate, in unchanged PKI/test code.

This broad suite is **not PASS**. No baseline run established that the failure predates the mission; no unrelated PKI/ACL correction or weakened assertion was applied. Its retained log is `ops-regression-tests.stderr.log`.

## Publication and qualification still pending

- Commit/push and the announced native-v1 retirement were authorized on 2026-11-18. Already saved-image admission is unchanged; external required-check configuration has not been modified.
- Separate approval before enabling `CI_SELECTIVE_PR_TESTS`.
- Hosted full-run coverage equivalence, real Linux fixture health, cold/warm/backend-unavailable qualification and measured cache footprint/time.
- The user explicitly authorized clearing all repository Actions caches before the first hosted run on 2026-11-18, replacing the earlier deferred-deletion plan.

No storage reduction, hosted speedup, remote green CI or complete mission publication is inferred from static/local evidence.
