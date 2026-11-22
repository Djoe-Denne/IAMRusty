---
id: STORY-020
title: Sign in through my organization's SAML provider
status: Accepted intent
assignee: []
created_date: '2026-11-22T14:08:39Z'
updated_date: '2026-11-22T14:08:54Z'
labels: []
dependencies: []
documentation:
  - https://www.oasis-open.org/standard/saml/
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

An eligible person can sign in to AIForAll through a configured, trusted SAML 2.0 identity provider without entering their work password into AIForAll. This story starts from AIForAll; provider-initiated sign-in is not assumed. The integration can be direct or through a trusted provider bridge; no connector choice or delivery is approved.

## Account rules

Reuse STORY-005 through STORY-007. A returning person with an existing AIForAll account signs in there and explicitly links the work identity, with recent account proof, any required 2FA, and confirmation from the work provider. Identify a work account by its stable identifier within the trusted authority, not by matching email, domain, or username. If an email or external identity is already in use, give safe guidance without silently linking, moving the identity, or creating a duplicate. If a second AIForAll account already exists, leave both accounts separate; no merge or retroactive identity transfer. A successful enterprise sign-in does not grant organization membership, administrator rights, or private project access. Existing account-state and 2FA rules still apply. Automated provisioning and deprovisioning, provider-wide logout, and group-to-role mapping are out of scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - Starting in AIForAll, choose the configured organization option and authenticate at the trusted provider. After the required sign-in steps, return to the originating AIForAll page or home.
- [ ] #2 E2 - A linked identity returns to the same account after an email or display-name change. A first-time identity follows explicit linking or signup, not automatic association.
- [ ] #3 E3 - If the provider response cannot be verified for the connection and the attempt, the person cancels, or the provider is unavailable, open no session from that attempt. Offer retry or an already available method as appropriate.
<!-- AC:END -->
