# CI service-local execution — ADR-0507

- Milestone: platform CI / IAM compile-once. Canon: `docs/adr/0507-ci-service-local-dag-shared-compiler-cache.md`; Status: Proposed; Reality: Partial. See the ADR file for the complete decision and historical qualifications.
- Service-local DAG, pinned authenticated shared sccache, outcome-preserving diagnostic aggregation and canonical/Sonar admission are established; cache HTTP 429/409 issues remain unresolved. No global performance qualification or formal acceptance.
- User-authorized IAM-only adoption follows five separate pilot test replays (44351218515) and five E2E scenarios, restoration and cleanup PASS (44493607389 attempt 1, source aba26931ff01de2149587275b5a928ba74194d73). These are not a combined canonical run.
- Contract: `docs/contracts/ci-iam-compile-once-adoption.md`. One IAM producer target shares compatible instrumented units, retains build-iam / Build (IAMRusty), freezes server/migration before test compilation, and feeds canonical whole-binary replay/fixtures/receipts. Other services are unchanged.
- Explicit typed current-CI instrumented IAM admission preserves exact provenance/success checks; native v2, release and saved-image admission remain non-instrumented. E2E profiles stay private and out of coverage.
- IAM coverage and E2E wait for complete producer test compilation/bundling; measure this new critical path. Complete adopted hosted CI, full coverage/Clippy/Sonar/final gate and E2E/cleanup remain required; no speedup or Implemented claim yet.
