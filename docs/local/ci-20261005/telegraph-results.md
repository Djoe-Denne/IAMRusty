# Telegraph local reproduction — 2026-10-05

Scope: notification_business_logic_test only first; no IAM/Hive/SDK edits.
HEAD observed: 87a6a10fa20445983a24535c4e6775def745a5ac (briefing SHA stale; investigation pointers verified against saved failed log).
Exclusive parent Cargo lease: compiler 493026f9bfc97cf2a64e768f4d7ab198875c4b72d62f8f7bb84fe0fe320ad805, host network, /workspace and /linux-target. Before job: sleep infinity only.
Run ID: telegraph-20261005-183954. Fixture ledger scope: /evidence/telegraph-20261005-183954; parent root ledger untouched. Fixture ownership derives only from returned created IDs. Compiler retained for parent.
Evidence: parent temp ci-20261005/telegraph-20261005-183954; test output repro.log; inventory before.txt; fixture JSONL and exact-ID observations.
Active jobs: targeted reproduction `sh_10cf00fdd001G41cGbobk1EayK`; bounded exact-ID fixture observer `sh_10cf0da9a001L07dYXq49XMnvx` (10-minute observation ceiling). Logs remain in the evidence scope. No service change justified yet.
Source pointers reverified: SDK TCP-only readiness at sqs_testcontainer.rs:151–188 and singleton publication at :730 precede protocol initialization; no causal conclusion from those pointers alone.
Exact CI build package list confirmed in ci.yml:51–60/111; coverage command at :283 matches investigation handoff. Compiler currently lacks cargo-llvm-cov, so coverage instrumentation needs parent tooling authorization if reproduction progresses that far.
The requested evidence markdown currently appears untracked, not ignored; it will not be staged/committed.

## Targeted result

Exact handoff command passed: 1 passed, 0 failed, 4 filtered, 23.28s, exit 0. No assertion/source changes.
Observed startup gap: host-side TCP true at 16:42:11Z while SQS.ListQueues connection closed; at 16:42:13Z TCP true but API timeout; API HTTP 200 at 16:42:18Z. This verifies TCP is not SQS readiness locally, not the root cause of the CI failure. Harness queue initialization succeeded.
Owned returned IDs (all running with OOM=false after process exit; all gracefully stopped before next run):
- OpenFGA `8fdf85e6d0bcb74ad6980861f6a4430cb014ddd11c48b655f2eca0764c7549e6`, mapped 44847.
- PostgreSQL `92b533c572e83aa06701b2b6580d5285001392b7207342cc7f34ce57862697e9`, mapped 37275.
- SQS `963f1599a7d64e7a2fc97155a7dba058b9186e1dac5afeccde964ac54955e03d`, mapped 43857.
Next: whole binary, unique run ID `telegraph-20261005-binary01`, same fixture ledger directory. Compiler idle verified after targeted cleanup.

## Whole-binary result

Handoff whole-binary command passed: 5 passed, 0 failed, 27.20s, exit 0. Serial/single-thread reproduction flags are diagnostic only, not a source fix or CI configuration change. No unknown fixture events. All fixtures running/OOM=false at completion, then gracefully stopped:
- OpenFGA `799f1d81376ad6b8fb2c119cd5e5077bd7299115e3fc0f8813f49ab6867b68ce` (45835).
- PostgreSQL `3948234913dd59af014ebe527ac32b43599f200153f33556f7701acd597705d1` (39779).
- SQS `d6803c96a7bf4095892273f40bfff9a7f837d2b11d2ba4c251a30e35e2b90fd3` (36191).
Both fixture observers completed; no permanent monitor.
Exact Telegraph matrix build started: `sh_10cf447af001gsVjuRb576SZSo`, build.log/build.exit. Coverage tooling downloaded from taiki-e cargo-llvm-cov official latest release into the evidence mount and installed only inside compiler; llvm-tools-preview added. No project dependencies or SDK sources changed.

Matrix build completed PASS (exit 0, 1m31s): `cargo build --locked --all-targets --package telegraph-service --package telegraph-domain --package telegraph-application --package telegraph-infra --package telegraph-http_server --package telegraph-setup --package telegraph-configuration`.
Tooling verified: cargo-llvm-cov 0.9.1; rustup installed llvm-tools-x86_64-unknown-linux-gnu. Archive SHA256 b3f68e625481fed9b16444174f3fa5ecdbde4a1878803a35eabe2dcefcdc41a. Initial direct binary `--version` syntax rejected; correct `cargo-llvm-cov llvm-cov --version` passed, no reinstall/retry loop.
All six previous fixture IDs verified Exited and compiler idle before coverage. Coverage uses unique run `telegraph-20261005-cover01`, exact CI flags with default test threading; outer timeout 20 minutes (TERM only, no forced kill). Observer bounded 21 minutes and ends on coverage exit marker. No IAM package is selected.
Active coverage job `sh_10cf68fb00010o1XrC7m23iCIM`; observer `sh_10cf6a683001ywozxA5MuMV7BK`. Exclusive Cargo lease retained until completion/idle verification. Next bounded task upon automatic notification: summarize coverage result, reconcile returned fixture IDs, capture states/logs, gracefully stop remaining owned fixtures, return idle compiler to parent.

