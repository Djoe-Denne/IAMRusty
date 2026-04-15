---
title: Using RustyCog Meta
category: skills
tags: [rustycog, packaging, skills, visibility/internal]
sources:
  - rustycog/Cargo.toml
  - Cargo.toml
summary: Decision guide for consuming rustycog-meta as an umbrella dependency versus selecting individual rustycog crates explicitly.
provenance:
  extracted: 0.86
  inferred: 0.06
  ambiguous: 0.08
created: 2026-04-15T17:15:56.0808743Z
updated: 2026-04-15T17:15:56.0808743Z
---

# Using RustyCog Meta

Use this guide when deciding how to consume `[[projects/rustycog/references/rustycog-meta]]`.

## Workflow

- Start with `rustycog-meta` when bootstrapping a new service quickly with the full default stack.
- Switch to explicit per-crate dependencies when you need tighter compile/dependency control.
- Keep dependency policy explicit in the service README/setup docs so onboarding stays predictable.
- Re-evaluate dependency mode as service boundaries mature and only a subset of crates is actively used.

## Common Pitfalls

- Treating meta-package and explicit dependencies as interchangeable without checking workspace packaging drift.
- Keeping unused crates via meta dependency long after service scope narrows.
- Adopting explicit mode but forgetting transitive crates previously supplied by meta.

## Sources

- [[projects/rustycog/references/rustycog-meta]]
- [[projects/rustycog/rustycog]]
- [[concepts/shared-rust-microservice-sdk]]
