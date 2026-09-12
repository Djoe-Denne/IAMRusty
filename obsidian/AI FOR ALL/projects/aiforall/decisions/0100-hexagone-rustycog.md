---
title: "ADR 0100–0103 — hexagone RustyCog et crates"
category: decisions
tags: [architecture, rustycog, hexagonal, visibility/internal]
aliases: [hexagone rustycog, crates par couche]
summary: "Quatre services métier = vertical slices. Une crate par couche. setup = unique composition root. Ports + factory de commandes à clé string."
created: 2026-09-12T10:20:00Z
updated: 2026-09-12T10:20:00Z
sources:
  - docs/adr/0100-services-metier-hexagonaux-rustycog.md
  - docs/adr/0101-crates-par-couche-hexagonale.md
  - docs/adr/0102-setup-composition-root.md
  - docs/adr/0103-ports-adapters-command-factory.md
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
---

# ADR 0100–0103 — hexagone RustyCog et crates

Canon : `docs/adr/0100`–`0103`. Hub : [[projects/aiforall/decisions/index]]. Toutes **Accepted / Implemented**.

## 0100 — Quatre vertical slices

**IAMRusty**, **Hive**, **Telegraph**, **Manifesto** sont les seuls services métier hexagonaux. Golden path scaffold = [[projects/manifesto/manifesto]]. Golden path IdP = [[projects/iamrusty/iamrusty]].

Hors slice : [[projects/sentinel-sync/sentinel-sync]] (0303), `oodhive-monolith` (0404), crate `readiness` (0405), crates Apparatus P0 (0406).

Préfixes : `/iam`, `/telegraph`, `/hive`, `/manifesto`.

Logging : la revue d’août disait encore « Manifesto hand-rolled » ; le code actuel réexporte `rustycog::logger::setup_logging` dans les quatre `configuration`. La page [[concepts/architecture-coherence-across-services]] est caduque sur ce point. ^[inferred]

## 0101 — Une crate par couche

| Couche | Rôle |
|---|---|
| `domain` | entités, ports, services domaine — pas de SeaORM/Axum |
| `application` | use cases, commandes, factory de registre |
| `infra` | adapters (DB, HTTP sortant, JWT/OAuth, outbox) |
| `http` | `RouteBuilder`, handlers minces |
| `configuration` | config typée `rustycog-config` |
| `migration` | schéma SeaORM |
| `setup` | composition root |

Plus le binaire `*-service` (`src/main.rs`) et `tests/` d’intégration (0200). Les crates `*-events` sont un contrat workspace (0300), pas une 8ᵉ couche. Noms HTTP divergents (`hive-http` vs `*-http_server`) = choix local.

Cela clôt l’ancienne question ouverte de [[projects/iamrusty/concepts/hexagonal-architecture]] : `configuration` et `setup` **sont** des crates de premier rang. ^[inferred]

## 0102 — `setup` unique composition root

Seul `setup/src/app.rs` câble pool, publishers, repos, use cases, factory, extractor JWT, checker, `AppState`. Les handlers : parse → commande → `command_service.execute`. Le binaire ne câble pas les adapters. Le monolithe compose `create_router` / `create_prefixed_router`, **jamais** `run()`.

IAM injecte `InMemoryPermissionChecker` pour satisfaire `AppState` (IdP, pas un second root).

## 0103 — Ports et factory à clé string

Le domaine ne connaît que des ports. Infra = adapters. HTTP = adapter entrant. Commandes enregistrées par **clé string** (`ManifestoCommandRegistryFactory`, `HiveCommandRegistryFactory`, `TelegraphCommandRegistryFactory`, `CommandRegistryFactory` IAM). Une route sans `register` n’est pas une commande. L’OpenAPI Hive plus large que le registre reste une divergence, pas une permission de skip.

Mapping d’erreurs encore **local** (`http/src/error.rs`), pas `ServiceError` unifié.

## Related

- [[skills/building-rustycog-services]]
- [[concepts/shared-rust-microservice-sdk]]
- [[projects/aiforall/decisions/0400-services-runtime]]
