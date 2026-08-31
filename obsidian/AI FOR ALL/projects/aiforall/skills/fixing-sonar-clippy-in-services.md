---
title: Fixing Sonar / Clippy in AIForAll services
category: skills
tags: [skills, sonar, clippy, aiforall, visibility/internal]
aliases:
  - aiforall-sonar-policy
summary: >-
  Policy for closing Clippy/Sonar smells in IAMRusty, Hive, Telegraph,
  Manifesto, and sentinel-sync after the August 2026 campaigns (persist-id
  Result, OAuth Result, TryFrom, Send, migration helpers, Telegraph fixtures).
provenance:
  extracted: 0.85
  inferred: 0.1
  ambiguous: 0.05
created: 2026-08-31T09:45:00Z
updated: 2026-08-31T13:30:00Z
---

# Fixing Sonar / Clippy in AIForAll services

Portable agent copy: `.agents/skills/aiforall-sonar-policy/SKILL.md`.
SDK campaign: `rustycog/.cursor/skills/rustycog-sonar-parallel/SKILL.md` (created 2026-08-30 15:56, audit through `6b7f5fd`).

See [[skills/using-rustycog-core]] for `DomainError` / persist-id.
See [[skills/using-rustycog-testing]] for `TestOpenFga` and a single `#[path]` in `tests/common.rs`.

## Mechanical

Rustdoc `# Errors` / `# Panics`. Local Clippy (`map_or`, `ptr_arg`, `CommandConfig` by ref). Extract helpers instead of `#[allow(clippy::too_many_lines)]` on `new()` / `up()`.

## Judgment (2026-08-30)

- Persist id → `ok_or_else` + `DomainError::internal_error`, not `expect` on the business id.
- OAuth URLs from config `String` → `new()` / `from_config()` return `Result`.
- HTTP debug bodies → `from_utf8_lossy`.
- `From<String>` + panic → `FromStr` / `TryFrom`. Keep infallible `From<Enum> for &str`.
- Hive `MemberRolePermission::Delete` is not a `PermissionLevel`.
- `future_not_send`: no `allow`; do not hold a `MutexGuard` across `.await`.
- Migrations: one helper per table, `expect("DDL")`.
- Our `serde_json` → `expect("Serialize")`. Keep `Mutex::lock().unwrap()`.
- Telegraph: one `#[path = "fixtures/mod.rs"]` in `tests/common.rs`.

## Invalidated skips (operator 2026-08-31)

These families are **in scope** — no more “policy / hors lot / casse API” skip:

- Builders: `OptionalField::{Unset,Set}` or [[concepts/optional-field-update]], not `Option<Option<T>>`.
- Fluents `is_*` (`mut self -> Self`): rename `with_*`; getters `&self -> bool` stay.
- `FromStr` on Manifesto **and** Hive VOs, with `use std::str::FromStr` at call sites.
- `provider_link_service` `future_not_send`: `Send + Sync` bounds, no `allow`.
- `Mutex::lock()`: `unwrap_or_else(PoisonError::into_inner)`.
- `create_hive_registry` length: extract `register_*` / `setup_*`.
- Hive persist unwraps: `try_into_model` / `model_after_persist` + `ok_or_else` internal_error.

Parallel execution: [[projects/aiforall/skills/running-parallel-sonar-lanes]].
