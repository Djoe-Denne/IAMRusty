# CI compiler caching and service-local test lanes

<!-- CI scope probe: classify a documentation-only change. -->

Implementation status: published under [ADR-0507](../adr/0507-ci-service-local-dag-shared-compiler-cache.md). Two earlier full hosted executions met the user's criterion excluding Telegraph integration tests, but their GitHub conclusions remained red and did not qualify Sonar. See [those historical results](../missions/20261118-ci-cache-hosted-qualification.md). A subsequent [diagnostic run](../missions/20261118-sccache-backend-errors.md) qualified canonical coverage and Sonar with all Telegraph partitions successful; its overall workflow remained red because of an unrelated report deployment failure. Cache throttling and reservation conflicts remain unresolved.

The published [coverage-outcome and Telegraph partition amendment](../missions/20261118-ci-coverage-outcomes-and-telegraph-shards.md) has hosted proof: real diagnostic merging succeeds despite failed Telegraph tests. The earlier two-run results apply to their recorded source/layout. Finer partitioning is deployed but did not eliminate Telegraph intermittency on the observed run.

## What is shared

Every existing Rust build/test/lint lane installs the same pinned sccache release. The repository installer verifies the archive before parsing/extraction, exposes only the verified executable and registers no upstream executable post hook. Its GHA backend stores independently keyed compilation results, not one mutable archive that concurrent jobs must merge.

Keys distinguish sources, dependencies, compiler arguments, toolchain, target and features. Different coverage/feature variants remain different results. Local crates can be eligible for caching; linker-producing crates and metadata-only checks are not equivalent to cacheable libraries.

One registry-only Cargo download archive is shared by all lanes. Only the trusted push/master `build-iam` job saves it. It contains no `target/` or Cargo-bin archive. A cache miss remains a real build; installation/digest and compiler failures remain failures.

