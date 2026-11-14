# Telegraph CI repair — proportionate verification

## Scope and invariants
- Approved extension: fix the remaining Telegraph integration failure, publish and check CI. No weakened tests, fake authorization, blind retry, unrelated runtime change or new proof harness.
- Existing ADR0200–0202 boundaries: native Windows test binaries and real testcontainers infrastructure. Outgoing HTTP mocks only where already appropriate; real SQS remains real.
- One Cargo,24configured jobs, targetE. Preserve foreign AGENTS.md/wiki edits, SDK `nul`, and four preexisting containers.

## Discriminating evidence
1. Inspect CI43569896393/job132373537827 and the exact shared SQS initialization path. Separate setup failures from business-test failures.
2. Run unchanged `ops/tests/it-native.ps1 -Suite telegraph -TestBinary outbox_runtime_tests`; keep the fresh fixture IDs, logs and mandatory cleanup. A local PASS is not proof the CI issue is fixed.
3. Choose a correction only after a causal hypothesis and discriminating observation. If shared SDK methods change, inspect consumers and add the smallest regression at the affected boundary. A public structural API change or new harness requires arbitration.
4. Validate the changed scope with targeted unit/static checks and the real seven-case outbox runtime binary; add only risk-justified regressions. Use existing fixture tooling, no Kind.
5. Verify fmt/Clippy as applicable, integrate and publish sibling SDK before any pin update, publish AIForAll master with normal hooks, then check exact-SHA CI. Report partial results and remaining environment limitations honestly.

## Runtime evidence
- Before inventory `.tmp/telegraph-ci-repair-runtime-before.jsonl`: one foreign stopped SMTP container and three foreign running full-profile Kind nodes. Do not adopt or stop them; no global Desktop/WSL shutdown.
- Existing it-native lifecycle records newly created IDs and cleanup in daily append-only registries. No Docker resource has been created by the extension before the first probe.

## Observed gap and selected verification
- Unchanged real outbox binary passed7/7locally; CI cause remains intermittent. A separate deterministic regression proved that the old TCP+2s check accepts a listener returning HTTP503.
- The bounded private SDK correction requires successful ListQueues within30s before queue creation. It does not retry a failed business operation or restart the whole test. HTTP5xx/429 and transport unavailability may recover within the deadline; permanent/invalid HTTP responses fail.
- Five HTTP-boundary unit cases cover unavailable service,503→429→validSQS recovery with SDK retries disabled in the test client,permanent403 rejection,invalid200response, and stalled-request deadline. These mocks do not establish authentication or real LocalStack readiness.
- Run full native SDK lib tests withfull,testing(noKafkaWindows),fmt and Clippy; publish SDK before pinning; then rerun the same seven-case real outbox binary against the pin and check exact-SHA CI.
- The first real probe created8containers,removed7andleft0running. One task-owned MailHog exited2 is retained for diagnosis/explicit removal arbitration. Never report total cleanup as complete while it remains.
- Final cleanup was explicitly authorized for the two stopped MailHog fixtures
  from the baseline and corrective campaigns. Diagnostics were archived, all
  registry holds checked, and both exact IDs removed without force or volumes.
  Combined final result:16created/16removed/0ownedrunning; four foreign resources
  preserved. The corrected pinned seven-case binary passed with zero filtering.
