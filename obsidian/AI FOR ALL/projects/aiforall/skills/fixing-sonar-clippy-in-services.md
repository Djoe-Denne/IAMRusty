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
updated: 2026-10-08T16:50:00Z
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

## Campaign 2026-10-08

`#[allow]` does not close an issue. Full do/don't table: `.agents/skills/aiforall-sonar-policy/SKILL.md` and `.cursor/skills/aiforall-sonar-policy/SKILL.md`. SDK pitfalls: `rustycog/.cursor/skills/rustycog-sonar-parallel/SKILL.md`.

Rules closed in that campaign:

- Clippy: `double_must_use`, `doc_markdown`, `explicit_auto_deref`, `needless_borrow`, `needless_borrows_for_generic_args`, `items_after_statements`, `must_use_candidate`, `return_self_not_must_use`, `missing_const_for_fn`, `missing_errors_doc`, `missing_panics_doc`, `option_if_let_else`, `map_unwrap_or`, `redundant_closure`, `redundant_closure_for_method_calls`, `or_fun_call`, `semicolon_if_nothing_returned`, `ignored_unit_patterns`, `collapsible_if`, `manual_assert`, `needless_raw_string_hashes`, `cloned_ref_to_slice_refs`, `unreadable_literal`, `too_many_lines`, `too_many_arguments`, `used_underscore_binding`, `struct_field_names`, `large_futures`, `significant_drop_tightening`, `cast_possible_wrap`, `cast_possible_truncation`, `future_not_send`, `unused_async`, `unused_self`, `needless_pass_by_value`, `expect_used`, `redundant_pub_crate`, `option_option`, `match_bool`, `needless_continue`, `assertions_on_constants`, `needless_option_as_deref`, `match_same_arms`, `bool_to_int_with_if`, `clone_on_copy`, `redundant_clone`, `format_push_string`.
- Sonar: `rust:S1612`, `rust:S107`, `rust:S2208`, `rust:S7493`, `secrets:S6706`.

Sharp traps: `async_trait` 0.1.89 injects `#[must_use]` — bump to 0.1.92, do not add the attribute. `large_futures` → `Box::pin`. `rust:S7493` → `spawn_blocking` only where the key exists. `needless_pass_by_value` stays by value when a reference would clone (`filter_jwks`, `to_domain`). `expect` → `panic!` does not fix `expect_used`. `secrets:S6706` → `test_rs256_*_pem()` plus every caller. Dropping `Deref` must keep `OwnedContainer::stop`. One Windows `cargo`, `-j 1`.

Parallel execution: [[projects/aiforall/skills/running-parallel-sonar-lanes]].
