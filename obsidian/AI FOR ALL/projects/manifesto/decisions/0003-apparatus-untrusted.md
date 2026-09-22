---
title: >-
  ADR-0003 — code Apparatus non digne de confiance
category: decisions
tags: [architecture, security, visibility/internal]
sources:
  - docs/adr/0003-apparatus-untrusted-plugin.md
  - apparatus-contracts/src/harness.rs
summary: >-
  Plugin = processus OS et identité workload distincts ; managed-only V1 ; harness = double de test. Implemented (A-DEC 2026-09-22).
provenance:
  extracted: 0.88
  inferred: 0.12
  ambiguous: 0.00
created: 2026-09-12T09:30:00Z
updated: 2026-09-22T14:00:00Z
---

# ADR-0003 — code Apparatus non digne de confiance

Canon : `docs/adr/0003-apparatus-untrusted-plugin.md`. Hub : [[projects/manifesto/decisions/index]].

## Décision (Accepted 2026-09-10)

- Le code auteur s'exécute dans un processus OS et une identité workload distincts de Manifesto, du monolithe et des composants privilégiés. Séparation logique même espace d'adressage = insuffisant.
- V1 : runtime **managed** seulement, isolation par projet et par binding ; `shared`/`organization` hors V1.
- Le monolithe expose les API de contrôle mais ne charge jamais le binaire communautaire.
- Le harness in-process P0 est un double de test : aucun secret IAM, jamais `VALID`/`VERIFIED`.

## Réalité : Implemented

- Isolation Kind/Calico v3.29.7 + 4 SA sur le chemin invoke (M5–M6) ; plugin hors processus privilégiés.
- `TestHarness`/`InMemoryKv` TEST-ONLY (feature `test-harness`) — **peut rester** ; jamais VALID/VERIFIED.
- **APP-01** tranché par ADR-0008 (moteur K8s hors Manifesto) ; budget CPU numériques (`APP-06`) peuvent rester ouverts.

## Non décidé ici

Budget CPU numériques (`APP-06`), scale-to-zero, cluster prod (dette D-PROD).

## Related

- [[projects/manifesto/concepts/apparatus-capabilities-and-isolation]]
- [[projects/manifesto/concepts/apparatus-p1-persistence]]
