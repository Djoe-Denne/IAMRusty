---
title: "ADR 0300–0303 — contrat events, outbox, JWT, sentinel-sync"
category: decisions
tags: [architecture, events, authz, visibility/internal]
aliases: [outbox, jwt openfga]
summary: "Crates *-events = contrat sans transport. Outbox same-txn Hive/Manifesto seulement (Partial). AuthN HS256 ; AuthZ OpenFGA. sentinel-sync = worker, pas un hexagone."
created: 2026-09-12T10:20:00Z
updated: 2026-09-12T10:20:00Z
sources:
  - docs/adr/0300-crates-events-contrat-sans-transport.md
  - docs/adr/0301-outbox-transactionnel-rustycog.md
  - docs/adr/0302-authn-jwt-authz-openfga.md
  - docs/adr/0303-sentinel-sync-worker-fga.md
  - docs/platform/events-outbox.md
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
---

# ADR 0300–0303 — events, outbox, AuthN/AuthZ

Canon : `docs/adr/0300`–`0303`. Hub : [[projects/aiforall/decisions/index]].

## 0300 — Contrat sans transport (Implemented)

`iam-events`, `hive-events`, `manifesto-events`, `telegraph-events` portent les types de domaine. Le transport = `QueueConfig` : `Sqs` | `Kafka` | `Disabled`. `Disabled` est un transport valide.

`apparatus-events/` existe sur disque mais **hors** `workspace.members`. **NATS** n’est pas un transport plateforme.

La file physique (`telegraph-events`, `sentinel-sync-events`) est un câblage d’opérateur, pas le schéma.

## 0301 — Outbox transactionnel (Partial)

Publication durable = outbox rustycog, pas un `send` après `COMMIT`. La file n’est **pas** la vérité métier.

| Service | Réalité |
|---|---|
| Hive, Manifesto | outbox **same-txn** que l’agrégat |
| IAMRusty | outbox dans une **txn séparée** (perte possible si la 2ᵉ txn échoue) |
| Telegraph | **pas** d’outbox |

Une IT ne doit pas traiter l’absence de message SQS comme preuve d’absence d’état métier. Voir [[projects/rustycog/references/rustycog-outbox]].

## 0302 — JWT HS256 + OpenFGA (Implemented)

AuthN = Bearer HS256 via rustycog-http `UserIdExtractor` (`iss=iamrusty`, `aud=aiforall`). IAM est l’émetteur. AuthZ = OpenFGA `with_permission_on` (Hive, Manifesto, Telegraph). IAM = IdP, `InMemoryPermissionChecker`, pas de PDP FGA.

L’impersonation de `docs/project/Archi.md` est **caduque**. JWKS existe sur le routeur IAM mais l’extractor ne l’utilise pas — [[projects/aiforall/concepts/jwt-issuer-vs-consumer]].

## 0303 — sentinel-sync = worker

Worker événements → tuples FGA, **pas** une vertical slice HTTP, **absent** du Compose et du monolithe. Sans lui, OpenFGA dérive après un event AuthZ (membership, visibilité). Voir [[projects/sentinel-sync/sentinel-sync]].

## Related

- [[concepts/event-driven-microservice-platform]]
- [[concepts/openfga-as-authorization-engine]]
- [[projects/sentinel-sync/concepts/db-to-openfga-reconcile]]