## Final verification / handback

All listed jobs and observers COMPLETED. Exact CI coverage invocation (seven Telegraph packages, `--locked --no-report --no-fail-fast --tests`, no thread override) PASS exit 0: **50 passed / 0 failed / 0 ignored**, 18 test-target summaries; instrumentation compilation 5m10s. Business-logic binary under coverage: 5 passed / 0 failed, 27.81s. No SQS setup failure reproduced.
`cargo llvm-cov report --lcov --output-path /evidence/telegraph-20261005-183954/lcov-integration-telegraph.info` PASS exit 0; artifact 428580 bytes. Kept in evidence rather than project root.

Coverage returned 24 ledger-proven OpenFGA/PostgreSQL/SQS IDs, all running/OOM=false after tests; all gracefully stopped, final Exited confirmed in `coverage-final-states.txt` (complete exact-ID list). No UNKNOWN ledger events across all three runs. Six earlier ledger IDs also Exited. The normal process-exit harness did not stop these singleton fixtures; no SDK lifecycle changes attempted.

SMTP is not covered by that SDK ledger. Reconciliation of immutable Docker create-response Id/Warnings records in the three test logs found 35 SMTP returned IDs: 34 absent after normal teardown, one leaked alive. Ownership of the latter was proven by coverage.log:12759 (create response), :12761 (start), :12773 (testcontainers ready), not the name. Exact ID `7b004a95a3adddd504975688128a6a97b24044fd33bc96d40304e8efc7678651` recorded in `smtp-owned.json`, then gracefully stopped and verified Exited/OOM=false (exit 2 from MailHog stop). No foreign resource stopped; no prune/deletion commands issued by this worker.

Final running inventory is EXACTLY retained compiler `493026f9bfc97cf2a64e768f4d7ab198875c4b72d62f8f7bb84fe0fe320ad805` and the two unchanged foreign Kind IDs supplied by parent. Compiler `docker top` after coverage/report/cleanup: sleep infinity ONLY; Cargo lease released to parent. All observers completed; no background task remains owned here.

**Conclusion:** cannot reproduce CI SQS dispatch/readiness failure locally, including exact coverage with default threading. TCP-before-SQS readiness gap verified locally, but CI crash/cache/readiness causality remains cross-environment UNKNOWN. No causal service fix established; no service, SDK, pool, Borrow, threading, or assertion edits are justified. Next arbitration: parent may approve instrumented CI startup capture and an architecte-reviewed SDK protocol-readiness/success-only singleton proposal; this work does not authorize it.

Files changed by this worker: this evidence markdown only in repository (untracked, NOT ignored as requested); temporary run-scoped observer and supplementary SMTP ownership evidence outside repo. Telegraph and SDK diff empty. No manifests/lock/keys/IAM/Hive edits, no commit/push.
Docker Desktop/WSL retained under parent lease and foreign/protected workloads; no global runtime shutdown. RAM delta not measured.

### User-requested final handoff revalidation

Re-read existing results via Context Mode only; no new Cargo/tests/tool installation/rerun. repro/binary/build/coverage/report exit markers all 0; coverage 18 summaries, 50 passed, 0 failed. Exact-ID inspection confirms all 30 SDK ledger fixtures plus the supplemental leaked SMTP ID Exited (31 stopped IDs); 34 other SMTP IDs previously verified absent after normal teardown. Three observer-finished records match completed tool notifications; coverage job `sh_10cf68fb00010o1XrC7m23iCIM` and observer `sh_10cf6a683001ywozxA5MuMV7BK` are completed.
Compiler top reverified sleep infinity only, no Cargo/rustc. Cargo slot RELEASED. Compiler remains parent-leased, not worker-owned cleanup scope; parent must stop it if no explicit active follow-up lease exists (idle/open session is not retention justification). No new fixture cleanup needed.
Remaining Telegraph task: CI-environment causal localization, since all local reproduction gates passed and no cause/fix was established. Any instrumented CI rerun or SDK readiness/cache patch needs fresh parent authorization and applicable architectural/publication gates.
