---
title: >-
  ADR-0005 — même protocole, VALID distinct de VERIFIED
category: decisions
tags: [architecture, components, visibility/internal]
sources:
  - docs/adr/0005-apparatus-same-protocol-valid-verified.md
  - apparatus-reference-kv/apparatus.toml
summary: >-
  Un seul protocole officiel/communautaire ; admission, VALID, VERIFIED, installabilité = 4 notions distinctes.
provenance:
  extracted: 0.90
  inferred: 0.10
  ambiguous: 0.00
created: 2026-09-12T09:30:00Z
updated: 2026-09-12T09:30:00Z
---

# ADR-0005 — même protocole, VALID distinct de VERIFIED

Canon : `docs/adr/0005-apparatus-same-protocol-valid-verified.md`. Hub : [[projects/manifesto/decisions/index]].

## Décision (Accepted 2026-09-10)

- Un seul protocole backend/UI/capacités pour tous publishers ; `VERIFIED` n'ajoute aucune méthode, ne saute ni gateway ni confinement.
- Admission = attestation indépendante (publisher, plugin, harness, éditorial) ; enregistrer une release ne l'auto-admet pas.
- `VALID` = digest + version politique conformance + rapport attesté. `VERIFIED` = signal éditorial distinct, révocable. Installabilité = digest admis + consentement + politique locale.
- Zéro bypass officiel dans contrats, SDK, harness ; la référence emprunte le même contrat wire (`manifesto-apparatus/1`).

## Réalité : Partial

- Aucun champ `trusted_*` dans DTO/manifeste ; harness et tests ne produisent ni `VALID` ni `VERIFIED`.
- P1 : 0 `VALID`/`VERIFIED`, 0 second UUID, 5 routes, FGA 5 types (T3+T5+T6+T7). Aucun pipeline d'admission n'existe.

## Non décidé ici

`APP-02` (publishers), Factory/OCI (P4), drain/destruction/rétention après révocation (P4/P6).

## Related

- [[projects/manifesto/references/apparatus-factory-and-distribution]]
- [[projects/manifesto/concepts/apparatus-p1-persistence]]
