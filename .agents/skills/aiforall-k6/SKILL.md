---
name: aiforall-k6
description: Author, run, and diagnose AIForAll functional k6 E2E tests. Load for scenario registration, API fixtures, direct Linux or WSL runs, GitHub PR comment qualification, the hosted Kind campaign, Chromium OAuth, SAN restoration, or k6 evidence and cleanup. This skill does not cover Rust unit or integration tests.
---

# AIForAll k6: fixture-first E2E

## Choose the execution boundary

- **Direct scenario:** On Linux or WSL, run the same script with `k6 run` against
  an already prepared, authorized, isolated target. Use the target receipt and
  fresh private summary path in the [scenario guide](../../../ops/tests/k6/scenarios/README.md).
  This command does not start a lab, rotate certificates, or prove full E2E.
- **Complete pre-merge qualification:** After the ordinary PR builds have
  published all three artifact bundles, an authorized repository writer
  comments `/k6-e2e` on an internal PR targeting `master`. The separate GitHub
  Action runs the full catalogue and cleanup on a disposable Ubuntu Kind
  runner. It refuses stale, missing, expired, or incompatible artifacts; it
  never recompiles applications or substitutes master binaries.
- **Automatic E2E:** Only on `push/master`, from that CI attempt's own builds.
  PR E2E does not publish Pages or replace `CI scope and result`.

See the [hosted runbook](../../../ops/tests/k6/ci/README.md) for the exact
producer, supported manual preview, reports, and failure behavior. The
native Windows k6 wrappers are retired. A WSL browser smoke is not a live
OAuth result. Do not revive a retired launcher or start a local runtime for
documentation. Load `local-runtime-lifecycle` and `aiforall-test-evidence`
before real execution; this skill grants no additional authorization.

## Author one standard scenario

Read the [registration contract](../../../docs/contracts/k6-scenario-registration.md).
Identify the real API, authorization boundary, initial state, expected effects,
negative oracle, and safe inverse before sending stimuli. A standard scenario
needs one k6 script and one record in `scenarios/catalog.json`, not a second
registration, Python or PowerShell verdict, or workflow edit.

- Use one VU and iteration, named checks, `group` in `systemTags`, and the
  `scenarioContract` thresholds and summary. Counts must prove execution,
  not only passes. Use real API fixture setup and teardown, with run-scoped
  identities. Compensate partial writes in failing setup; teardown cannot
  rescue it. If cleanup cannot be proven, only an owned disposable target can
  contain the remaining effects.
- Keep the hosted chart in `ops/tests/k6/ci/chart/` and use existing declared
  backing fixtures. A database alone does not isolate queues, events, or mail.
  The protocol IdP stands in for the external provider, not IAM or OAuthConnect.
  Do not seed credentials or records to bypass the pathway being tested.
- Keep Python focused on what k6 cannot safely manage: target admission,
  provisioning, owned process and resource custody, private inputs, evidence
  correlation, and cleanup. OAuth's sandbox and SAN rotation and restoration
  remain explicit lifecycles. Do not discover arbitrary JavaScript files.

## Receive a verdict, not just an exit code

Require the exact revision and producer attempt, fresh receipts, named check
counts, causal observations, explicit SAN restoration, and independent
cleanup. Report incomplete execution, functional failure, infrastructure
failure, and cleanup failure separately. Do not count a skip, synthetic probe,
packaging run, or WSL smoke as live E2E. Publish only allowlisted projections;
never raw k6/browser logs, private inputs, keys, or OAuth data.
