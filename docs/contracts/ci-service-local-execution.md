# CI service-local execution contract

Status: Proposed. Parent: [ADR-0507](../adr/0507-ci-service-local-dag-shared-compiler-cache.md).
Authored by `architecte`; persisted by the implementing root agent.

## Boundary and publication hold

Change CI workflows, local actions, planner/shard/report scripts, focused tests and documentation only. Do not change application/SDK behavior, live-test opt-ins, Sonar policy, billing, quotas or remote required checks.

Commit/push, the announced native-v1 retirement and hosted qualification were explicitly authorized on 2026-11-18. This does not authorize changing external required checks, billing, quotas or selective-PR activation. Selective PR execution remains default off.

## DAG and inventory

- Eight explicit build jobs retain baseline package selections and display names.
- Each service coverage caller waits only for its build and the plan. Standalone readiness/shared coverage does not wait for an unrelated service.
- E2E waits only for its three native producers. Clippy waits for formatting, not builds or tests.
- Complete coverage plus complete Clippy reports qualify Sonar. The final summary distinguishes selected failures, expected skips and unevaluated full coverage.
- The versioned inventory records the 59 baseline metadata packages by ownership and policy. Inventory inclusion does not claim all packages are tested. Lazaret/operator/OAuthConnect/shared-crate gaps remain explicit.
- Master and default PRs select full declared execution. Opt-in PR selection uses reverse Cargo and conservative runtime-consumer edges. Manifest/lockfile/submodule/global/unknown changes or uncertain comparisons select full execution; new unregistered packages fail rather than disappear.
- Compare immutable PR base/head SHAs. Treat renamed paths as deletion plus addition. Manifest changes force full scope, including changes to the dependency graph.

## Coverage evidence

- Unit and integration test targets are disjoint, whole-binary assignments; never drop pure tests under `tests/`.
- Small deterministic shard counts: IAM 4, Telegraph 2, Hive 2, Manifesto 3. These are initial partitions, not duration-optimal claims.
- Each shard has a private hosted runner/target directory, fixture run ID/ledger and profile accumulation. No test-case-level concurrency increase or nextest migration.
- Pin cargo-llvm-cov0.9.1; preserve its wrapper chaining and `--no-report` accumulation across selected package commands.
- Preserve every test/compiler failure even when report generation succeeds.
- A receipt binds source/run/attempt, inventory/toolchain digests, scope, service/lane/shard, features, assigned targets, outcome and LCOV digest.
- Aggregation admits exactly the expected complete, disjoint, successful, compatible full-scope receipt/report set. Missing/extra/failed/partial/corrupted evidence fails.
- Only this aggregation emits canonical `lcov.info`/`lcov`. Partial PR outputs are diagnostic, with explicit partial receipts; they do not qualify Sonar.
- Preserve Apparatus P0's separate baseline commands, including their doctest semantics.

## Compiler cache and native bundle v2

- All baseline compiling lanes share the pinned `.github/actions/setup-compiler-cache` action.
- The Cargo download cache has no target/bin archives and one trusted master writer. Compiler results use a stable shared GHA sccache namespace, not a run/SHA/service-specific directory snapshot.
- CI uses `CARGO_INCREMENTAL=0`; local Windows settings, 24 jobs and the single Cargo slot remain unchanged.
- Cache I/O fallback must still run the real compiler. Never swallow installation integrity errors or compiler failures. Record hit/miss/non-cacheable/error statistics.
- `.github/ci/sccache-policy.json` pins the release archive and reviewed executable SHA256. Authenticate archive bytes before parsing/extraction, install only the selected regular executable, verify its version/hash before PATH publication and recheck before collection.
- Do not use an installer that prepends unverified release contents or registers an unconditional executable post hook. The local credential bridge exports masked runner-issued cache capabilities without installing content; missing credentials disable remote caching.
- Statistics authenticate the resolved executable first. An unapproved executable is never invoked on failure or teardown.
- Native roles bind exactly: `iam → build-iam`, `oauth → build-oauth`, `ext-authz → build-ext-authz`. Preserve display names and all repository/workflow/source/SDK/attempt/artifact/ELF/package checks.
- V2 records actual wrapper settings and the version/platform/executable-digest tuple. Reject other wrappers, workspace wrappers, compiler flags, Cargo indirection, wrong role/job or unknown digest/version. Do not clear the wrapper to collect a bundle.
- Reject native v1 explicitly; do not silently rebuild or select another attempt. Existing saved-image admission remains unchanged.

## Clippy and acceptance

- Production and test/bench policies remain byte-for-argument equivalent to local aliases; no new `-D warnings`.
- Require valid complete JSONL, one final successful compiler record per lane, exact concatenation and current-source/policy/toolchain receipt.
- Sonar downloads and verifies both inputs; it does not install Rust or invoke Cargo.
- Validate the real YAML with actionlint, existing static CI/E2E refusal tests, Clippy parity and new planner/partition/evidence tests. Run separately named asynchronous commands with durable logs.
- Static tests do not prove hosted cache performance, coverage equivalence, Linux fixture health or backend-unavailable behavior. Those qualifications remain pending an authorized hosted run.
