---
id: STORY-005.01
title: Report the correct explicit provider-link conflict
status: Accepted intent
assignee: []
created_date: '2026-11-22T15:53:41Z'
updated_date: '2026-11-22T15:55:09Z'
labels:
  - iam
  - backend-alignment
dependencies: []
references:
  - services/IAMRusty/application/src/command/provider.rs
  - services/IAMRusty/http/src/error.rs
documentation:
  - docs/guides/product-delivery-workflow.md
parent_task_id: STORY-005
priority: high
type: bug
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
STORY-005 E4 and E5. High value, small effort. The f9fb748d review found that the command and HTTP error mappings misclassify two existing refusals. Preserve rejection without mutation. Confirm the public error contract and consumer compatibility before tests. No ownership, login, relink, or dependency changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An unavailable email produces the address-specific refusal without identifying another account.
- [ ] #2 An identity already linked elsewhere produces the identity-specific refusal without identifying another account.
- [ ] #3 Integration checks verify that both refusals preserve addresses and sign-in methods on both accounts.
<!-- AC:END -->
