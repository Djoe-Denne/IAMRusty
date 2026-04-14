---
title: >-
  OAuth Provider Linking
category: concepts
tags: [iam, oauth, authentication, visibility/internal]
sources:
  - IAMRusty/README.md
  - IAMRusty/docs/ARCHITECTURE.md
  - IAMRusty/docs/OAUTH_SECURITY_GUIDE.md
summary: >-
  IAMRusty treats identities as provider-agnostic users and lets authenticated accounts attach additional OAuth providers safely.
provenance:
  extracted: 0.84
  inferred: 0.11
  ambiguous: 0.05
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T17:03:47.5107188Z
---

# OAuth Provider Linking

`[[projects/iamrusty/iamrusty]]` distinguishes between signing in with an OAuth provider and linking a new provider to an already authenticated user. The design keeps the user identity model provider-agnostic while still allowing provider-specific token handling.

## Key Ideas

- The same OAuth start endpoint supports both login and provider-linking flows depending on whether the request already carries an authenticated context.
- Linked accounts share one user record, while emails are stored separately so new provider emails can be added without replacing the primary address.
- Operation type is encoded in the OAuth state payload, which keeps the callback flow secure and stateless.
- Link-provider operations require authenticated context, exact redirect handling, and conflict detection so one provider identity cannot be silently attached to multiple users.
- This concept depends on `[[concepts/oauth-state-and-csrf-protection]]` to prevent CSRF, code injection, and callback tampering.

## Open Questions

- The current docs focus on GitHub and GitLab, so support for future providers is implied rather than documented.
- The wiki still does not capture user-facing UX decisions around conflict resolution when providers disagree on profile data. ^[ambiguous]

## Sources

- [[references/iamrusty-service]] — IAMRusty authentication and architecture summary
- [[references/iamrusty-runtime-and-security]] — Security model that hardens linking
- [[projects/iamrusty/iamrusty]] — Service overview for the concept's main home
- [[concepts/oauth-state-and-csrf-protection]] — Adjacent security mechanism this flow depends on
