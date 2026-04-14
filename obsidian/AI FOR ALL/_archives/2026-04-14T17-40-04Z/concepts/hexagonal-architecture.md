---
title: >-
  Hexagonal Architecture
category: concepts
tags: [architecture, hexagonal, ddd, visibility/internal]
sources:
  - IAMRusty/README.md
  - IAMRusty/docs/ARCHITECTURE.md
  - Manifesto/README.md
  - rustycog/README.md
  - Manifesto/docs/rustycog-hexagonal-web-service-guide.md
summary: >-
  Several services share a ports-and-adapters structure that keeps domain logic isolated from infrastructure and HTTP concerns.
provenance:
  extracted: 0.78
  inferred: 0.17
  ambiguous: 0.05
created: 2026-04-14T16:54:59.5971424Z
updated: 2026-04-14T17:03:47.5107188Z
---

# Hexagonal Architecture

The strongest architectural through-line in this repository is the use of hexagonal or clean layering. `[[projects/iamrusty/iamrusty]]` documents it explicitly, `[[projects/manifesto/manifesto]]` adopts the same shape, and `[[projects/rustycog/rustycog]]` turns the pattern into reusable platform code.

## Key Ideas

- Business logic lives behind ports and use cases, while infrastructure and HTTP adapters stay outside the core.
- IAMRusty documents the clearest four-layer model: domain, application, infrastructure, and HTTP.
- Manifesto's blueprint guides make the composition root explicit: concrete dependencies are wired in setup, not leaked into domain or handlers.
- RustyCog packages configuration, HTTP, DB, permissions, events, and testing utilities that fit naturally around this model.
- The repeated appearance of the pattern suggests the repo is standardizing service construction around a common architecture kit. ^[inferred]

## Open Questions

- IAMRusty is documented in depth, but comparable implementation detail for every other service is not yet in the wiki.
- It is not yet clear whether all services follow the same strict dependency rules or only the same high-level layering. ^[ambiguous]

## Sources

- [[references/iamrusty-service]] — Detailed architecture and layer responsibilities
- [[references/manifesto-service]] — Manifesto's clean architecture shape
- [[references/rustycog-service-construction]] — Blueprint wiring guide for the pattern
- [[references/platform-building-blocks]] — Shared SDK conventions that reinforce the pattern
