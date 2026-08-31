---
title: >-
  RustyCog
category: project
tags: [rustycog, sdk, platform, visibility/internal]
sources:
  - rustycog/README.md
  - .agents/skills/rustycog-submodule/SKILL.md
  - skills/building-rustycog-services.md
summary: >-
  Shared Rust microservice SDK consumed as a git submodule. Crate how-tos live under skills/using-rustycog-*; this page is the project hub.
provenance:
  extracted: 0.72
  inferred: 0.22
  ambiguous: 0.06
created: 2026-04-15T17:15:56Z
updated: 2026-08-31T13:30:00Z
---

# RustyCog

RustyCog (`rustycog-framework`, imported as `rustycog`) is the feature-gated SDK every AIForAll service composes: command registry, config, HTTP, events, DB, permissions, logging, testing.

This vault’s detailed crate reference pages (`projects/rustycog/references/rustycog-*`) were linked from older indexes but are **missing from disk** after later syncs. Until they are restored, use the portable skills below as the crate map. ^[ambiguous]

## Pin

`AIForAll/rustycog/` is a git submodule, not a vendored tree. See [[projects/aiforall/concepts/rustycog-git-submodule]].

## Crate skills

- [[skills/building-rustycog-services]]
- [[skills/using-rustycog-core]]
- [[skills/using-rustycog-config]]
- [[skills/using-rustycog-db]]
- [[skills/using-rustycog-command]]
- [[skills/using-rustycog-events]]
- [[skills/using-rustycog-http]]
- [[skills/using-rustycog-permission]]
- [[skills/using-rustycog-testing]]
- [[skills/using-rustycog-logger]]

## Related platform pages

- [[concepts/shared-rust-microservice-sdk]]
- [[concepts/architecture-coherence-across-services]]
- [[projects/aiforall/aiforall]]
