# CI cache reset and hosted qualification

## Goal and authorization

Continue [the reviewed CI implementation](20261117-ci-cache-critical-path.md) on `master`. On 2026-11-18 the user authorized commit/push, deletion of all repository GitHub Actions caches and continued corrections until two consecutive hosted CI executions qualify.

The announced native-v1 to v2 migration is authorized for publication. ADR-0507 remains Proposed; operational publication does not imply architectural acceptance.

## Success criterion

- Execute the complete declared baseline CI scope; leave selective PR execution disabled.
- Require two consecutive complete executions of the final implementation, not two selected job retries.
- Telegraph integration-test failures are excluded from the user's qualification criterion; Telegraph build and unit failures, infrastructure failures and invalid reports are not excluded.
- Keep excluded test outcomes visible. Record the actual GitHub workflow conclusion separately from any qualification with this exception.
- Never describe skipped dependent gates as executed passes or failed/partial reports as authoritative coverage.
- Do not change billing, quotas, external required checks, local caches or Docker caches.

## Initial state and validation reuse

Initial application HEAD: `f8ddff4f0e958b024d59b528e6e465a777cdcfd7`; SDK: `1db6c9de8708f6451d91a56f482b053026487c8f`. Repository: `Djoe-Denne/IAMRusty`, default branch `master`.

The existing CI implementation has not changed since its final local validations: 20 planner/cache/report tests passed; CI/E2E static suites had 132 passes and one skip; Clippy parity had eight passes; actionlint and the real metadata/partition audit passed. These are reused rather than rerun for documentation-only authorization updates.

The broad local Ops suite's Windows PKI ACL failure remains a separate non-green result; no unrelated correction or claim of pre-existing failure is made.

## Cache reset

Authorized scope: all GitHub Actions caches of this repository, across refs. Source files, workflow artifacts, local Cargo caches and Docker caches are outside scope.

Full before/after inventories and deletion command logs are retained in ignored `.tmp/ci-cache-20261118/`. The authorized reset removed **18 caches totaling 11,218,257,942 bytes**; the post-reset JSON inventory contains zero entries.

The wrapper initially reported an incorrect count because PowerShell retained a nested JSON array as one pipeline object. Independent strict JSON parsing confirmed the successful deletion and empty result; the wrapper counting was corrected without repeating deletion. Cache space freed at reset is not a measurement of the replacement's steady-state footprint.

## Published implementation and first hosted execution

Commit `a3c7916b30cf41539e87c447805611f2d42574d6` was published to `origin/master` with its normal formatting hook. Run [44274785092, attempt1](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44274785092/attempts/1) completed `failure`, with41 of45 jobs successful.

The only failed test was Telegraph integration `test_user_email_verified_event_creates_database_notification`, at `services/Telegraph/tests/user_email_verified_event_test.rs:83` (no notification after its wait). Telegraph build/unit/other shard and all other service coverage, Clippy and E2E jobs passed. The failed coverage receipt correctly blocked merging; Sonar was skipped and the final status reported those facts. The user-excluded test failure is not represented as a green GitHub workflow or an evaluated Sonar pass.

Separately, the cache was not working: IAM recorded993 misses/993 write errors/zero hits; Clippy recorded1207 misses/1207 write errors/zero hits. The after-run inventory contained only the145,140,019-byte Cargo download archive. Correct compilation fallback did not qualify remote caching.

## Cache protocol diagnosis and correction

A temporary branch `ci-cache-probe-20261118`, without another worktree, isolated the diagnosis from the expensive full pipeline.

- Probe [44279607855](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44279607855), commit `82dd2570ba582dab393d1c50899cccae5bb73a97`: two compilations, two misses, two write errors. Credential-redacted daemon errors showed404 on the backend read/write check.
- Pinned source verification: sccache0.19.0 uses OpenDAL0.58.1. The [service selector](https://github.com/apache/opendal/blob/v0.58.1/core/services/ghac/src/core.rs#L432-L459) chooses V1 unless `ACTIONS_CACHE_SERVICE_V2` is nonempty. The bridge had exported the URL/token but omitted that flag.
- Correction `dd4574e16a0bc8a7f649cf5c223a7a8f75bd304a`: export `ACTIONS_CACHE_SERVICE_V2=true` for this github.com repository; preserve the missing-capability local fallback, masking and token scope. Added a Node subprocess regression with no inherited service flag.
- Local correction checks:21 pipeline/cache/report tests passed; actionlint passed; the normal formatting commit hook passed.
- Corrected probe [44280204339](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44280204339): first daemon had one miss and zero write errors; after stopping it and removing the object, the new daemon had one real remote hit, zero misses and zero write errors. This proves persistent write/read reuse for the probe, not a full-run hit-rate guarantee.

The temporary workflow is removed before master integration. Both probe commits and the correction are retained in the integrated history; no unverified binary or raw daemon log is delivered.

## Final full-run qualification

The corrected full implementation still requires two consecutive completed executions. Baseline run `44150614642` and diagnostic probes do not count. The initial all-write-error execution does not qualify the final corrected cache implementation.

Record immutable source, attempts, actual conclusions and any Telegraph-only exception after observing the final executions. No successful Sonar run or final streak is claimed yet.

## Resources and closeout

No task branch/worktree/team or local Docker/Kind/WSL runtime has been created. Hosted execution is still pending; no speedup, storage reduction or remote green result is claimed.
