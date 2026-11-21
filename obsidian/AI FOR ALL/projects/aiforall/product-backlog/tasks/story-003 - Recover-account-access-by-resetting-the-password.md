---
id: STORY-003
title: Recover account access by resetting the password
status: Accepted intent
assignee: []
created_date: '2026-11-21T16:33:04Z'
updated_date: '2026-11-21T19:15:06Z'
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

A person who has forgotten the password for an existing email/password account and still has access to its inbox can set a new password and regain access. A signed-in person who knows their current password can change it directly instead.

## Delivery scope

This is approved product intent v3. After a successful reset from a link, keep the existing confirmation response and separate sign-in. All earlier sessions must immediately lose authenticated access and the ability to renew. Preventing renewal alone is not sufficient. The signed-in change follows the existing current-password flow instead of requesting a recovery link. This revision does not set a new policy for when the signed-in change requires reauthentication.

The interface is part of the story but deferred from delivery. If delivery is later authorized, scope backend and interface work separately. No delivery tasks are scheduled here.

See the [identity handbook](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/identite.md) for the existing API behavior. The referenced QA scenario is an evidence candidate, not an additional product rule.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - Anonymous requests for known and unknown addresses receive the same acknowledgement without disclosing account existence. An anonymous request does not end sessions or lock the account. A repeat request for the same address within one minute does not send another reset email, even from a different source.
- [ ] #2 E2 - Given a valid reset link, successfully saving a new password confirms the reset without creating a session. The interface invites the person to sign in separately with the new password.
- [ ] #3 E3 - As soon as the reset succeeds, all earlier sessions, including on other devices, lose authenticated access and cannot renew. This does not wait for the old access tokens to expire or for another sign-in.
- [ ] #4 E4 - An expired or already used link cannot reset the password. The interface offers a new reset request on that page without requiring sign-in.
- [ ] #5 E5 - A signed-in person who knows their current password can enter it with a valid new password and change it directly. AIForAll confirms the change without sending a reset email or opening a new session. An incorrect current password leaves the password unchanged.
- [ ] #6 E6 - If consecutive failures disabled password sign-in, verified recovery binds a new password and restores that method without signing the person in. Waiting alone does not restore password sign-in.
<!-- AC:END -->
