---
id: STORY-005.02
title: Preserve verification proof on secondary provider email import
status: Accepted intent
assignee: []
created_date: '2026-11-22T15:53:50Z'
labels:
  - iam
  - backend-alignment
dependencies: []
documentation:
  - docs/guides/product-delivery-workflow.md
parent_task_id: STORY-005
priority: medium
type: bug
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
STORY-005 E3 and E6. Medium value, small effort. The provider link service imports new secondary addresses as unverified even with a trusted exact-address assertion. Reuse the existing connector proof; do not add a proof source or change the primary address, prior methods, or relink behavior. Evidence: services/IAMRusty/domain/src/service/provider_link_service.rs lines 168 to 239 and services/IAMRusty/tests/oauth_browser_transactions.rs lines 1266 to 1294 at f9fb748d.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A newly imported eligible secondary address is verified only when the provider attests verification of that exact address.
- [ ] #2 Missing or false proof keeps the new secondary address unverified. The primary address and existing methods remain unchanged.
- [ ] #3 Integration checks cover both outcomes and preserve the no-mutation outcome on a refused link.
<!-- AC:END -->
