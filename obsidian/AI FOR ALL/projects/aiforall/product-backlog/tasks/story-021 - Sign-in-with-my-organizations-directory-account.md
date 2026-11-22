---
id: STORY-021
title: Sign in with my organization's directory account
status: Accepted intent
assignee: []
created_date: '2026-11-22T14:08:39Z'
updated_date: '2026-11-22T14:09:10Z'
labels: []
dependencies: []
documentation:
  - https://www.rfc-editor.org/info/rfc4511
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

An eligible person can use an LDAP- or Active Directory-backed work identity without AIForAll maintaining a copy of the directory password. This story does not require publicly exposing the directory, importing users in bulk, or writing back to the directory. No connector choice or delivery is approved.

## Account rules

Reuse STORY-005 through STORY-007. A returning person with an existing AIForAll account signs in there and explicitly links the work identity, with recent account proof, any required 2FA, and confirmation from the work provider. Identify a work account by its stable identifier within the trusted authority, not by matching email, domain, or username. If an email or external identity is already in use, give safe guidance without silently linking, moving the identity, or creating a duplicate. If a second AIForAll account already exists, leave both accounts separate; no merge or retroactive identity transfer. A successful enterprise sign-in does not grant organization membership, administrator rights, or private project access. Existing account-state and 2FA rules still apply. Automated provisioning and deprovisioning, provider-wide logout, and group-to-role mapping are out of scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - Choose the configured organization connection and authenticate through its sign-in service. Open a session only after the trusted service confirms the work identity and AIForAll sign-in requirements are met.
- [ ] #2 E2 - A linked directory identity still reaches the same AIForAll account after a rename. Identical names from different directories do not identify the same person.
- [ ] #3 E3 - If the trusted service cannot confirm authentication or reports that the directory account cannot sign in, open no session through that connection. Evaluate other already established methods under their own rules.
<!-- AC:END -->
