---
id: STORY-013
title: Replace my authenticator app while I still have access
status: Accepted intent
assignee: []
created_date: '2026-11-21T19:48:22Z'
labels: []
dependencies: []
documentation:
  - >-
    https://cheatsheetseries.owasp.org/cheatsheets/Multifactor_Authentication_Cheat_Sheet.html#changing-mfa-factors
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A person with AIForAll two-factor authentication (2FA) enabled who can still use their existing authenticator app can replace it with a new app or phone without turning off 2FA.

## Scope

This is approved product intent v1 for planned replacement. Initial activation belongs to STORY-004. Losing access to the existing authenticator is a separate recovery journey. The exact email wording, recipient selection among already verified addresses, and handling of suspected compromise are not specified here. The interface is part of the intended journey; no delivery is scheduled.

[OWASP guidance on changing MFA factors](https://cheatsheetseries.owasp.org/cheatsheets/Multifactor_Authentication_Cheat_Sheet.html#changing-mfa-factors) is a design reference, not a conformity claim.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - A signed-in person starts replacement from their account security settings, authenticates again with an established sign-in method, and enters a valid code from the old app. An active session alone does not authorize replacement.
- [ ] #2 E2 - After the person sets up the new app and confirms a first valid code from it, the new setup replaces the old one without turning off 2FA. Codes from the old setup are no longer accepted.
- [ ] #3 E3 - If a code is invalid, or setup is cancelled or left unfinished, the old setup stays active and the new one is not enabled. A person without the old authenticator cannot use this flow instead of recovery.
- [ ] #4 E4 - After successful replacement, send a security email to an already verified address, or send none if no verified address is available. An unsuccessful or cancelled change does not produce a success email.
- [ ] #5 E5 - After successful replacement, the person can continue in the session where they just reauthenticated. Other AIForAll sessions lose protected access and cannot renew. An unsuccessful or cancelled replacement does not end sessions.
<!-- AC:END -->
