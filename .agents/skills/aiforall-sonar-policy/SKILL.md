---
name: aiforall-sonar-policy
description: >-
  Clippy/Sonar policy and reusable correction decisions for AIForAll services
  (IAMRusty, Hive, Telegraph, Manifesto, sentinel-sync). Consult precedents before
  fixing diagnostics or GitHub lint issues; record new decisions and obtain
  approval for deviations. Use when fixing
  Sonar/Clippy issues on Djoe-Denne_IAMRusty, writing rustdoc # Errors/# Panics,
  replacing unwrap after persist, OAuth URL construction, From vs TryFrom,
  future_not_send, too_many_lines migrations/setup, duplicate_mod in
  Telegraph tests, or the 2026-10-08 rules (double_must_use, large_futures,
  rust:S7493, rust:S2208, secrets:S6706, needless_pass_by_value).
---

# AIForAll — politique Sonar / Clippy

Skill **services** (pas le SDK). Pour rustycog-framework, voir
`rustycog/.cursor/skills/rustycog-sonar-parallel/SKILL.md`.

Lots file-disjoint, claim écrit avant toute édition. Mutex = package Cargo ×
`{src|tests}`. Un seul `cargo` Windows à la fois, avec **24 jobs de compilation**
via `.cargo/config.toml`, selon `AGENTS.md` (décision utilisateur 2026-10-25).
Ne pas imposer `-j 1`, `-j 2` ou un `CARGO_BUILD_JOBS` contradictoire : les
anciennes limites liées aux OOM LLVM (os error 1455) sont historiques. Si la
pression mémoire réapparaît, demander un arbitrage avant de réduire les jobs.
Pas de cargo pendant que d’autres lots éditent. Pas de compilation dans Docker.

## Consulter, appliquer et conserver les décisions

Les modèles peuvent proposer plusieurs correctifs valides pour une même règle.
Pour conserver les choix du projet, consulter **avant toute correction** les
tables de ce skill et le registre partagé
[`docs/guides/clippy-decisions/README.md`](../../../docs/guides/clippy-decisions/README.md)
(chemin depuis la racine du dépôt). Le registre ne remplace pas les tables :
il référence les nouveaux arbitrages et leurs preuves, sans second canon.

1. **Identifier le contexte réel** : règle Clippy/Sonar, code actuel, production
   ou tests, API/appelants, features et version d'outil si pertinentes. Une issue
   GitHub ou un ancien numéro de ligne ne suffit pas à établir ce contexte.
2. **Chercher un précédent** dans les tables puis l'index du registre ; lire
   l'entrée pertinente, son statut, ses conditions et ses exceptions. Réutiliser
   une décision `Accepted` ou une consigne existante du skill **seulement si ses
   conditions sont remplies**. Citer son ID ou le titre de section + la règle
   dans le compte rendu/PR de correction. Ne pas créer une entrée par occurrence
   pour une correction mécanique déjà couverte.
3. **Sans précédent applicable**, consigner un nouvel arbitrage non trivial
   `Proposed`, avec le modèle du registre : règle, contexte, solution choisie,
   justification, alternatives, exemple et validations. Faire valider avant de
   le promouvoir en règle partagée `Accepted` ; un build vert ou l'avis du modèle
   auteur ne constitue pas cette approbation. Une proposition n'est pas un
   précédent acquis. L'implémentation éventuelle reste bornée au mandat du lot.
4. **En cas de divergence**, ne pas remplacer silencieusement le précédent.
   Documenter la différence de contexte ou la dérogation proposée et obtenir une
   validation explicite de l'utilisateur ou d'un reviewer mandaté pour cet
   arbitrage **avant d'appliquer un choix contraire**. Si deux précédents se
   contredisent, suspendre ce choix et demander l'arbitrage. Un changement
   structurel suit aussi les ADR/contrats ; ce registre ne peut les déroger.
5. **Conserver la trace dans le même lot** : entrée et index, référence de
   l'issue/PR/commit, validations réellement exécutées et identité/référence de
   l'approbation. Une décision remplacée devient `Superseded` avec lien vers la
   suivante ; conserver son historique, ne pas la réécrire rétroactivement.

En audit/read-only, proposer l'entrée dans le rapport **sans écrire** dans le
registre. Les membres parallèles rendent leurs propositions au lead, qui tient
l'index sans écritures concurrentes. Ces consignes ne déclenchent ni mise à jour
automatique du skill/wiki ni nouvelle campagne ; ne pas modifier le sous-module
RustyCog pour capitaliser une décision concernant les services.

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
| `Mutex::lock().unwrap()` | `unwrap_or_else(PoisonError::into_inner)` | `unwrap()` / `expect()` |
| Telegraph tests `duplicate_mod` | un seul `#[path = "fixtures/mod.rs"]` dans `tests/common.rs` | re-`path` dans chaque `*_test.rs` |

Hive : un rôle invalide fait échouer tout l’add/update (`collect::<Result<_>>()`).

Setup IAM : extraire `setup_*` dans `services/IAMRusty/setup/src/app.rs`, pas d’`allow` sur `new()`.

## Skips invalidés (opérateur 2026-08-31)

Ces familles se traitent, plus de skip « policy / hors lot / casse API » :

