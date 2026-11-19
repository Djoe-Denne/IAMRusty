# CI service-local execution contract

Status: Proposed. Parent: [ADR-0507](../adr/0507-ci-service-local-dag-shared-compiler-cache.md).
Authored by `architecte`; persisted by the implementing root agent.

## Boundary and publication hold

Change CI workflows, local actions, planner/shard/report scripts, focused tests and documentation only. Do not change application/SDK behavior, live-test opt-ins, Sonar policy, billing, quotas or remote required checks.

Commit/push, the announced native-v1 retirement and hosted qualification were explicitly authorized on 2026-11-18. This does not authorize changing external required checks, billing, quotas or selective-PR activation. Selective PR execution remains default off.

## DAG and inventory

The [IAM-only compile-once adoption contract](ci-iam-compile-once-adoption.md)
specializes the native IAM producer, IAM consumer compilation and current-CI
E2E input clauses below after the qualified pilot. Its successful-producer wait
now includes IAM test compilation/bundling. Other services, canonical coverage
policy and native/release/saved-image admission remain unchanged.

- Eight explicit build jobs retain baseline package selections and display names.
- Native builds use `cargo build --locked` with those unchanged package selections and default features. Cargo builds production libraries/binaries without redundantly precompiling test, benchmark and example targets. Existing coverage assignments/execution commands and both Clippy policies remain unchanged. This intentionally removes a separate native compile/link-only pass for non-production targets; Clippy is not equivalent link evidence.
- Each service coverage caller waits only for its build and the plan. Standalone readiness/shared coverage does not wait for an unrelated service.
- E2E waits only for its three native producers. Clippy waits for formatting, not builds or tests.
- Complete coverage plus complete Clippy reports qualify Sonar. The final summary distinguishes selected failures, expected skips and unevaluated full coverage.
- The versioned inventory records the 59 baseline metadata packages by ownership and policy. Inventory inclusion does not claim all packages are tested. Lazaret/operator/OAuthConnect/shared-crate gaps remain explicit.
- Master and default PRs select full declared execution. Opt-in PR selection uses reverse Cargo and conservative runtime-consumer edges. Manifest/lockfile/submodule/global/unknown changes or uncertain comparisons select full execution; new unregistered packages fail rather than disappear.
- Compare immutable PR base/head SHAs. Treat renamed paths as deletion plus addition. Manifest changes force full scope, including changes to the dependency graph.

## Coverage evidence

- Unit and integration test targets are disjoint, whole-binary assignments; never drop pure tests under `tests/`.
- Small deterministic integration shard counts: IAM 4, Telegraph 4, Hive 2, Manifesto 3. Telegraph's unit partition and original `tests/0` stay unchanged; its former `tests/1` becomes routing (`tests/1`), HTTP/outbox/common/mTLS/domain/application (`tests/2`) and email-verified (`tests/3`). Their observed baseline sizes are 17, 41 and 8 cases; these are not duration-optimal claims.
- Each shard has a private hosted runner/target directory, fixture run ID/ledger and profile accumulation. No test-case-level concurrency increase or nextest migration.
- Pin cargo-llvm-cov0.9.1; preserve its wrapper chaining and `--no-report` accumulation across selected package commands.
- Preserve every test/compiler failure even when report generation succeeds.
- A receipt binds source/run/attempt, inventory/toolchain digests, scope, service/lane/shard, features, assigned targets, outcome and LCOV digest.
- Validate identity, schema, scope, policy/toolchain, features, exact target assignments, unique partitions and report path/digest independently of the test verdict. Require correctly typed outcome fields; record nonzero test outcomes rather than rejecting valid report data.
- Keep strict report admission: missing/duplicate/foreign receipts, incompatible assignments, missing/empty/corrupt reports, nonzero `report_status`, malformed LCOV and merger failure make aggregation fail. Never silently omit a partition or publish a successful subset merge.
- A successful merge always publishes `coverage-diagnostic`, containing `diagnostic-lcov.info` and a source-bound `coverage-summary.json`. Record declared scope, expected/validated partitions, report completeness, every test/report outcome, failed partitions and the authoritative decision/reasons.
- Any nonzero test outcome makes execution completeness unproven and the merged report non-authoritative, even if every report is present. Cargo exit 101 can mean compilation failure or failed assertions; target assignments describe intended execution, not proof every target ran.
- Only full-scope, complete, successfully merged evidence with every test/report outcome successful additionally publishes canonical `lcov`. Expose this decision as a job output; Sonar requires it and verifies the canonical manifest/report without Cargo. Diagnostic/partial coverage cannot qualify Sonar.
- Keep test jobs unsuccessful on failure; no blanket `continue-on-error`. A verified diagnostic-only merge makes Sonar's skip deliberate, not a passed quality gate. Final status still fails on actual test failures, invalid aggregation and unexplained selected-job skips, without mislabeling successful diagnostic merging as a failed merge.
- Preserve Apparatus P0's separate baseline commands, including their doctest semantics.

