---
id: STORY-005
title: Add an external provider as a sign-in method
status: Accepted intent
assignee: []
created_date: '2026-11-21T17:47:31Z'
updated_date: '2026-11-21T17:49:45Z'
labels: []
dependencies: []
documentation:
  - docs/functional/identite.md
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A signed-in account holder can link their account at an external provider as another way to sign in to the same AIForAll account, without replacing existing sign-in methods.

## Provider scope

This story is not limited to specific providers. It covers any compatible OpenID Connect (OIDC) provider or OAuth 2.0 service from which AIForAll can verify a stable user identity scoped to that provider. OAuth 2.0 authorization alone is not proof of identity. Configuring and accepting providers is separate from a person linking their account at a provider.

## Security and scope

This is approved product intent. Require recent AIForAll authentication and explicit confirmation with the provider. A matching email address, even when verified, does not authorize linking or access to an existing account. Explain refusal reasons only after authentication on both sides, without identifying another account. The recency window and exact messages remain open.

The shared signup entry is shown once per attempt. The existing-account choice leads to this journey instead of creating an account. If an unlinked provider returns an already registered email, the entry journey guides the person on screen to sign in to the existing account. It creates no account or draft and sends no resumption email. This story does not define the full signup or provider sign-in journeys.

The interface is part of the intended journey. No delivery is scheduled. Account creation, signing in with an already-linked provider, replacing or removing a method, verifying an address from the profile, and merging AIForAll accounts are separate. Linking does not grant access to provider resources or private projects.

See the [identity handbook](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/identite.md) for existing API behavior, not additional acceptance criteria.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - With recent AIForAll authentication, the person can choose and confirm an available external provider identity without another AIForAll sign-in prompt. The new method appears on the same account without replacing or removing existing sign-in methods.
- [ ] #2 E2 - If the person cancels or provider authentication fails, no new sign-in method is added and existing methods are unchanged.
- [ ] #3 E3 - Different email addresses do not prevent explicit linking. An importable provider address is saved as a secondary address without changing the primary address.
- [ ] #4 E4 - If the provider email belongs to a different AIForAll account, refuse the entire link, explain that the address cannot be used without identifying the other account, and leave the addresses and sign-in methods of both accounts unchanged.
- [ ] #5 E5 - If the provider identity is already linked to another AIForAll account, refuse and say the provider account is already linked without identifying the other account or transferring any sign-in method.
- [ ] #6 E6 - Mark an imported secondary address verified in AIForAll if the provider confirms verification of that exact address. Otherwise it remains unverified.
- [ ] #7 E7 - If AIForAll authentication is not recent, request authentication again through an established method with any required second factor. Do not add the link until that succeeds and the person confirms the provider link.
- [ ] #8 E8 - When the person enters through I already have an AIForAll account, the existing AIForAll account must already have a verified email address before association. If it does not, do not add the link or create a new account.
<!-- AC:END -->
