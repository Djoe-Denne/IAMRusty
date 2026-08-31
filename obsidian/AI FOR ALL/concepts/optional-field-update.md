---
title: >-
  Optional field update
category: concepts
tags: [domain, rust, manifesto, visibility/internal]
sources:
  - Manifesto/domain/src/value_objects/field_update.rs
  - Manifesto/application/src/command/project.rs
  - Manifesto/application/src/usecase/project.rs
  - .agents/skills/aiforall-sonar-policy/SKILL.md
summary: >-
  FieldUpdate distinguishes leave-unchanged from assign, including assign-None, so PATCH-style updates do not need Option<Option<T>>.
provenance:
  extracted: 0.88
  inferred: 0.12
  ambiguous: 0.00
created: 2026-08-31T13:30:00Z
updated: 2026-08-31T13:30:00Z
---

# Optional field update

Manifesto models partial writes with `FieldUpdate<T>`: `Unchanged` (default) versus `Set(T)`. `T` may itself be `Option<U>`, so “clear this field” is `Set(None)` and “do not touch it” is `Unchanged`.

## Why it exists

- Clippy / Sonar campaigns flagged tri-state `Option<Option<T>>` builders as unreadable and easy to misuse. ^[inferred]
- The Aug 31 Sonar policy now requires `OptionalField::{Unset,Set}` (or this VO) plus `with_x` / `clear_x` instead of nested options. See [[projects/aiforall/skills/fixing-sonar-clippy-in-services]].

## How to read it

- A use case applies only `Set` variants onto the aggregate.
- `Unchanged` must not overwrite an existing value, including when the HTTP body omitted the key.
- Infallible `From<Enum> for &str` stays; fallible string parsing stays on `FromStr` / `TryFrom`.

## Related

- [[projects/manifesto/manifesto]]
- [[projects/manifesto/concepts/project-ownership-and-publication-lifecycle]]
- [[entities/project]]
