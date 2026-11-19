# ADR-0507: CI uses service-local build dependencies and a shared compiler cache

- Status: Proposed
- Reality: Partial; diagnostic and authoritative coverage/Sonar verified; IAM pilot E2E/cleanup qualified, complete adopted run pending; cache throttling and reservation conflicts unresolved
- Date: 2026-11-17
- Author: `architecte`, session `ses_e152b44f9ffeCvHhlurgQWqbkK`; persisted by the root agent
- Decision makers: Pending explicit human acceptance
- Supersedes: None while Proposed. Acceptance would replace only the Clippy job-placement clause of ADR-0501, Decision 2.
- Implementation contract: [CI service-local execution](../contracts/ci-service-local-execution.md)
- IAM-only forward specialization: [compile-once adoption](../contracts/ci-iam-compile-once-adoption.md)

## Context

Baseline: application `f8ddff4f0e958b024d59b528e6e465a777cdcfd7`, SDK `1db6c9de8708f6451d91a56f482b053026487c8f`.

The eight-entry native build matrix globally blocks both coverage matrices and E2E. Clippy starts only after coverage. The existing compiler cache retains overlapping dependency archives but removes local compilation artifacts.

E2E admission binds exact successful producer display names, the literal job ID `build`, empty compiler-wrapper settings, source/SDK revisions, workflow/run/attempt identities and artifact digests. Changing the DAG or introducing a compiler wrapper requires a deliberate producer-contract migration, not hiding its environment.

## Decision

1. Replace the build matrix with eight explicit top-level build jobs and a repository-local build composite. Preserve their package selections and display names. Tests depend on their own service build; E2E depends only on `build-iam`, `build-oauth` and `build-ext-authz`.
2. Share content-addressed sccache results across compiling lanes. Keep Cargo downloads in a separate registry-only cache with one trusted writer. Do not archive `target/`, profiles or sccache's local directory, and do not introduce a workspace prewarm barrier.
3. Pin Rust, the sccache release archive/executable and cargo-llvm-cov. Use a repository-owned digest-first installer: authenticate archive bytes before extraction, expose only the verified executable and register no upstream post hook. Preserve cargo-llvm-cov's default wrapper, which chains the prior compiler wrapper in version 0.9.1. Disable incremental compilation in CI only.
4. Emit native bundle schema `aiforall.k6-binaries/v2`. Bind each role to its exact job ID and unchanged display name. Permit only literal `RUSTC_WRAPPER=sccache`, with the reviewed executable version/digest verified before compilation and collection. Retain all instrumentation, Cargo-config, ELF, package and immutable artifact checks.
5. Inventory all workspace packages while preserving the existing executed scope and explicit coverage gaps. Assign selected lib/bin and integration targets exactly once to whole-binary shards. Preserve libtest, serial fixtures, ignored tests, live-test opt-ins and the separate Apparatus P0 gate.
6. Move the existing two Clippy policies unchanged to an independent job after formatting. Sonar imports complete validated Clippy and coverage reports and invokes no Cargo command. Empty or incomplete reports cannot qualify a scan.
7. Master always executes the complete declared baseline scope. PRs also execute it by default. Selective PR execution requires `vars.CI_SELECTIVE_PR_TESTS == 'true'`; unknown/global changes and comparison uncertainty fall back to full execution. Partial PR coverage never becomes canonical `lcov` or complete Sonar coverage.

The v2 implementation rejects native v1 bundles; saved-image admission is unchanged. On 2026-11-18 the user authorized publication after the native-v1 migration and changed check names were disclosed. This operational authorization does not promote this ADR to Accepted or enable selective PR execution. External required-check configuration is unchanged.

### Forward amendment: diagnostic aggregation and Telegraph isolation

Authored by `architecte`, session `ses_e152b44f9ffeCvHhlurgQWqbkK`, on the user's 2026-11-18 request. This amendment leaves ADR-0507 Proposed.

Coverage aggregation separates report integrity, declared-scope completeness and test verdict. A complete, compatible set of structurally valid LCOV reports may merge successfully despite a nonzero test outcome. Such output is diagnostic, not authoritative: Cargo exit 101 may mean compilation failure or failed assertions, and report generation does not prove every assigned target ran.

