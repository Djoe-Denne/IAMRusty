---
title: >-
  ADR-0003 — code Apparatus non digne de confiance
category: decisions
tags: [architecture, security, visibility/internal]
sources:
  - docs/adr/0003-apparatus-untrusted-plugin.md
  - apparatus-contracts/src/harness.rs
summary: >-
  Plugin = processus OS et identité workload distincts ; managed-only V1 ; harness = double de test. Partial.
provenance:
  extracted: 0.88
  inferred: 0.12
  ambiguous: 0.00
created: 2026-09-12T09:30:00Z
updated: 2026-09-12T09:30:00Z
---

# ADR-0003 — code Apparatus non digne de confiance

Canon : `docs/adr/0003-apparatus-untrusted-plugin.md`. Hub : [[projects/manifesto/decisions/index]].

## Décision (Accepted 2026-09-10)

- Le code auteur s'exécute dans un processus OS et une identité workload distincts de Manifesto, du monolithe et des composants privilégiés. Séparation logique même espace d'adressage = insuffisant.
- V1 : runtime **managed** seulement, isolation par projet et par binding ; `shared`/`organization` hors V1.
- Le monolithe expose les API de contrôle mais ne charge jamais le binaire communautaire.
- Le harness in-process P0 est un double de test : aucun secret IAM, jamais `VALID`/`VERIFIED`.

## Réalité : Partial

- `TestHarness`/`InMemoryKv` TEST-ONLY (feature `test-harness`, absent prod), namespacé par `binding_id`.
- P1 : zéro workload tiers démarré (faux broker in-memory, 0 polling/worker), gate T7 vert. Aucun runtime isolé de production n'existe — `Partial` maintenu.

## Non décidé ici

`APP-01` (budget, CPU, moteur d'isolation, registry, signature), adaptateur prod, scale-to-zero.

## Related

- [[projects/manifesto/concepts/apparatus-capabilities-and-isolation]]
- [[projects/manifesto/concepts/apparatus-p1-persistence]]
