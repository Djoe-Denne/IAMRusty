# ADR-0507: CI uses service-local build dependencies and a shared compiler cache

- Status: Proposed
- Reality: Partial; local review passed, publication authorized on 2026-11-18, hosted qualification pending
- Date: 2026-11-17
- Author: `architecte`, session `ses_e152b44f9ffeCvHhlurgQWqbkK`; persisted by the root agent
- Decision makers: Pending explicit human acceptance
- Supersedes: None while Proposed. Acceptance would replace only the Clippy job-placement clause of ADR-0501, Decision 2.
- Implementation contract: [CI service-local execution](../contracts/ci-service-local-execution.md)

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

## Consequences and limits

- Explicit jobs remove the matrix-wide dependency barrier without delaying producer admission until tests finish.
- Shared compilation results do not require merging per-service archives. Different feature/coverage/compiler settings remain distinct.
- Linker-producing crates, procedural macros, metadata-only lint work, native build scripts, cold parallel misses and GHA backend request limits constrain reuse.
- Neither a cache below 10 GB nor faster hosted execution is claimed without measurement.
- Runner-issued GHA capabilities are exported by a minimal repository-owned JavaScript action without a post hook. Missing capabilities disable remote caching; statistics authenticate the executable before execution.
- Shards still compile their selected instrumented targets. Native binaries are not silently reused as coverage binaries.
- Receipt-checked aggregation must reject missing, failed, duplicated, foreign, partial or corrupted shards.
- No change to Sonar exclusions, thresholds, warnings, billing, quotas, application behavior or SDK is authorized.
- ADR-0501 remains an unchanged retrospective record; this Proposed ADR is not an implicit acceptance or global quality pass.

## Rejected alternatives

- One build matrix: a dependency cannot select one matching child.
- Reusable producer workflows: unnecessary name/identity complexity at the bundle admission boundary.
- Combined build/test producers: delay E2E producer success until tests finish.
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

Hosted cold/warm/backend-unavailable qualification remains pending publication authorization.
