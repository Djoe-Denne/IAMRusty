---
id: STORY-020
title: Simplify k6 E2E orchestration and remove duplicated test verdicts
status: Accepted intent
assignee: []
created_date: '2026-11-22T18:35:45Z'
labels:
  - technical
  - e2e
  - k6
  - ci
  - to-be-refined
dependencies: []
references:
  - ops/tests/k6/ci/campaign.py
  - ops/tests/k6/ci/evidence.py
  - ops/tests/k6/ci/reporting.py
  - ops/tests/k6/native/campaign.ps1
  - ops/tests/k6/iam/campaign.ps1
  - ops/tests/support/oauth-idp-simulator.py
  - .github/workflows/k6-e2e.yml
documentation:
  - docs/guides/ops-declarative-boundaries.md
  - docs/contracts/k6-three-scenario-native-parity.md
  - docs/contracts/k6-three-scenario-ubuntu-ci.md
priority: high
type: task
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## To be refined

High-priority standalone engineering task. Reduce the cost of adding or changing a k6 end-to-end scenario without weakening the existing CI or infrastructure guarantees. No implementation or design choice is approved yet.

## Current wiring and evidence

- Linux: the post-merge GitHub Actions workflow `.github/workflows/k6-e2e.yml` invokes `ops/tests/k6/ci/campaign.py run` and an independent `clean` step. The parent owns disposable Kind, image provenance, Helm rendering, readiness, private inputs, forwards, the k6 process, and scoped cleanup. Kubernetes workload definitions already come from Helm; do not replace them with a second imperative deployment implementation.
- Windows: `ops/tests/k6/native/campaign.ps1` is a fixed health / OAuth Chromium / TLS SAN campaign against the existing local-full lab. It is not the launcher that starts Kind. `ops/tests/k6/iam/campaign.ps1` is a separate stateless IAM path. They are not interchangeable with the hosted Linux campaign.
- k6 already decides the functional result: `check()` records an observation; a threshold such as `checks: [rate==1]` makes a failed check return nonzero. The JavaScript scenarios also enforce start/completion and, where applicable, request counts. `ci/evidence.py` requires exit zero, a fresh run-bound summary, exact check names and counts, and external correlations. `ci/reporting.py` and the workflow publish an allowlisted result. A nonzero k6 result cannot become a pass.
- The recurring friction is multiple registrations of ordinary scenarios: script dispatch and sequence in `campaign.py`, named-check expectations in `evidence.py`, and artifact publication in the workflow. A second handwritten check list catches an accidentally omitted assertion, but it must be updated with the JS. Keep the distinction between functional failure, incomplete execution, infrastructure failure, and cleanup failure.
- The TLS SAN test is not an ordinary JS invocation: the runner changes a certificate between phases and must restore it even on failure. Keep that lifecycle in an explicitly separate case rather than forcing it into the common path.
- Pilot finding: the Windows campaign routed business traffic to a run-owned IAM but reused shared ext-authz, whose JWKS source was nominal IAM. The nominal Service had no ready endpoint, so k6 never started. Attempts with a run-local verifier progressed to fixture deployment but still stopped during IAM startup. Owned resources and dedicated databases were removed. Offline Helm and harness checks passed; deployed E2E was not proved. The extra Windows profile is not a reference implementation.
- The Python OAuth simulator is a protocol fixture, not an orchestrator. It previously supplied one identity; distinguishing identity conflict from address conflict requires selectable, run-scoped synthetic identities. Keep fixture settings declarative and scenario behavior in JS. Detailed persistence checks already supplied by the affected integration tests need not be duplicated in every E2E harness.

## Refinement questions

1. Can the hosted Linux runner remain the reference, with one checked catalogue for ordinary k6 scenarios and one source of truth for their verdict? Compare reusing the existing scenario registry with a small manifest; do not design a new deployment or test framework.
2. Which guarantees must remain outside k6: correct image and target, scenario completeness, infrastructure correlations, cancellation and cleanup, and sanitized reporting? Prove that a failed or omitted assertion, missing or stale result, and failed cleanup cannot go green before removing the duplicated oracle.
3. Is native Windows E2E still required? The current Python harness explicitly admits hosted Linux and depends on the Unix Docker socket, `/proc`, POSIX permissions, and Linux Chromium isolation. Windows handles certificate trust, private ACLs, and process trees differently. Compare Linux-only E2E, an explicitly admitted disposable Linux environment for local use, and a shared Python core with small OS adapters if native Windows is necessary. Do not translate PowerShell into a second Python runner.
4. What can be migrated or retired after checking all consumers, proving default health/OAuth/SAN parity, and verifying clean failure paths? Keep Windows-native Rust unit/integration tests separate.

## Non-goals

Do not change the deliberate post-merge E2E policy, PR merge gates, nominal shared lab configuration, test security checks, or deploy another pre-merge runner as part of this task. Do not publish credentials or private run artifacts.
<!-- SECTION:DESCRIPTION:END -->
