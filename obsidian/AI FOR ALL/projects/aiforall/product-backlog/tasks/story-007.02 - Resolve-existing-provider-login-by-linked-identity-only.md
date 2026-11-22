---
id: STORY-007.02
title: Resolve existing provider login by linked identity only
status: Accepted intent
assignee: []
created_date: '2026-11-22T15:54:25Z'
labels:
  - iam
  - backend-alignment
dependencies:
  - STORY-007.01
documentation:
  - docs/guides/product-delivery-workflow.md
parent_task_id: STORY-007
priority: high
type: bug
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
STORY-007 E1 to E3 and STORY-006 E3 to E4. High value, medium effort after the federation contract. Apply the agreed intent boundary to existing provider login. Reuse stable provider identity lookup and provider authentication. No email-based association, new MFA, or client path. Preserve existing links and provider boundaries.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An already linked identity signs in to the same account when provider email is absent or different, without changing addresses or links.
- [ ] #2 Ordinary provider login without a link opens no session and creates no account or link even when the email matches.
- [ ] #3 Integration checks preserve no mutation on cancellation or refusal and verify provider identity scope.
<!-- AC:END -->