- Builders fluents : plus de `Option<Option<T>>` — `OptionalField::{Unset,Set}` + `with_x` / `clear_x`.
- Fluents `is_*` (`mut self -> Self`) : rename `with_*` ; getters `&self -> bool` inchangés.
- `FromStr` sur VO Manifesto **et** Hive : `impl FromStr` + `use std::str::FromStr` aux call sites.
- `future_not_send` `provider_link_service` : bornes `Send + Sync`, pas d’`allow`.
- `Mutex::lock()` : `unwrap_or_else(PoisonError::into_inner)`.
- `too_many_lines` `create_hive_registry` : extraire `register_*` / `setup_*`, pas d’`allow`.
- `unwrap` persist Hive infra : `try_into_model` / `model_after_persist` + `ok_or_else` internal_error.

## Campagne 2026-10-08 — règles traitées

`#[allow]` n’est pas un correctif. Lignes Sonar périmées : lire le fichier.
Un `#[path]` dupliqué se corrige une fois dans la source. Une signature
changée met à jour tous les appelants avant de rendre le lot.

| Règle | Faire | Ne pas faire |
|---|---|---|
| `double_must_use` | Attribut déjà dans le source et l’obligation est d’awaiter : `#[must_use = "await the future"]`. Injecté par `async_trait` 0.1.89 : monter la crate à **0.1.92** (retire le `#[must_use]` nu). La 0.1.91 change la mutabilité des receivers : un cargo vert avant les lots parallèles. | Ajouter un attribut absent du source. |
| `doc_markdown`, `missing_errors_doc`, `missing_panics_doc` | backticks ; `# Errors` / `# Panics` | réécrire le paragraphe |
| `explicit_auto_deref`, `needless_borrow`, `needless_borrows_for_generic_args` | le déréférencement ou l’emprunt demandé | |
| `items_after_statements` | déclarations en tête de fonction | |
| `must_use_candidate`, `return_self_not_must_use`, `missing_const_for_fn` | `#[must_use]` ou `const fn` si le corps l’est déjà | `const` sur un trait public |
| `option_if_let_else` | laisser un `match` déjà en place | réécrire ce `match` |
| `map_unwrap_or`, `redundant_closure`, `redundant_closure_for_method_calls`, `or_fun_call`, `rust:S1612` | appel de méthode ou de fonction | closure inutile |
| `semicolon_if_nothing_returned`, `ignored_unit_patterns`, `collapsible_if`, `manual_assert`, `needless_raw_string_hashes`, `cloned_ref_to_slice_refs`, `unreadable_literal` | le correctif Clippy local | |
| `too_many_lines`, `too_many_arguments`, `rust:S107` | helpers privés ; struct de paramètres. `tests/common.rs` reste entier chez un seul agent. | `#[allow]` ; couper ce fichier entre agents |
| `used_underscore_binding` | `_fixture` → `fixture` quand le champ est lu | renommer un binding vraiment ignoré |
| `struct_field_names` | renommer le champ **et** tous les appelants | renommer un champ `pub` sans ses appelants |
| `large_futures` | `Box::pin(...).await` sur la future grosse (`setup_test_server`) | |
| `significant_drop_tightening` | `drop` dès que le guard ne sert plus | tenir un `MutexGuard` au-delà d’un `.await` |
| `cast_possible_wrap`, `cast_possible_truncation` | `TryFrom` | `as` |
| `future_not_send` | extraire ; ne pas tenir un guard au `.await` | `#[allow]` |
| `rust:S7493` | `spawn_blocking` sur l’I/O bloquante de la clé | l’inventer sur une fn sync sans clé |
| `unused_async` | `fn` s’il n’y a pas de `.await`, appelants inclus | retirer `async` d’une API que les appelants `.await` encore |
| `unused_self` | fonction associée s’il n’y a aucun appelant méthode | |
| `needless_pass_by_value` | `&T` / `&Arc<T>` quand l’appelant peut prêter | une référence qui force des clones (`filter_jwks`, `to_domain` restent par valeur) |
| `expect_used` | `Result` ou `TryFrom` | remplacer `expect` par `panic!` |
| `redundant_pub_crate` | `pub` dans un module déjà privé | élargir la visibilité réelle |
| `rust:S2208` | imports explicites | `use super::*` |
| `secrets:S6706` | clé de test via `OnceLock` et `test_rs256_*_pem()` ; chaque appelant de `TEST_RS256_*_PEM` suit | constante PEM dans le source |
| `option_option` | `OptionalField::{Unset,Set}` | `Option<Option<T>>` |
| `Deref` retiré | garder les méthodes qui n’existaient que via `Deref` (`OwnedContainer::stop`) | supprimer le trait et oublier l’appel |

`match_bool` se réécrit en `if`. `needless_continue`, `assertions_on_constants`,
`needless_option_as_deref`, `match_same_arms`, `bool_to_int_with_if`,
`clone_on_copy`, `redundant_clone`, `format_push_string` (`write!`) : correctif
local, sans changement de signature.

## Après un lot

`cargo check -p <crate>` (et `--tests` si lane tests). Clippy ciblé
`-W clippy::future_not_send` / `-W clippy::too_many_lines` sur les fichiers
touchés. Ne pas `change_sonar_issue_status` sauf vrai faux positif.
