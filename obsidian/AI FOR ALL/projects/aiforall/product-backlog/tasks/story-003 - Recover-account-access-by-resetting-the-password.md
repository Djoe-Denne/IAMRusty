---
id: STORY-003
title: Recover account access by resetting the password
status: Accepted intent
assignee: []
created_date: '2026-11-21T16:33:04Z'
labels: []
dependencies: []
documentation:
  - docs/functional/identite.md
  - services/IAMRusty/qa/scenarii/16_password_reset_unauthenticated_success.md
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A person who has forgotten the password for an existing email/password account and still has access to its inbox can set a new password and regain access.

## Delivery scope

This is approved product intent v2 for the complete recovery journey. Keep the existing confirmation response and the separate sign-in after reset. Where the story calls for sessions to end, both authenticated access and renewal must stop immediately. Preventing renewal alone is not sufficient. Changing a known password while signed in is a separate story.

The interface is part of the story but deferred from delivery. If delivery is later authorized, scope backend and interface work separately. No delivery tasks are scheduled here.

See the [identity handbook](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/identite.md) for the existing API behavior. The referenced QA scenario is an evidence candidate, not an additional product rule.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - Anonymous requests for known and unknown addresses receive the same acknowledgement without disclosing account existence. An anonymous request does not end sessions or lock the account. A repeat request for the same address within one minute does not send another reset email, even from a different source.
- [ ] #2 E2 - Given a valid reset link, successfully saving a new password confirms the reset without creating a session. The interface invites the person to sign in separately with the new password.
- [ ] #3 E3 - As soon as the reset succeeds, all earlier sessions, including on other devices, lose authenticated access and cannot renew. This does not wait for the old access tokens to expire or for another sign-in.
- [ ] #4 E4 - An expired or already used link cannot reset the password. The interface offers a new reset request on that page without requiring sign-in.
- [ ] #5 E5 - When a reset request from a signed-in person for their own account is accepted, all sessions end immediately, including the initiating session, without locking the account.
- [ ] #6 E6 - If consecutive failures disabled password sign-in, verified recovery binds a new password and restores that method without signing the person in. Waiting alone does not restore password sign-in.
<!-- AC:END -->