Every successful aggregation publishes `coverage-diagnostic` with an explicit outcome manifest. Canonical `lcov` and Sonar admission retain the previous full-scope, complete-report, all-tests-successful conditions. Sonar's skip on diagnostic-only evidence is deliberately reported as coverage not qualified. Test failures remain unsuccessful; the user's Telegraph exclusion is a mission-assessment rule, not a blanket CI exemption.

Preserve Telegraph's unit partition and exact original `tests/0`. Replace only its former second integration partition with three whole-binary groups: routing (17 baseline cases), HTTP/outbox/common/mTLS/domain/application (41) and email-verified (8). Telegraph now has one unit and four integration partitions. Preserve libtest, features, fixtures and the complete target union; no test-case sharding or application/flakiness fix is implied.

### Forward amendment: remove redundant native test builds

Authored by `architecte`, session `ses_e152b44f9ffeCvHhlurgQWqbkK`, following the user's request for measured build-time improvement rather than diagnostic counters. This amendment leaves ADR-0507 Proposed.

The eight native build jobs use `cargo build --locked` with unchanged explicit packages and default features. Default target selection builds production libraries/binaries instead of precompiling test, benchmark and example targets that separate instrumented coverage jobs compile again. Coverage assignments and execution commands remain unchanged, as do both Clippy policies. This removes redundant compilation, not tests; it intentionally removes a separate native compile/link-only assurance for non-production targets.

Native bundles record the actual successful build command. Native-v2 consumers deliberately admit the new production form and the historical all-targets form through a closed package-only grammar. Producer identities, required executables, dev profile, ABI, instrumentation exclusions, verified wrapper and immutable provenance remain unchanged; native-v1 stays rejected.

Measure comparable warm hosted builds before changing cache transport at the same time. The user authorized an isolated IAM seed/rebuild workflow with two fresh runners and no target transfer. If per-object GHA requests still dominate measured time, batched persistence of the pinned local sccache store requires an explicit trusted-writer/consolidation contract and cost qualification. Do not introduce a prewarm barrier, per-service target archives, a new network service or a quota/billing change.

### Forward amendment: isolated IAM compile-once pilot

An opt-in, reserved-branch pilot builds IAM test executables and instrumented production executables in one producer target directory. Fresh test shards and the existing E2E campaign reuse those binaries without rebuilding. Only compatible Cargo units are eligible to share; E2E companions OAuth and ext-authz remain native. The dedicated artifact and admission boundary is described in the [pilot contract](../contracts/ci-iam-compile-once-pilot.md). Canonical CI and release behavior are unchanged. The pilot does not establish runtime parity or a speedup until measured on hosted runners.

### Forward amendment: adopt compile-once execution for IAM only

The user authorized full-pipeline adoption after a verified instrumented IAM E2E campaign. Hosted run [44493607389, attempt 1](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44493607389), source `aba26931ff01de2149587275b5a928ba74194d73`, passed all five existing scenarios, restoration and cleanup. The five test partitions passed separately in [44351218515](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44351218515). These are separate qualifications, not a successful combined canonical run or a speedup measurement. ADR-0507 remains Proposed / Partial; implementation authorization is not formal acceptance.

Specialize the main CI IAM producer, retaining job ID `build-iam` and display name `Build (IAMRusty)`: build instrumented IAM server/migration and the exact canonical test-target union in one producer target directory. Reuse only compatible Cargo units; preserve distinct profile, feature, host and harness variants. Retain the qualified producer-only `line-tables-only` settings. Fresh IAM coverage runners replay their assigned whole binaries without application compilation, through the existing canonical coverage entry point, fixtures, outcome receipts and aggregation policy. Other services retain their existing behavior.

Use a dedicated, typed instrumented-IAM admission path bound to the existing successful producer identities and exact source/SDK/workflow/run/attempt/artifact provenance. Never relabel instrumented output as native v2 or widen native, release or saved-image admission. OAuth and ext-authz remain native. Preserve the unchanged E2E scenarios, trust restrictions and cleanup; E2E profiles remain private and excluded from canonical coverage.

