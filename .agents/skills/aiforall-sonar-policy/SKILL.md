---
name: aiforall-sonar-policy
description: >-
  Clippy/Sonar policy for AIForAll services (IAMRusty, Hive, Telegraph,
  Manifesto, sentinel-sync) after the Aug 2026 campaigns. Use when fixing
  Sonar/Clippy issues on Djoe-Denne_IAMRusty, writing rustdoc # Errors/# Panics,
  replacing unwrap after persist, OAuth URL construction, From vs TryFrom,
  future_not_send, too_many_lines migrations/setup, or duplicate_mod in
  Telegraph tests.
---

# AIForAll — politique Sonar / Clippy

Skill **services** (pas le SDK). Pour rustycog-framework, voir
`rustycog/.cursor/skills/rustycog-sonar-parallel/SKILL.md`.

Lots file-disjoint. Mutex = package Cargo × `{src|tests}`. Pas de `cargo`
workspace Docker pendant que d’autres lots éditent. Check crate-local.

## Mécanique (faire)

1. Rustdoc `# Errors` / `# Panics` (`missing_errors_doc` / `missing_panics_doc`).
2. Clippy local : `uninlined_format_args`, `map_or` / `map_or_else`, `ptr_arg`,
   `redundant_closure`, `CommandConfig` / gros structs **par référence**.
3. Extraire un helper plutôt que `#[allow(clippy::too_many_lines)]` sur `new()`
   ou `up()`.

## Judgment — idiome Rust (Aug 30)

| Règle | Faire | Ne pas faire |
|---|---|---|
| `id.unwrap()` après persist | `ok_or_else` + `DomainError::internal_error("… missing id after persist")` | type-state, `expect` sur l’id métier |
| URL OAuth depuis **config `String`** | `new()` / `from_config()` → `Result` (`DomainError::OAuth2Error`) | `AuthUrl::new(url).unwrap()` / `expect` |
| Body HTTP `from_utf8` (debug) | `String::from_utf8_lossy` | `unwrap` |
| `From<String>` + panic | `FromStr` / `TryFrom` | garder le panic |
| `From<Enum> for &str` infaillible | **garder** | |
| `MemberRolePermission::Delete` | `TryFrom` → `Err` (pas un `PermissionLevel` Hive) | mapper silencieusement |
| `future_not_send` | extraire, ne pas tenir un `MutexGuard` au-delà d’un `.await` | `#[allow(clippy::future_not_send)]` |
| DDL migration | extraire un helper par table ; `expect("DDL")` | `unwrap` |
| `serde_json` de **nos** types | `expect("Serialize")` | |
| `Mutex::lock().unwrap()` | **garder** (poison = bug) | |
| Telegraph tests `duplicate_mod` | un seul `#[path = "fixtures/mod.rs"]` dans `tests/common.rs` | re-`path` dans chaque `*_test.rs` |

Hive : un rôle invalide fait échouer tout l’add/update (`collect::<Result<_>>()`).

Setup IAM : extraire `setup_*` dans `IAMRusty/setup/src/app.rs`, pas d’`allow` sur `new()`.

## Skip volontaire (ne pas « corriger »)

- Builders fluents `Option<Option<_>>` **tri-état** (unset = défaut vs `Some(None)` = NULL).
- `is_*` fluents `mut self -> Self` si call sites hors lot (`wrong_self_convention`).
- `FromStr` sur VO Manifesto si `Type::from_str` existe déjà sans `use FromStr` (casse API).
- `future_not_send` dans `IAMRusty/domain/src/service/provider_link_service.rs` (B014).
- `unwrap` restants : repos Hive infra, `IAMRusty/http/src/validation.rs`.
- `too_many_lines` sur `create_hive_registry`.

## Après un lot

`cargo check -p <crate>` (et `--tests` si lane tests). Clippy ciblé
`-W clippy::future_not_send` / `-W clippy::too_many_lines` sur les fichiers
touchés. Ne pas `change_sonar_issue_status` sauf vrai faux positif.
