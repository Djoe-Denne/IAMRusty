---
id: STORY-007
title: Sign in with an already-linked external provider
status: Accepted intent
assignee: []
created_date: '2026-11-21T18:28:59Z'
labels: []
dependencies: []
documentation:
  - docs/functional/identite.md
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A returning person can sign in to their existing AIForAll account with an external provider identity already linked to it. This covers compatible OpenID Connect providers and OAuth 2.0 services that provide a verifiable user identity.

## Identity and security

This is approved product intent v1. Sign-in uses the linked provider identity, never an email match. An unlinked identity does not create an account or establish a link during this sign-in journey. Provider linking and signup are separate in STORY-005 and STORY-006.

Complete all required authentication before opening an AIForAll session. The [OWASP ASVS guidance on identity providers](https://github.com/OWASP/ASVS/blob/v5.0.0/5.0/en/0x15-V6-Authentication.md#v68-authentication-with-an-identity-provider) informs how to assess provider evidence; accepted evidence and freshness rules are not specified here. Return navigation does not grant access to a page the person is not allowed to see.

## Delivery scope

The interface is part of the story, but no delivery is scheduled. Provider outage recovery and unavailable return destinations are not defined here. See the [identity handbook](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/identite.md) for existing API behavior, not additional acceptance criteria.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - After successful provider authentication and completion of any required AIForAll checks, open a session for the account attached to the authenticated provider identity, not an account selected by email.
- [ ] #2 E2 - If the provider returns a different email or omits it, the authenticated, already-linked identity still signs in to the same AIForAll account. Do not retarget the link or change any saved addresses.
- [ ] #3 E3 - If the person cancels or the provider cannot authenticate them, no AIForAll session opens and no account or link changes.
- [ ] #4 E4 - After sign-in is complete, including any required second factor, return to the originating AIForAll page or to the home page if sign-in was opened directly.
- [ ] #5 E5 - If AIForAll can verify that an accepted provider used sufficient multi-factor authentication for this sign-in, do not request another code.
- [ ] #6 E6 - If AIForAll 2FA is enabled and adequate provider MFA cannot be verified for this sign-in, request the AIForAll code and open no session unless it is valid. If AIForAll 2FA is off, this fallback does not ask for a code.
<!-- AC:END -->
