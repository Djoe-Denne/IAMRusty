---
id: STORY-003.03.03
title: Enforce replica freshness and bounded IAM fallback in ext-authz
status: Done
assignee: []
created_date: '2026-11-27T15:54:22Z'
updated_date: '2026-11-27T19:49:40Z'
labels:
  - iam
  - session-liveness
dependencies:
  - STORY-003.03.02
documentation:
  - docs/contracts/password-recovery-session-liveness.md
parent_task_id: STORY-003.03
priority: high
type: task
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Use the replicated session state for normal mesh admission without an IAM request per admission. Verify progress in the background. Keep the existing IAM checker as a budgeted fallback and fence stale data after a restart or recovery.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Deterministic tests with an injectable clock cover fresh, boundary, expired, unreachable and restored copies. Definitive denials and the healthy replica path do not invoke fallback.
- [x] #2 A stale copy triggers authoritative fallback within a capped budget, with concurrent lookups for the same principal shared. An unverifiable decision never admits or downgrades the request to anonymous.
- [x] #3 Capture the expected failing assertions first; use the same tests for green and cover recovery without overwriting newer state.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented opt-in PostgreSQL replica admission, strict proof age, dedicated committed IAM checkpoints, bounded exact-claim fallback, and fail-closed Check handling. Preserved the observed failing tests and their green reruns. Native verification passed for gateway policy and HTTP, IAM checkpoint guards, and real PostgreSQL 15 checkpoint and retention behavior. Deployed cutoff qualification remains STORY-003.03.04.
<!-- SECTION:FINAL_SUMMARY:END -->