The credential bridge also exports `ACTIONS_CACHE_SERVICE_V2=true` for this github.com repository. With the pinned OpenDAL0.58.1 backend, `ACTIONS_RESULTS_URL` and the runtime token alone do not select the v2 protocol: an absent service flag selects retired v1 calls against the results endpoint and produces404/write failures. `SCCACHE_GHA_VERSION` remains the cache namespace, not the service-protocol selector. See [the pinned backend selection](https://github.com/apache/opendal/blob/v0.58.1/core/services/ghac/src/core.rs#L432-L459).

Do not clear caches between runs. Do not expect a cache hit for every invocation or claim less than 10 GB without measuring the hosted working set. Explicit local statistics steps authenticate the executable before invoking it. Missing runner cache capabilities disable remote caching instead of qualifying or failing the compiler result.

### Backend error diagnostics

The IAM build captures daemon diagnostics for sccache 0.19.0 using only server debug and compiler error logging. The raw log stays in the hosted runner's temporary directory and is never printed or uploaded: backend errors can include scoped credentials or signed URLs.

`ops/ci/cache_diagnostics.py` publishes only fixed classification labels and numeric counts in `iam-cache-diagnostics` (three-day retention). It distinguishes remote lookup and asynchronous write failures, OpenDAL error kinds, HTTP status codes and selected transport signals. Unknown failures remain counted; no arbitrary error text is echoed. Tests cover secret non-disclosure and artifact wiring.

In this sccache version, remote lookup failures can increment **Cache errors** even when the separately displayed **Cache read errors** counter is zero. Treat the counters as implementation-specific signals, not disjoint failed-compilation totals. Diagnostic capture changes no cache namespace, quota, build selection or test verdict.

The [hosted backend investigation](../missions/20261118-sccache-backend-errors.md) reproduced HTTP 429 throttling on lookups and writes, plus HTTP 409 duplicate entry reservations. Cache reuse works, but clean backend operation remains unqualified; no traffic-control or retry-policy fix has been applied.

The workflow neither raises the repository quota nor configures billing. Existing `v0-rust-<service>-...` target archives are no longer refreshed by the new workflow. No remote caches were deleted during the initial local implementation. The user subsequently authorized an explicit repository-wide Actions cache reset before hosted qualification on 2026-11-18.

## Build and test dependencies

Eight named native build jobs replace the previous matrix barrier. A service's test caller starts after its own build, independent of unrelated builds. E2E waits only for IAM, OAuthConnect and Ext Authz producers.

Native builds now compile production libraries/binaries using `cargo build --locked` with the unchanged package lists. They no longer compile all test targets before separate coverage jobs compile them again. IAM previously selected53integration-test binaries in that redundant pass; no test execution or coverage assignment is removed. Native-v2 bundles record the actual successful command, while consumers retain explicit historical all-targets compatibility.

The manual `IAM warm-cache qualification` workflow builds IAM, flushes compiler-cache writes, then rebuilds IAM on a fresh runner. It transfers no target artifacts and publishes separate Cargo timings; complete job timelines include setup/cache overhead. The first job may read existing caches and is not a forced cold-cache baseline. Reserved branch `ci-iam-cache-qualification` also triggers this focused workflow without the full master CI. The [first measured pair](../missions/20261118-ci-warm-build-time.md#hosted-qualification-result) reduced Cargo time from 82 to 48 seconds, while slower system dependency setup made the second job longer overall.

Whole integration targets are deterministically partitioned: IAM4, Telegraph4, Hive2, Manifesto3. Telegraph preserves its unit and original first integration partition; the former second integration partition becomes routing17, other41 and email-verified8 baseline cases. Each partition keeps normal Cargo/libtest execution; it does not increase test-case concurrency or remove serial fixture protections. Unit lib/bin targets run once in their own lane, while pure integration targets under `tests/` are preserved.

The explicit baseline inventory is `.github/ci/inventory.json`. "Full" means its declared coverage scope, not a claim that every workspace package is covered. The existing coverage gaps and opt-in live-test boundaries are recorded there.

## Evidence, not best-effort coverage

Each shard publishes LCOV and a receipt binding source/run/attempt, policy/toolchain, scope, assigned targets, result and report digest. Aggregation rejects missing, duplicate, partial, foreign or corrupted evidence and failed report generation. A failed test command alone no longer prevents valid report data from merging.

Every successful merge publishes `coverage-diagnostic`: `diagnostic-lcov.info` plus `coverage-summary.json`, with explicit partition outcomes and qualification reasons. When any test command fails, this artifact remains diagnostic and execution completeness is unproven. Test jobs stay red.

Only full-scope, complete valid evidence with all test/report outcomes successful additionally emits canonical `lcov`. Sonar requires the authoritative job decision and verifies that artifact; otherwise it is deliberately skipped, not passed. Empty-report fallbacks remain forbidden. Clippy runs its unchanged independent policies, and Sonar imports without Cargo.

For example, `test_status=101` with `report_status=0` now allows diagnostic merging instead of triggering the old generic rejection. This does not assert that exit101 was an assertion-only failure, nor does it qualify authoritative coverage. Identity, partition, report-generation and digest failures remain blocking and receive explicit diagnostics.

`CI scope and result` fails on selected jobs that fail, are cancelled or are unexpectedly skipped. It distinguishes a deliberately unevaluated full Sonar gate from a passing gate.

## Selective PR execution is opt-in

`master` and non-documentation PRs run the complete baseline by default. A PR changing only regular Markdown files at `README.md` or directly under `docs/adr/`, `docs/contracts/`, `docs/guides/`, or `docs/missions/` runs only the scope planner and `CI scope and result`. Unproven merge ancestry, mixed changes, and unexpected paths or file modes do not qualify.

`CI_SELECTIVE_PR_TESTS=true` is a repository variable reserved for a separately reviewed opt-in. When enabled, ordinary documentation-only PRs can skip Rust lanes; service source changes select conservative Cargo/runtime consumers. Unknown/global/shared/manifest/lockfile/submodule changes and comparison uncertainty select full execution.

Partial PR receipts are diagnostic. They cannot qualify canonical monorepo coverage or Sonar. Review required-check expectations before enabling the variable. This implementation changes no repository variables or branch protection.

For other PRs, the advisory scope step previews separate native build and integration lists with reasons; it does not drive job conditions. It compares the exact tested PR merge to its base parent and defaults to complete scope on uncertain input. Cargo build dependencies expand the build list; declared runtime consumers expand the integration list instead. Workflow changes, manifests, lockfiles and unclassified inputs remain full-scope candidates.

Full-execution PRs produce fresh LCOV and Clippy reports for the tested merge. The PR quality summary records changed Rust lines found in LCOV and reports unmapped lines separately; they are not counted as covered or assumed to be executable. The `pr-quality` artifact is evidence for this run, not a cache. Sonar PR analysis uses its new-code metrics and imports only Clippy warnings whose primary location overlaps the PR diff. The scanner runs on the PR head only if its tree is identical to the tested merge and the PR can access scanner credentials. Otherwise, the run reports that Sonar PR analysis was not evaluated. Full `master` analysis keeps the unfiltered reports.

## Native bundle migration and publication

Native E2E bundles use `aiforall.k6-binaries/v2`. The actual sccache wrapper, version and executable digest are retained and validated, not cleared during collection. Role/job bindings are closed:

- `iam`: `build-iam`, `Build (IAMRusty)`.
- `oauth`: `build-oauth`, `Build (OAuthConnect)`.
- `ext-authz`: `build-ext-authz`, `Build (Ext Authz)`.

The new native reader rejects v1 bundles. Already saved image admission is unchanged. The user authorized publication of the announced migration on 2026-11-18; external required-check configuration remains unchanged. Selective PR activation requires separate approval.

## Qualification still required

Local Python refusal/planner tests and actionlint validate wiring and deterministic contracts. They do not prove hosted cold/warm performance, GHA rate-limit behavior, real Linux fixture health, coverage equivalence or backend-unavailable behavior.

Hosted persistent cache write/read and compiler fallback have now been observed, including reuse on a second full execution. Full coverage equivalence, successful Sonar import, unavailable-backend qualification and the cause of remaining write errors are still open. Do not infer steady-state storage/time gains from the current snapshots. No local Docker, Kind or WSL startup is needed for the static checks.
