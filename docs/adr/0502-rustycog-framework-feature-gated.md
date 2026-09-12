# ADR-0502 : rustycog-framework est un submodule feature-gated, pas un service

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

RustyCog est le SDK hexagonal (config, HTTP, DB, events, outbox, tests). Le traiter comme membre du workspace Cargo ou comme cinquième service mélangerait le framework et le métier. Une crate `rustycog-macros` n’existe pas dans ce pin.

## Décision

1. **`rustycog/`** est un **git submodule** (`Djoe-Denne/rustycog`), crate **`rustycog-framework`** 0.1.1, **feature-gated**. Défaut : `core` + `config`. Features : `command`, `db`, `events`, `kafka`, `http`, `logger`, `outbox`, `permission`, `server`, `testing`, `test-utils`, `scaleway-loki`, `full`. Kafka est **séparé** de `events` / `full` sur ce pin local.
2. **Pas un workspace member.** `Cargo.toml` du monorepo déclare `rustycog-framework = { path = "rustycog" }` sous `[workspace.dependencies]` seulement. `members` = events, readiness, IAMRusty, Telegraph, Hive, Manifesto, monolith, sentinel-sync, apparatus-contracts, apparatus-reference-kv.
3. **`rustycog-macros` est ABSENT** (pas de crate, pas de membre, pas de path).
4. **Logging** : les 4 services métier réexportent `rustycog::logger::setup_logging` (IAMRusty, Hive, Telegraph, Manifesto — `configuration` → `main`). **`sentinel-sync`** et **`monolith`** initialisent `tracing_subscriber::fmt` eux-mêmes (`sentinel-sync/src/main.rs`, `monolith/src/runtime.rs`).

## Conséquences

- Un service active des features rustycog ; il ne vend pas rustycog comme binaire métier.
- Ne pas ajouter `rustycog` à `workspace.members` ni recréer `rustycog-macros` sans ADR.
- Ne pas exiger `setup_logging` rustycog sur sentinel-sync / monolithe (0404, 0303).

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| rustycog membre du workspace | Submodule + path dep ; crates.io 0.1.1 diverge (rdkafka via `events`/`full`) |
| Crate `rustycog-macros` dans le pin | Absente du checkout |
| `setup_logging` unique y compris worker / monolithe | Ils restent sur `tracing_subscriber::fmt` |

## Non décidé ici

- Remontée Kafka dans `full` vs feature `kafka` dédiée.
- Unification logging worker / monolithe vers `rustycog-logger`.
- Publication / bump du submodule.

## Références

- Submodule : `.gitmodules` (`path = rustycog`)
- Crate : `rustycog/Cargo.toml` (`[features]`)
- Workspace : `Cargo.toml` (`members` sans rustycog ; `workspace.dependencies.rustycog-framework`)
- Logging métier : `*/configuration/src/lib.rs` + `*/src/main.rs` (4 slices)
- Logging hors slice : `sentinel-sync/src/main.rs`, `monolith/src/runtime.rs`
- Preuve : path dep feature-gated ; macros absentes ; 4 × `setup_logging` vs 2 × `tracing_subscriber::fmt`
