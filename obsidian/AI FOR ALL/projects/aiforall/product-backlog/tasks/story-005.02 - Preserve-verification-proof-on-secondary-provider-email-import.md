---
id: STORY-005.02
title: Preserve verification proof on secondary provider email import
status: Done
assignee: []
created_date: '2026-11-22T15:53:50Z'
updated_date: '2026-11-24T11:22:56Z'
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
- [x] #1 A newly imported eligible secondary address is verified only when the provider attests verification of that exact address.
- [x] #2 Missing or false proof keeps the new secondary address unverified. The primary address and existing methods remain unchanged.
- [x] #3 Integration checks cover both outcomes and preserve the no-mutation outcome on a refused link.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Reuse the existing authenticated connector profile in the explicit LINK import only. The provider email and its attestation travel as one pair; a missing wire assertion defaults to false. Preserve ownership checks, existing addresses and methods, and RELINK. Verify the persisted secondary flag through the IAM HTTP/Postgres callback harness (true, false, and omitted) and keep the existing conflict no-mutation checks.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Native Windows HTTP/Postgres, based on acadec83: the test-only run failed only on the attested address (20 passed, 1 expected failure). With the fix, all 21 browser transaction integration tests passed, including true, false, and omitted proof, unchanged primary and existing sign-in method, and unchanged snapshots on both explicit LINK refusals. The callback response and the persisted secondary flag agree. These tests mock the outbound connector. The deployed explicit LINK success journey is not covered by the existing hosted OAuth scenario (registration, login, and relink), so deployed E2E remains unverified.

Hosted E2E resolves the earlier deployed coverage gap. [PR #45](https://github.com/Djoe-Denne/IAMRusty/pull/45) landed at `08daed1d` with the qualified tree. In [hosted k6 run 45241087507, attempt 1](https://github.com/Djoe-Denne/IAMRusty/actions/runs/45241087507/attempts/1), all 67 checks passed, including 33 explicit LINK checks across 23 requests. The LINK case verified the attested, false, and omitted exact-address proof outcomes and preserved the original primary addresses and sign-in methods. The native HTTP/Postgres tests above supply the refused-link no-mutation evidence. This closes the backend-alignment task only; the parent story remains open.
<!-- SECTION:NOTES:END -->
