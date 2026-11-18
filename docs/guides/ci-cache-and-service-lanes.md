# CI compiler caching and service-local test lanes

Implementation status: local review passed under [ADR-0507](../adr/0507-ci-service-local-dag-shared-compiler-cache.md); publication authorized on 2026-11-18, hosted qualification pending.

## What is shared

Every existing Rust build/test/lint lane installs the same pinned sccache release. The repository installer verifies the archive before parsing/extraction, exposes only the verified executable and registers no upstream executable post hook. Its GHA backend stores independently keyed compilation results, not one mutable archive that concurrent jobs must merge.

Keys distinguish sources, dependencies, compiler arguments, toolchain, target and features. Different coverage/feature variants remain different results. Local crates can be eligible for caching; linker-producing crates and metadata-only checks are not equivalent to cacheable libraries.

One registry-only Cargo download archive is shared by all lanes. Only the trusted push/master `build-iam` job saves it. It contains no `target/` or Cargo-bin archive. A cache miss remains a real build; installation/digest and compiler failures remain failures.

The credential bridge also exports `ACTIONS_CACHE_SERVICE_V2=true` for this github.com repository. With the pinned OpenDAL0.58.1 backend, `ACTIONS_RESULTS_URL` and the runtime token alone do not select the v2 protocol: an absent service flag selects retired v1 calls against the results endpoint and produces404/write failures. `SCCACHE_GHA_VERSION` remains the cache namespace, not the service-protocol selector. See [the pinned backend selection](https://github.com/apache/opendal/blob/v0.58.1/core/services/ghac/src/core.rs#L432-L459).

Do not clear caches between runs. Do not expect a cache hit for every invocation or claim less than 10 GB without measuring the hosted working set. Explicit local statistics steps authenticate the executable before invoking it. Missing runner cache capabilities disable remote caching instead of qualifying or failing the compiler result.

The workflow neither raises the repository quota nor configures billing. Existing `v0-rust-<service>-...` target archives are no longer refreshed by the new workflow. No remote caches were deleted during the initial local implementation. The user subsequently authorized an explicit repository-wide Actions cache reset before hosted qualification on 2026-11-18.

## Build and test dependencies

Eight named native build jobs replace the previous matrix barrier. A service's test caller starts after its own build, independent of unrelated builds. E2E waits only for IAM, OAuthConnect and Ext Authz producers.

Whole integration targets are deterministically partitioned: IAM4, Telegraph2, Hive2, Manifesto3. Each partition keeps normal Cargo/libtest execution; it does not increase test-case concurrency or remove serial fixture protections. Unit lib/bin targets run once in their own lane, while pure integration targets under `tests/` are preserved.

The explicit baseline inventory is `.github/ci/inventory.json`. "Full" means its declared coverage scope, not a claim that every workspace package is covered. The existing coverage gaps and opt-in live-test boundaries are recorded there.

## Evidence, not best-effort coverage

Each shard publishes LCOV and a receipt binding source/run/attempt, policy/toolchain, scope, assigned targets, result and report digest. Aggregation rejects missing, duplicate, partial, failed, foreign or corrupted evidence.

Only a complete full-scope aggregation emits canonical `lcov`. Empty-report fallbacks were removed. Clippy independently runs the same production/test policies as the local aliases, then publishes complete JSONL plus a verified receipt. Sonar imports these inputs without compiling.

`CI scope and result` fails on selected jobs that fail, are cancelled or are unexpectedly skipped. It distinguishes a deliberately unevaluated full Sonar gate from a passing gate.

## Selective PR execution is opt-in

Default behavior remains full coverage for both master and PRs.

`CI_SELECTIVE_PR_TESTS=true` is a repository variable reserved for a separately reviewed opt-in. When enabled, ordinary documentation-only PRs can skip Rust lanes; service source changes select conservative Cargo/runtime consumers. Unknown/global/shared/manifest/lockfile/submodule changes and comparison uncertainty select full execution.

Partial PR receipts are diagnostic. They cannot qualify canonical monorepo coverage or Sonar. Review required-check expectations before enabling the variable. This implementation changes no repository variables or branch protection.

## Native bundle migration and publication

Native E2E bundles use `aiforall.k6-binaries/v2`. The actual sccache wrapper, version and executable digest are retained and validated, not cleared during collection. Role/job bindings are closed:

- `iam`: `build-iam`, `Build (IAMRusty)`.
- `oauth`: `build-oauth`, `Build (OAuthConnect)`.
- `ext-authz`: `build-ext-authz`, `Build (Ext Authz)`.

The new native reader rejects v1 bundles. Already saved image admission is unchanged. The user authorized publication of the announced migration on 2026-11-18; external required-check configuration remains unchanged. Selective PR activation requires separate approval.

## Qualification still required

Local Python refusal/planner tests and actionlint validate wiring and deterministic contracts. They do not prove hosted cold/warm performance, GHA rate-limit behavior, real Linux fixture health, coverage equivalence or backend-unavailable behavior.

After publication is authorized, compare those cases on identical source/toolchain/feature selections before reporting performance gains. No local Docker, Kind or WSL startup is needed for the static checks.
