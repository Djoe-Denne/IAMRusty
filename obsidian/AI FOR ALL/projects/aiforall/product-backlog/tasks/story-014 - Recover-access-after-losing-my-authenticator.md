---
id: STORY-014
title: Recover access after losing my authenticator
status: Accepted intent
assignee: []
created_date: '2026-11-21T20:00:34Z'
labels: []
dependencies: []
documentation:
  - >-
    https://cheatsheetseries.owasp.org/cheatsheets/Multifactor_Authentication_Cheat_Sheet.html#resetting-mfa
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A person who has AIForAll two-factor authentication (2FA) enabled and can no longer obtain a valid code from their authenticator can recover access using a single-use recovery code prepared before the loss.

## Scope

This is approved product intent v1. If the authenticator app can be restored and generates valid codes again, the person signs in normally; AIForAll does not restore app backups. Initial activation is described in STORY-004. Replacing an authenticator that still works is covered in STORY-013. This story does not change other approved sign-in methods.

This story does not specify the timing of code consumption during an interrupted attempt, issuing a replacement code after use, or the exact email text and recipient selection among previously verified addresses. The interface is part of the intended journey; no delivery is scheduled. [OWASP recovery guidance](https://cheatsheetseries.owasp.org/cheatsheets/Multifactor_Authentication_Cheat_Sheet.html#resetting-mfa) is a design reference, not a conformity claim.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - When 2FA is activated, offer a single-use recovery code and explain that it should be stored separately from the phone that runs the authenticator.
- [ ] #2 E2 - A person who cannot get an app code can start recovery by proving an established sign-in method and presenting their previously saved unused recovery code. In this recovery journey, neither proof alone opens an ordinary session or authorizes replacement.
- [ ] #3 E3 - After both proofs, the person sets up a new app and confirms a first valid code. On success, the old configuration no longer works, 2FA has not been turned off, and the recovery code cannot be reused.
- [ ] #4 E4 - If recovery proof is missing, invalid, or already used, or recovery is cancelled or fails, do not replace the factor, open an ordinary session, or end existing sessions. Password reset and inbox access do not authorize a new code after loss.
- [ ] #5 E5 - On successful recovery, all prior AIForAll sessions immediately lose access and cannot renew. Recovery opens no new session. The person signs in normally afterward with the new app available.
- [ ] #6 E6 - After successful recovery, send a security email to an already verified address. If none is available, send no email.
<!-- AC:END -->
