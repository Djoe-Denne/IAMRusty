---
id: STORY-003.03
title: Invalidate prior access immediately after successful password reset
status: Accepted intent
assignee: []
created_date: '2026-11-22T15:54:52Z'
labels:
  - iam
  - backend-alignment
dependencies:
  - STORY-003.02
documentation:
  - docs/guides/product-delivery-workflow.md
parent_task_id: STORY-003
priority: high
type: bug
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
STORY-003 E3 and STORY-010 E2, reset boundary only. High value, large effort depending on the approved session contract. Reuse the existing reset transaction and refresh purge where applicable. Verify actual protected access on the direct and mesh paths in scope. Preserve no session issuance on reset and existing signed-in password change semantics. No logout, browser binding, or new factor flow.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Successful link reset immediately ends earlier protected access and renewal across devices without waiting for access expiry.
- [ ] #2 Anonymous reset requests do not revoke access. Successful reset creates no new session.
- [ ] #3 Integration evidence checks the affected access boundaries and concurrent renewal as specified by the contract.
<!-- AC:END -->
