---
title: >-
  IAMRusty Service Docs
category: references
tags: [reference, iam, oauth, visibility/internal]
sources:
  - IAMRusty/README.md
  - IAMRusty/docs/ARCHITECTURE.md
summary: >-
  Combined source summary for IAMRusty capabilities, configuration, runtime concerns, and hexagonal architecture.
provenance:
  extracted: 0.93
  inferred: 0.07
  ambiguous: 0.00
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T16:54:59.5971424Z
---

# IAMRusty Service Docs

These sources define the main behavior and structure of `[[projects/iamrusty/iamrusty]]`.

## Key Ideas

- IAMRusty is a Rust IAM service supporting OAuth login, provider linking, JWT issuance, and structured configuration.
- The user model is provider-agnostic, with email addresses managed separately and provider tokens stored independently.
- The architecture is explicitly hexagonal, with ports, adapters, use cases, and infrastructure layers clearly separated.
- The service includes advanced testing and database infrastructure, including migrations and read/write split support.

## Open Questions

- Additional operational guides exist, but this first pass did not ingest every IAMRusty doc page.
- Event contract details remain indirect because `iam-events` was not included in the source set.

## Sources

- [[projects/iamrusty/iamrusty]] — Service overview page
- [[concepts/hexagonal-architecture]] — Shared architecture pattern
- [[concepts/oauth-provider-linking]] — Authentication-specific concept distilled from these docs
