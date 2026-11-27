---
id: STORY-003.03.02
title: Replicate IAM session epochs for admission
status: Done
assignee: []
created_date: '2026-11-27T15:54:15Z'
updated_date: '2026-11-27T17:58:24Z'
labels:
  - iam
  - session-liveness
dependencies:
  - STORY-003.03.01
documentation:
  - docs/contracts/password-recovery-session-liveness.md
  - docs/contracts/password-recovery-replica-admission.md
parent_task_id: STORY-003.03
priority: high
type: task
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the approved state replication boundary. With the PostgreSQL logical replication option, publish only the minimum session state from the same IAM transaction as password recovery; provision and bootstrap the writable source and read copy reproducibly. Start with a failing integration check using a real primary and subscriber.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A committed reset advances the state on the copy; a failed reset does not advance it. A read-only admission role cannot query unrelated IAM data or write session state.
- [x] #2 Initial synchronization, interrupted application and catch-up are observable; a reachable copy with an unapplied source checkpoint cannot qualify as current.
- [x] #3 Record the expected failing integration assertion before production changes and rerun the same assertion after the implementation.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented the PostgreSQL 15 transactional IAM projection and explicit publication and subscriber bootstrap SQL. Native integration proved the missing projection before production changes, then proved commit, rollback, paused apply, catch-up, existing-account backfill, deletion, downgrade, and read-only subscriber permissions. The gateway freshness decision and deployed qualification remain separate tasks.
<!-- SECTION:FINAL_SUMMARY:END -->
