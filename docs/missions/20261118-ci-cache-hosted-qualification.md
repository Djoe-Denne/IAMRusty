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
- Local correction checks: 21 pipeline/cache/report tests passed; actionlint passed; the normal formatting commit hook passed.
- Corrected probe [44280204339](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44280204339): first daemon had one miss and zero write errors; after stopping it and removing the object, the new daemon had one real remote hit, zero misses and zero write errors. This proves persistent write/read reuse for the probe, not a full-run hit-rate guarantee.

The temporary workflow is removed before master integration. Both probe commits and the correction are retained in the integrated history; no unverified binary or raw daemon log is delivered.

## Final full-run qualification

The user's criterion is met by **two consecutive complete executions outside Telegraph integration tests**, both at `4eeadab436aef179c823feb9cef276ba821d9513`:

- [Run 44280690285, attempt 1](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44280690285/attempts/1): 40 of 45 jobs successful. Telegraph integration assertions failed in both test shards. Signup email-content/happy-path and email-verified notification/different-users assertions failed. All builds, unit tests, other service integration tests, standalone gates, Clippy and E2E passed.
- [Run 44280690285, attempt 2](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44280690285/attempts/2): 40 of 45 jobs successful. The full workflow was rerun, not selected jobs. Telegraph signup event-support/email-content/happy-path and email-verified multiple-events/notification assertions failed. The same non-exempt gates passed.

**Both actual GitHub conclusions are `failure`, not `success`.** The coverage merger correctly rejected failed Telegraph receipts, the final scope/status job reported the dependent failures, and Sonar was skipped in both attempts. Those downstream gates are not represented as evaluated passes. No failed receipt was changed, ignored by aggregation or imported as authoritative coverage.

Baseline run `44150614642`, diagnostic probes and the initial all-write-error run do not count toward this final-source streak. The evidence establishes the explicitly narrowed user criterion, **not two unconditionally green GitHub CIs**, coverage equivalence or successful hosted Sonar import.

## Observed cache behavior and remaining limitation

- After corrected attempt 1: 1,818 entries / 2,341,395,409 bytes across master and the tiny probe ref. IAM recorded 63 hits, 930 misses, 930 write errors and zero read errors; hit rate 6.34%.
- After complete attempt 2: 2,526 entries / 3,801,614,429 bytes across refs, including only 571 bytes from the temporary probe. IAM recorded 236 hits, 757 misses, 757 write errors and zero read errors; hit rate 23.77%.
- Clippy attempt 2 recorded 72 hits, 1,135 misses, 1,135 write errors and zero read errors; hit rate 5.97%.
- The master cache population grew by 708 entries between the corrected runs, and real remote reuse was observed. The missing-v2-selector defect is fixed.
- Remaining write errors have **not** been causally classified. Do not label them benign collisions, rate limits or quota failures without additional evidence. Builds retained correct compiler fallback; neither clean writes for all jobs nor steady-state storage/time savings are established.

Snapshots and exact job/attempt logs are retained under ignored `.tmp/ci-cache-20261118/`; the protocol regression suite's 21-pass log is under `.tmp/ci-cache-20261117/cache-v2-protocol-tests.stderr.log`.

## Resources and closeout

The temporary probe branch was fully integrated by fast-forward, its workflow removed, and its local/remote branch refs deleted. Its two cache entries (`10716303268`, `10716302171`, 571 bytes total) were removed by ID after verifying their probe-only ref; the post-cleanup API inventory confirms zero probe-ref entries. New master compiler/download caches are intentionally retained; the initial repository-wide reset is not repeated.

Only the canonical `master` worktree remains. The SDK is unchanged. No team, additional worktree or local Docker/Kind/WSL runtime was created. Hosted runner reclamation is GitHub-managed; no claim of independently inventoried remote Docker cleanup is made.

The original report publication at `4bb6c291` was documentation-only relative to the twice-executed candidate. The subsequent diagnostic correction below is separate and does not turn either recorded red workflow into a green result.

## Follow-up: exact merge rejection diagnosis

The user requested verification that the merge error was not independent of Telegraph. For run `44288096995`, attempt 1, all 19 real coverage archives were downloaded and verified against their API SHA256 digests. Production-rule replay and field-by-field comparison found exactly one mismatch: `lcov-telegraph-tests-1.json` has `test_status=101`, while `report_status=0`. All expected partitions, identities, policy/toolchain hashes, features, targets and report hashes match.

The observed rejection is therefore the Telegraph test outcome, before invoking the LCOV executable. No independent merger defect was established in this execution. The previous generic message obscured that distinction.

A diagnostic-only correction names the exact receipt/mismatching fields and safe numeric outcomes, without accepting failed/partial evidence. Two discriminating regressions were added; all 23 focused pipeline/cache/report tests passed. Replay of the unchanged real artifacts now gives `Rejected coverage receipt 'lcov-telegraph-tests-1.json': test_status (test_status=101, report_status=0)`. Full merge/Sonar success remains unclaimed until every required shard qualifies.
