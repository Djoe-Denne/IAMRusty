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

## Hosted evidence

No new implementation run has completed at the initial publication checkpoint. Baseline run `44150614642` at the initial HEAD was successful but does not count toward this campaign.

Record each attempt, immutable source SHA, workflow conclusion, job-level failures, exact exception disposition, cache observations and final consecutive streak here after observing completion.

## Resources and closeout

No task branch/worktree/team or local Docker/Kind/WSL runtime has been created. Hosted execution is still pending; no speedup, storage reduction or remote green result is claimed.