## Compiler cache and native bundle v2

- All baseline compiling lanes share the pinned `.github/actions/setup-compiler-cache` action.
- The Cargo download cache has no target/bin archives and one trusted master writer. Compiler results use a stable shared GHA sccache namespace, not a run/SHA/service-specific directory snapshot.
- CI uses `CARGO_INCREMENTAL=0`; local Windows settings, 24 jobs and the single Cargo slot remain unchanged.
- Cache I/O fallback must still run the real compiler. Never swallow installation integrity errors or compiler failures. Record hit/miss/non-cacheable/error statistics.
- `.github/ci/sccache-policy.json` pins the release archive and reviewed executable SHA256. Authenticate archive bytes before parsing/extraction, install only the selected regular executable, verify its version/hash before PATH publication and recheck before collection.
- Do not use an installer that prepends unverified release contents or registers an unconditional executable post hook. The local credential bridge exports masked runner-issued cache capabilities without installing content; missing credentials disable remote caching.
- Select the supported github.com service protocol explicitly with `ACTIONS_CACHE_SERVICE_V2=true`; the pinned backend otherwise defaults to retired v1 even with the v2 results URL/token. Keep this service selector distinct from the `SCCACHE_GHA_VERSION` namespace.
- Statistics authenticate the resolved executable first. An unapproved executable is never invoked on failure or teardown.
- Native roles bind exactly: `iam → build-iam`, `oauth → build-oauth`, `ext-authz → build-ext-authz`. Preserve display names and all repository/workflow/source/SDK/attempt/artifact/ELF/package checks.
- V2 records actual wrapper settings and the version/platform/executable-digest tuple. Reject other wrappers, workspace wrappers, compiler flags, Cargo indirection, wrong role/job or unknown digest/version. Do not clear the wrapper to collect a bundle.
- New producers export the exact successful build argv as `K6_BUILD_ARGS_JSON`; do not synthesize an `--all-targets` command that did not run. Native-v2 consumers admit two closed forms: `build --locked` or historical `build --locked --all-targets`, followed only by valid package argument pairs containing every required output package. New producers emit only the production form; arbitrary flags, malformed args, missing outputs and foreign provenance remain rejected. No schema fields change and native-v1 remains rejected.
- Reject native v1 explicitly; do not silently rebuild or select another attempt. Existing saved-image admission remains unchanged.

## Measured warm-build qualification

- The IAM-only qualification workflow runs a seed build followed by a rebuild on a fresh hosted runner, using the same build action as the full CI. Both target directories start empty; transfer only compiler-cache results, never `target/` artifacts.
- Existing warmed caches may be read. Do not describe the seed job as a forced cold-cache experiment or delete caches to manufacture that envelope.
- Record source/run/attempt/job, exact Cargo command, outcome and Cargo elapsed time. Preserve failed builds. Flush pending cache writes through the authenticated pinned executable before admitting the second job.
- Evaluate Cargo time, setup/cache overhead and complete IAM job duration against comparable hosted baselines, including the latest 365-second IAM job from run44322588851. Cache-hit counts alone are not performance acceptance.
- Change target selection first. If per-object GHA traffic remains a demonstrated wall-time bottleneck, a later batch-cache implementation requires an explicit trusted publisher, validated compatible object deltas, conflict handling and no build/E2E prewarm dependency. It must not archive `target/`, delete warmed caches or claim that concurrent snapshots merge automatically.

## Clippy and acceptance

- Production and test/bench policies remain byte-for-argument equivalent to local aliases; no new `-D warnings`.
- Require valid complete JSONL, one final successful compiler record per lane, exact concatenation and current-source/policy/toolchain receipt.
- Sonar downloads and verifies both inputs; it does not install Rust or invoke Cargo.
- Validate the real YAML with actionlint, existing static CI/E2E refusal tests, Clippy parity and new planner/partition/evidence tests. Run separately named asynchronous commands with durable logs.
- Static tests do not prove hosted cache performance, coverage equivalence, Linux fixture health or backend-unavailable behavior. Those qualifications remain pending an authorized hosted run.
