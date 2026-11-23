---
id: STORY-020
title: Simplify k6 E2E orchestration and remove duplicated test verdicts
status: Done
assignee: []
created_date: '2026-11-22T18:35:45Z'
updated_date: '2026-11-23T16:00:22Z'
labels:
  - technical
  - e2e
  - k6
  - ci
dependencies: []
references:
  - ops/tests/k6/ci/campaign.py
  - ops/tests/k6/ci/evidence.py
  - ops/tests/k6/ci/reporting.py
  - ops/tests/support/oauth-idp-simulator.py
  - .github/workflows/k6-e2e.yml
  - ops/tests/k6/scenarios/catalog.json
  - .github/workflows/k6-e2e-pr.yml
documentation:
  - docs/guides/ops-declarative-boundaries.md
  - docs/contracts/k6-three-scenario-native-parity.md
  - docs/contracts/k6-three-scenario-ubuntu-ci.md
  - docs/contracts/k6-scenario-registration.md
  - ops/tests/k6/ci/README.md
priority: high
type: task
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
An AIForAll contributor can add and run a standard k6 E2E scenario in the post-merge CI campaign and on a prepared local target using the same JavaScript, without maintaining equivalent Python and PowerShell scenario logic.

Use standard k6 for scenario setup, fixture data and cleanup through existing test APIs, test behavior, and functional verdicts. Prefer a simple, deliberate discovery convention when practical; otherwise, one declarative registration is sufficient. Keep infrastructure definitions declarative and external orchestration thin. A new scenario requires changes to Python only when it needs a new setup capability that k6 and the existing tooling cannot safely provide. No particular adapter design is required.

Preserve the existing guarantees for the correct test target and image, isolation, complete run-bound evidence, cancellation, scoped cleanup, and sanitized reporting. Keep the post-merge policy, PR gates, and Windows-native Rust unit and integration tests unchanged.

The approved delivery scope retires the Windows k6 adapters and uses the separate hosted PR campaign for full qualification. The same standard script can run directly on a prepared Linux or WSL target. Browser startup or a synthetic HTTPS pass is not application OAuth qualification under WSL. Native Windows Rust unit and integration tests are unchanged.

## Original wiring and pilot evidence (before delivery)

- Linux: the post-merge GitHub Actions workflow `.github/workflows/k6-e2e.yml` invokes `ops/tests/k6/ci/campaign.py run` and an independent `clean` step. The parent owns disposable Kind, image provenance, Helm rendering, readiness, private inputs, forwards, the k6 process, and scoped cleanup. Kubernetes workload definitions already come from Helm; do not replace them with a second imperative deployment implementation.
- Windows: `ops/tests/k6/native/campaign.ps1` is a fixed health / OAuth Chromium / TLS SAN campaign against the existing local-full lab. It is not the launcher that starts Kind. `ops/tests/k6/iam/campaign.ps1` is a separate stateless IAM path. They are not interchangeable with the hosted Linux campaign.
- k6 already decides the functional result: `check()` records an observation; a threshold such as `checks: [rate==1]` makes a failed check return nonzero. The JavaScript scenarios also enforce start/completion and, where applicable, request counts. `ci/evidence.py` requires exit zero, a fresh run-bound summary, exact check names and counts, and external correlations. `ci/reporting.py` and the workflow publish an allowlisted result. A nonzero k6 result cannot become a pass.
- The recurring friction is multiple registrations of ordinary scenarios: script dispatch and sequence in `campaign.py`, named-check expectations in `evidence.py`, and artifact publication in the workflow. A second handwritten check list catches an accidentally omitted assertion, but it must be updated with the JS. Keep the distinction between functional failure, incomplete execution, infrastructure failure, and cleanup failure.
- The TLS SAN test is not an ordinary JS invocation: the runner changes a certificate between phases and must restore it even on failure. Keep that lifecycle in an explicitly separate case rather than forcing it into the common path.
- Pilot finding: the Windows campaign routed business traffic to a run-owned IAM but reused shared ext-authz, whose JWKS source was nominal IAM. The nominal Service had no ready endpoint, so k6 never started. Attempts with a run-local verifier progressed to fixture deployment but still stopped during IAM startup. Owned resources and dedicated databases were removed. Offline Helm and harness checks passed; deployed E2E was not proved. The extra Windows profile is not a reference implementation.
- The Python OAuth simulator is a protocol fixture, not an orchestrator. It previously supplied one identity; distinguishing identity conflict from address conflict requires selectable, run-scoped synthetic identities. Keep fixture settings declarative and scenario behavior in JS. Detailed persistence checks already supplied by the affected integration tests need not be duplicated in every E2E harness.

## Non-goals

Do not change the deliberate post-merge E2E policy, required PR gates, shared lab configuration, or test security checks. The separate opt-in PR workflow reuses the same hosted campaign; it does not introduce another scenario runner. Do not publish credentials or private run artifacts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 E1: Given a standard scenario that creates its data through an existing API, adding its k6 script and at most one declarative registration makes the applicable post-merge campaign execute it and report its result without changes to Python, PowerShell, GitHub Actions, or a second scenario list.
- [x] #2 E2: Given a prepared, authorized, isolated local target and the documented inputs, the same script runs its normal setup and assertions with `k6 run`, without a Python or PowerShell launcher.
- [x] #3 E3: A failing or unexecuted expected assertion, an absent or stale result, or failed required cleanup cannot produce a green campaign. The report distinguishes functional, incomplete, infrastructure, and cleanup failures without duplicating functional expectations in Python or publishing private inputs or raw evidence.
- [x] #4 E4: If `setup()` creates test data and then fails, the run fails and does not rely on `teardown()` to remove the data: setup compensates, or the isolated run environment is safely disposed of.
- [x] #5 E5: If a scenario changes a server certificate for the TLS SAN test, restoration remains required even when the scenario fails. The special lifecycle does not become a generic script hook for ordinary scenarios.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Delivered in [PR #44](https://github.com/Djoe-Denne/IAMRusty/pull/44), merged on `master` at `b7b4fc235dd68aa888ba48d7bef4cb05b1ca412b`. The merged tree is identical to tested PR merge `eab01004dced1deb69fca4c07119c20613541f32`.

- E1: The catalogue extension probe added only one script and one record. The dispatch and reporting regressions verify selection, exact counts and reporting without a second registration.
- E2 and E4: Real `k6 run` against an owned synthetic API proves direct setup, checks and teardown. Injected setup failure returns nonzero and compensates the partial write without teardown.
- E3: Negative tests cover missing, duplicate and false assertions, stale or absent output, and failed cleanup. The hosted failure remained red when child evidence was absent; the final run verifies complete evidence and cleanup.
- E5: Failure-path tests keep SAN restoration mandatory. The [live qualification](https://github.com/Djoe-Denne/IAMRusty/actions/runs/45207940864) passed all 34 checks, real browser OAuth with the sandbox observed, SAN restoration and independent cleanup with no owned resources left running. It used [the PR CI builds](https://github.com/Djoe-Denne/IAMRusty/actions/runs/45204623316) from attempt 1; `CI scope and result` passed.

The `/k6-e2e` listener is installed on `master` for future open internal PRs. Live evidence above is from the equivalent pre-merge manual dispatch, not a comment event. The retired Windows k6 launchers do not affect Rust tests. The WSL synthetic verification did not exercise live application OAuth.
<!-- SECTION:FINAL_SUMMARY:END -->