Freeze the IAM server/migration bundle before test compilation can overwrite outputs. Consumers still require the entire producer job to succeed: early upload does not bypass this condition. IAM coverage and E2E now wait for IAM test-executable compilation and bundling. This new critical path is an explicit IAM-only tradeoff against repeated compatible compilation, not a general prewarm barrier. Measure it, transfers and consumer setup in a complete hosted run before reporting rollout qualification or a performance improvement. The [adoption contract](../contracts/ci-iam-compile-once-adoption.md) narrowly overrides the earlier native-only main-CI and consumer-compilation clauses for IAM; the pilot evidence remains experimental history.

## Consequences and limits

- Explicit jobs remove the matrix-wide dependency barrier without delaying producer admission until tests finish.
- Shared compilation results do not require merging per-service archives. Different feature/coverage/compiler settings remain distinct.
- Linker-producing crates, procedural macros, metadata-only lint work, native build scripts, cold parallel misses and GHA backend request limits constrain reuse.
- Neither a cache below 10 GB nor faster hosted execution is claimed without measurement.
- Runner-issued GHA capabilities are exported by a minimal repository-owned JavaScript action without a post hook. Missing capabilities disable remote caching; statistics authenticate the executable before execution.
- Non-IAM shards still compile their selected instrumented targets. The IAM forward specialization replays verified instrumented executables; native binaries are never silently reused as coverage binaries.
- Receipt-checked aggregation rejects missing, duplicated, foreign, partial or corrupted evidence and failed report generation. Failed test commands may yield a diagnostic merge but never canonical coverage or a passing test verdict.
- No change to Sonar exclusions, thresholds, warnings, billing, quotas, application behavior or SDK is authorized.
- ADR-0501 remains an unchanged retrospective record; this Proposed ADR is not an implicit acceptance or global quality pass.

## Rejected alternatives

- One build matrix: a dependency cannot select one matching child.
- Reusable producer workflows: unnecessary name/identity complexity at the bundle admission boundary.
- Combined build/test producers as a general policy: delay E2E producer success. The measured IAM-only forward specialization accepts the test-compilation/bundling wait, not test execution in the producer.
- One shared `target/` archive: first-writer snapshots are not merged and retain duplicated variants.
- Clearing the wrapper, arbitrary wrapper admission or accepting whichever LCOV files happen to exist: conceal provenance or incomplete evidence.
- A process-per-test runner migration: changes fixture economics and isolation beyond this task.

## References

- [ADR-0501](0501-qualite-fmt-clippy-sonar.md)
- `.github/workflows/ci.yml`, `.github/workflows/ci-coverage.yml`
- `ops/tests/k6/ci/binaries.py`, `ops/tests/k6/ci/artifacts.py`
- [sccache Rust limitations](https://github.com/mozilla/sccache/blob/main/docs/Rust.md)
- [sccache GHA backend](https://github.com/mozilla/sccache/blob/main/docs/GHA.md)
- [cargo-llvm-cov wrapper chaining](https://github.com/taiki-e/cargo-llvm-cov/blob/v0.9.1/src/wrapper.rs)

Hosted persistent cache reuse and two full executions outside Telegraph IT are recorded in [the qualification report](../missions/20261118-ci-cache-hosted-qualification.md). Both workflows remain red because excluded test failures block authoritative coverage and Sonar. Clean cache writes, steady-state performance and backend-unavailable qualification remain unproven.

The subsequent [coverage-outcome amendment](../missions/20261118-ci-coverage-outcomes-and-telegraph-shards.md) has real hosted proof that diagnostic merging succeeds despite failed test commands. Telegraph's finer partitions are deployed; underlying intermittency remains observed.

The [backend-error investigation](../missions/20261118-sccache-backend-errors.md) subsequently verified the positive canonical/Sonar path and reproduced HTTP 429 cache throttling plus HTTP 409 duplicate reservations. Its overall workflow remained unsuccessful because of a separate report deployment failure. No traffic-control or backend remediation is implied, and this evidence does not promote the ADR to Accepted.
