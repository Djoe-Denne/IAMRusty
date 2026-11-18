# Measured warm-build correction

## User objective

Reduce real service build wall time, especially IAM's approximately six-minute job. Cache error counters and added diagnostics are not the deliverable. The user also requested a focused CI that builds IAM, populates the cache and rebuilds IAM from it.

Application/SDK code, package lists, all coverage/test assignments, Clippy policy, billing and quotas remain unchanged. Commit/push is authorized. Do not reset warmed caches.

## Baseline

The latest comparable full run, `44322588851` at `35be6fb15f2f456c1f2b2efcee607c8c30114c5c`, completed IAM job `134865404091` in **365 seconds**. Cargo itself reported **4m22s**; the remaining job time includes setup, cache access, native packaging/statistics and post steps.

The earlier diagnostic run `44318107962` reported **4m35s** of IAM Cargo time. These observations show runner/cache variability and must remain separate historical measurements.

Native metadata shows that `--all-targets` selected 53 IAM integration-test programs in the native build, before coverage jobs separately compiled instrumented tests. Equivalent redundant integration targets were selected for Telegraph18, Hive26, Manifesto54, Ext Authz2 and OAuth1.

## Correction

- All eight service builds now use `cargo build --locked` with exactly their existing packages, compiling production libraries/binaries rather than precompiling test/bench/example targets.
- No actual test execution or coverage assignment is removed. The complete coverage baseline remains 193 targets across21partitions.
- Native-v2 producers record the exact successful Cargo argv. Consumers deliberately accept either the new production form or the historical all-targets form through a closed package-only grammar. All source/role/job/wrapper/instrumentation/ABI/output checks remain; native-v1 stays rejected.
- The architecture amendment was authored by the existing `architecte` session and persisted in ADR-0507 and its execution contract. The ADR remains Proposed.

## Focused qualification

`IAM warm-cache qualification` runs a seed build followed by a rebuild on a fresh hosted runner, using the same build-service action as the full CI. Both target directories must start empty; the build driver refuses timing qualification if target artifacts already exist.

The first job flushes pending cache writes using the authenticated pinned executable before the second job starts. Only timing JSON is uploaded; no target directory, executable or raw daemon log is transferred.

Timing records bind source/run/attempt/job, exact Cargo command, elapsed time and compiler exit status. The workflow reports both Cargo durations; complete job timelines include setup/cache costs. Existing caches are retained, so the first build is not a forced cold-cache benchmark.

The workflow supports manual dispatch. Reserved branch `ci-iam-cache-qualification` also runs the focused workflow without triggering the full master CI. All durable changes must subsequently be integrated/published on master and the temporary branch removed.

## Validation before hosted qualification

- 34 pipeline/command/provenance tests passed.
- 135 existing CI/E2E seam tests completed:134passed,1opt-in skip.
- Three fresh-target qualification/timing/refusal tests passed.
- Actionlint passed for the main CI and focused qualification workflow.
- No local application compilation or Docker/Kind/WSL runtime was started.

Hosted elapsed-time improvement is not claimed before the seed/rebuild measurements. If remaining per-object GHA cache traffic still materially delays the real build, assess a bounded batch-cache correction rather than declaring success from hit counters.
