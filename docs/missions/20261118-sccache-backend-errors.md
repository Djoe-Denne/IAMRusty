# IAM compiler-cache backend errors

## Scope and outcome

The user requested investigation of the 353 cache write errors and 360 cache errors in [IAM job 134819382948](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44309352880/job/134819382948). Its ordinary log proved cache reuse but did not retain daemon backend responses.

A representative full-workflow run reproduced two failure modes: **GitHub cache service throttling (HTTP 429)** and **existing-entry reservation conflicts (HTTP 409)**. This establishes the reproduced failures, not an exact retrospective breakdown of the old job.

No cache reset, namespace change, quota/billing modification, application/SDK edit or speculative backend remediation was performed.

## Diagnostic implementation and validation

Published commit `ab399939bf2099e20b90524a81a7c9b48843ef32` adds IAM-only daemon logging and `ops/ci/cache_diagnostics.py`. The raw log remains in the hosted runner's temporary directory and is never printed or uploaded. Only fixed labels and numeric classification counts are published.

- Four diagnostic tests passed, including multiline errors, unknown failures, secret non-disclosure, bounded events and artifact wiring.
- All 32 existing pipeline regressions passed.
- Actionlint and diff checks passed; the normal formatting commit hook passed.
- No local application build or Docker/Kind/WSL runtime was started.

## Hosted evidence

Run [44318107962](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44318107962), attempt 1, used the exact diagnostic commit. Its [IAM build job 134850290323](https://github.com/Djoe-Denne/IAMRusty/actions/runs/44318107962/job/134850290323) succeeded.

Authenticated artifact `iam-cache-diagnostics` (`14434257347`) contains one bounded JSON summary. The GitHub API/archive SHA-256 digest and member layout were verified. It reports 765 error events with no truncated events:

- **387 lookup errors:** all HTTP 429, OpenDAL `RateLimited`, operation `GetCacheEntryDownloadURL`.
- **378 write errors:**
  - 343 HTTP 429, `RateLimited`, operation `CreateCacheEntry`;
  - 35 HTTP 409, `AlreadyExists`, operation `CreateCacheEntry`.

The 378 write events exactly match the final `Cache write errors` counter. The lookup-event count and `Cache errors` counter are not identical: the latter was 385. Do not force equality or add event counters to hit/miss counts as disjoint compilation failures.

Final compiler-cache statistics were 579 hits, 414 misses, 58.31% overall hit rate, and 192 Rust hits at 33.51%. `Cache read errors` remained zero. The unchanged GHA namespace was `f8659dc1b7c8bb13939c4bc4a444baafdb4eb47f2ec1830028ad3fbee78fc36a`.

Evidence remains under ignored `.tmp/ci-cache-20261118/`: exact run/job metadata, job log, artifact API metadata, authenticated archive and sanitized summary. No raw daemon log was downloaded.

## Interpretation

**P1 — request throttling:** GitHub rejected both lookup and create-entry traffic with HTTP 429. This prevents existing cached results from being reused and new results from being retained, although compilation can still succeed. GitHub documents repository-wide rates of up to 200 cache uploads and 1,500 downloads per minute, with reset guidance in `Retry-After`. These documented limits provide context; the diagnostic summary does not measure the entire repository's request rate or retain response headers.

**P2 — existing-entry reservation conflicts:** HTTP 409/`AlreadyExists` means the immutable cache entry is already reserved or present. Parallel writers can cause this; a throttled lookup can also trigger compilation and an attempted write of a previously cached result. The sanitized evidence does not distinguish these explanations. This is redundant cache work, not evidence of a compiler-output hash collision or corrupt data.

The pinned sccache 0.19.0 source explains the misleading counters: `server.rs` increments `cache_errors` for `MissType::CacheReadError`, while asynchronous write failures increment `cache_write_errors`. The separately displayed `cache_read_errors` field is not that remote-lookup failure counter.

There was no 401, 403, 404 or 5xx classification among the captured error events. The reproduced errors are not the previously fixed cache-v1 protocol-selection 404 defect.

## Recommended next step, not applied

Bound repository-wide cache traffic and duplicate writes across concurrent jobs, then qualify hit rate and wall-clock performance under the same workload. Any retry/backoff change should respect server `Retry-After`; adding retries alone without controlling offered load can amplify throttling.

Treat immutable-entry HTTP 409 as an existing-entry reservation conflict rather than a storage-integrity failure where the backend contract permits it. Do not implement this by suppressing every write error.

Cache deletion or new per-job namespaces would not remove a repository-wide request-rate limit. No writer-policy, concurrency, retry, backend or architecture change is included in this investigation.

## Separate workflow observations

All Telegraph partitions, Merge coverage and SonarCloud succeeded in the diagnostic run. Authenticated `lcov` (`14435500343`) and `coverage-diagnostic` (`14433589566`) both recorded 21 partitions, 193 targets, complete reports, no failed partitions and `authoritative=true`.

The overall workflow remained unsuccessful because `Publish sanitized E2E report` failed at `Deploy public report and badge`, with final status also failing. That publisher failure is separate from cache diagnostics and was not investigated or fixed here. One passing Telegraph observation does not establish that intermittency is eliminated.

## Sources

- [Pinned sccache counter/write handling](https://github.com/mozilla/sccache/blob/v0.19.0/src/server.rs)
- [Pinned sccache lookup-error handling](https://github.com/mozilla/sccache/blob/v0.19.0/src/compiler/compiler.rs)
- [Upstream daemon debugging guidance](https://github.com/mozilla/sccache/tree/v0.19.0#debugging)
- [OpenDAL 0.58.1 GHA status classification](https://github.com/apache/opendal/blob/v0.58.1/core/services/ghac/src/core.rs)
- [GitHub cache usage limits](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#usage-limits-and-eviction-policy)
