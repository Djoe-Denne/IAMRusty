---
title: >-
  ADR-0002 — contrats versionnés et KV de référence d'abord
category: decisions
tags: [architecture, components, visibility/internal]
sources:
  - docs/adr/0002-apparatus-contract-first.md
  - apparatus-contracts/src/validation.rs
  - apparatus-reference-kv/apparatus.toml
summary: >-
  P0 avant Factory/host : apparatus.toml canonique, validateur unique, digest sha256, KV de référence. Partial.
provenance:
  extracted: 0.90
  inferred: 0.10
  ambiguous: 0.00
created: 2026-09-12T09:30:00Z
updated: 2026-09-12T09:30:00Z
---

# ADR-0002 — contrats versionnés et KV de référence d'abord

Canon : `docs/adr/0002-apparatus-contract-first.md`. Hub : [[projects/manifesto/decisions/index]].

## Décision (Accepted 2026-09-10)

- P0 avant P4/P5 : schéma canonique `apparatus.toml`, DTO release/binding/opération, API capacités, protocole backend privé (`manifesto-apparatus/1`).
- Identité release = SemVer manifeste ; identité installation = digest descripteur canonique ; `latest`/tags flottants = erreurs de validation.
- Une seule syntaxe (TOML), un seul validateur (Factory/CLI le réutiliseront).
- Apparatus de référence KV : backend Rust, stockage `kv-v1`, UI `schema`, zéro réseau.

## Réalité : Partial

- P0/P0.1 : `apparatus-contracts` + `apparatus-reference-kv`, 42/42 tests, CI job `apparatus-p0`. Voir [[projects/manifesto/concepts/apparatus-p0-contracts]].
- Restent hors code : macro, CLI `check/dev/publish`, Factory (P4), host (P5).

## Non décidé ici

Champs TOML de détail (itérables), commandes CLI, pipeline OCI, origine/sandbox/CSP du host.

## Related

- [[projects/manifesto/concepts/apparatus-p0-contracts]]
- [[projects/manifesto/references/apparatus-factory-and-distribution]]
