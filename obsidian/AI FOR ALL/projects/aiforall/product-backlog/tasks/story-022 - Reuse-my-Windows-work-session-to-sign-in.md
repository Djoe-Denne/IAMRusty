---
id: STORY-022
title: Reuse my Windows work session to sign in
status: Accepted intent
assignee: []
created_date: '2026-11-22T14:08:39Z'
labels: []
dependencies: []
documentation:
  - >-
    https://learn.microsoft.com/en-us/deployedge/microsoft-edge-security-identity
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A person on a supported, managed Windows work device can sign in without re-entering their work password when the trusted provider and browser support single sign-on. Microsoft Entra and intranet integrated authentication are distinct deployment profiles. The first device, browser, and provider combination remains to be qualified; no connector choice or delivery is approved.

## Account rules

Reuse STORY-005 through STORY-007. A returning person with an existing AIForAll account signs in there and explicitly links the work identity, with recent account proof, any required 2FA, and confirmation from the work provider. Identify a work account by its stable identifier within the trusted authority, not by matching email, domain, or username. If an email or external identity is already in use, give safe guidance without silently linking, moving the identity, or creating a duplicate. If a second AIForAll account already exists, leave both accounts separate; no merge or retroactive identity transfer. A successful enterprise sign-in does not grant organization membership, administrator rights, or private project access. Existing account-state and 2FA rules still apply. Automated provisioning and deprovisioning, provider-wide logout, and group-to-role mapping are out of scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - After the person selects work sign-in in AIForAll, an eligible Windows and browser session supplies identity proof through the trusted provider without asking for the work password again. Request additional proof when the applicable policy requires it.
- [ ] #2 E2 - If silent SSO is unavailable, offer normal interactive sign-in at the provider or another already available AIForAll method. A Windows username or managed-device membership alone opens no session.
- [ ] #3 E3 - A first-time work identity follows explicit linking or signup even if Windows SSO succeeds silently. The device or browser session never links or merges accounts on its own.
<!-- AC:END -->
