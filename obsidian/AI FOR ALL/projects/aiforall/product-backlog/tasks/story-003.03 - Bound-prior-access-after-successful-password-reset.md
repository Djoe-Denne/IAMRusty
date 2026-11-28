---
id: STORY-003.03
title: Bound prior access after successful password reset
status: Done
assignee: []
created_date: '2026-11-22T15:54:52Z'
updated_date: '2026-11-28T13:00:58Z'
labels:
  - iam
  - backend-alignment
dependencies:
  - STORY-003.02
documentation:
  - docs/guides/product-delivery-workflow.md
  - docs/adr/0316-password-recovery-session-liveness.md
  - docs/contracts/password-recovery-session-liveness.md
  - docs/missions/20261128-password-recovery-replica-qualification.md
parent_task_id: STORY-003
priority: high
type: bug
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Backend scope for STORY-003 E3 (approved v4) and STORY-010 E2. PR #57 already shipped the durable IAM session epoch, the reset and renewal transaction changes, direct IAM enforcement, and an optional synchronous IAM check at the mesh gateway. Reuse that foundation. The replacement admission design must keep IAM off the normal gateway request path and satisfy the five-second limit across supported direct and mesh paths. Preserve signed-in password change and reset confirmation semantics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A successful link reset prevents earlier sessions from renewing from commit. An earlier unexpired platform access token cannot obtain new protected access on any supported path after five seconds from commit, including during replication failure.
- [x] #2 An anonymous reset request does not revoke sessions; successful confirmation creates no session. If neither copy nor IAM provides a sufficiently fresh decision, admission fails temporarily without granting access.
- [ ] #3 Evidence covers the direct and deployed mesh paths, concurrent renewal, legacy tokens, and both normal admission from the copy and the bounded fallback. Normal mesh admission does not call IAM per request.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Closed at the owner's request with the [hosted single-worker evidence](https://github.com/Djoe-Denne/IAMRusty/actions/runs/46144607978) in [PR #62](https://github.com/Djoe-Denne/IAMRusty/pull/62). The five-second bound was observed on the tested paths. This closure does not qualify aggregate fallback under load or authorize mesh activation. The separate reset-email task STORY-003.01 remains open.
<!-- SECTION:FINAL_SUMMARY:END -->
