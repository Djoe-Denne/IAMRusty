---
id: STORY-003.03.01
title: Define bounded session revocation admission contract
status: In Progress
assignee: []
created_date: '2026-11-27T15:54:08Z'
updated_date: '2026-11-27T16:06:11Z'
labels:
  - iam
  - session-liveness
dependencies:
  - STORY-003.02
documentation:
  - docs/adr/0316-password-recovery-session-liveness.md
  - docs/contracts/password-recovery-session-liveness.md
  - docs/adr/0317-bounded-replica-session-admission.md
  - docs/contracts/password-recovery-replica-admission.md
parent_task_id: STORY-003.03
priority: high
type: spike
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Translate approved STORY-003 E3 v4 into an ADR and a testable contract. Evaluate a minimal PostgreSQL logical projection against the existing synchronous path. Preserve atomic reset and renewal semantics. Settle the commit-progress proof, freshness clock, fallback budget and scope of protected admission before writing dependent tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A reviewed ADR and contract specify the five-second maximum from reset commit, steady-state reads, proof expiry during failures and the behavior of budgeted IAM fallback.
- [ ] #2 The contract inventories supported direct, monolith and mesh admission and defines the checks required for sensitive routes, stale or restored data, JWT compatibility and both renew/reset orderings.
- [ ] #3 The contract identifies reusable PR #57 evidence and defines the failing integration and gateway assertions that must precede implementation.
<!-- AC:END -->
