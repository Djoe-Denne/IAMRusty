---
id: STORY-005.01
title: Report the correct explicit provider-link conflict
status: Done
assignee: []
created_date: '2026-11-22T15:53:41Z'
updated_date: '2026-11-23T16:39:10Z'
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
- [x] #1 An unavailable email produces the address-specific refusal without identifying another account.
- [x] #2 An identity already linked elsewhere produces the identity-specific refusal without identifying another account.
- [x] #3 Integration checks verify that both refusals preserve addresses and sign-in methods on both accounts.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Reuse the existing domain, command, and explicit LINK HTTP boundaries (ADR 0409 and 0412); no new ADR. In GET /iam/api/auth/{provider_name}/callback for explicit LINK, return nested error.status 409 with error.error_code provider_email_conflict and fixed message Provider email address cannot be used for an unavailable address. Return 409 with the existing provider_already_linked code and fixed identity message for an identity linked elsewhere. Preserve the envelope, both accounts, and one-time transaction consumption. No login, relink, ownership, or dependency change. No in-repository production consumer of these codes was found; external clients are unverified. Prove both refusals and unchanged account state with native HTTP/Postgres IT under ADR 0200-0202. Deployed E2E evidence is unverified; the existing k6 case covers relink, not these conflicts.

Delivery also requires a dedicated deployed E2E for both explicit LINK conflicts. The existing k6 OAuth run cannot substitute for this evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Native Windows HTTP/Postgres red on 6912d989 with the two regression tests: 17 existing tests passed; both new tests passed account-state and transaction checks, then failed on the expected misclassifications (address returned provider_already_linked; identity returned HTTP 500). No ignored or filtered tests.

Backend proof: all 52 Windows HTTP/Postgres integration tests and 23 focused mapper/HTTP tests passed. The independent correctness, test, and security reviews cleared the backend changes; the test follow-up verified both corrected legacy identity fixtures. Dedicated deployed E2E remains required and unverified. Keep this task In Progress until that gate and integration are complete.

The opt-in deployed k6 profile and fixture are implemented. The first baseline campaign stopped before any k6 scenario or account setup because the shared lab IAM Service had no ready endpoint for its verified JWKS check. This is not a behavioral red or an E2E pass. The campaign removed all 18 owned resources and its dedicated database. Keep the PR draft and the task In Progress pending verified deployed E2E.

Closeout (2026-11-23): the repository owner confirmed that the dedicated deployed E2E gate passed. [PR #42](https://github.com/Djoe-Denne/IAMRusty/pull/42) merged into master at 59a141341a934e1c9b2e9415cd8778bebe1e4a32. This records the later confirmation; the earlier aborted attempt remains historical evidence.
<!-- SECTION:NOTES:END -->
