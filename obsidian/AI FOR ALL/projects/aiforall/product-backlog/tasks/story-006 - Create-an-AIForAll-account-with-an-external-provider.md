---
id: STORY-006
title: Create an AIForAll account with an external provider
status: Accepted intent
assignee: []
created_date: '2026-11-21T18:12:26Z'
labels: []
dependencies: []
documentation:
  - docs/functional/identite.md
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A person who wants a new AIForAll account can create it using a compatible external identity provider as their initial sign-in method. This covers OpenID Connect and OAuth 2.0 services that provide a verifiable user identity, not a fixed list of providers.

## Entry and scope

This is approved product intent v1. Show the shared choice to create an account or use an existing AIForAll account once per signup attempt. An email match never establishes a link. The existing-account branch uses the explicit linking journey in STORY-005 rather than creating a second account.

The interface is part of the story. Successful signup opens the first session only after the person has authenticated at the provider and finished signup. The destination after signup and interface copy are not fixed. Later sign-in, verifying a saved email from the profile, and changing email/password rules are outside this story. No delivery tasks are scheduled.

See the [identity handbook](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/functional/identite.md) for existing API behavior, not additional acceptance criteria.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - A person chooses to create an account and authenticates with an unlinked provider that supplies an available email. When they confirm their username and finish signup, exactly one account is created with the linked provider and that primary address. The first AIForAll session opens without another provider sign-in or separate AIForAll email verification.
- [ ] #2 E2 - Choosing I already have an AIForAll account does not create a new account. The person can sign in with an established method and any required second factor, then separately confirm linking the provider.
- [ ] #3 E3 - If the provider supplies an already registered email, show existing-account guidance on screen without sending a resumption email. The address match creates no account, draft, or link.
- [ ] #4 E4 - If the provider identity is already linked, offer sign-in with the existing provider instead of creating a second account.
- [ ] #5 E5 - If the person abandons before completing signup, no AIForAll session is granted and no account is left able to sign in or use product features.
- [ ] #6 E6 - If the provider supplies no email, the person enters an available address to finish signup. Save it as the unverified primary address; provider sign-in does not require confirming that address first.
- [ ] #7 E7 - Mark a supplied primary address verified only if the provider confirms verification of that exact address. Otherwise it remains unverified.
- [ ] #8 E8 - If the provider supplies a login, prefill the AIForAll username with it as an editable suggestion. Otherwise the field is blank and the person fills it before completing signup.
<!-- AC:END -->
