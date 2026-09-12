---
title: >-
  ADR-0004 — gateway de capacités et KV plateforme
category: decisions
tags: [architecture, security, visibility/internal]
sources:
  - docs/adr/0004-apparatus-capability-gateway.md
  - apparatus-contracts/src/ports.rs
summary: >-
  Toute I/O via gateway ; KV namespacé par binding ; jamais de bearer IAM au plugin. Contrat P0, gateway P3.
provenance:
  extracted: 0.88
  inferred: 0.12
  ambiguous: 0.00
created: 2026-09-12T09:30:00Z
updated: 2026-09-12T09:30:00Z
---

# ADR-0004 — gateway de capacités et KV plateforme

Canon : `docs/adr/0004-apparatus-capability-gateway.md`. Hub : [[projects/manifesto/decisions/index]].

## Décision (Accepted 2026-09-10)

Contrat normatif dès P0 ; gateway réseau implémentée en P3.

- Le plugin ne reçoit jamais bearer IAM, JWT interne ni secret HMAC.
- Autorisation à l'appel = ACL ∩ release admise ∩ capacités déclarées ∩ consentement ∩ desired/grant_revision ∩ politique opérateur. Capacité inconnue = refus.
- L'état DB ferme l'accès dès commit (suspension/révocation/retrait) ; projection FGA périmée insuffisante.
- Projet public n'ouvre ni invoke, ni KV, ni secrets (V1 privé, `APP-05` en suspens).
- État durable V1 = KV plateforme namespacé par binding (quota, CAS) ; secrets = références opaques ; egress = connecteurs nommés admis uniquement.

## Réalité : Partial

- Taxonomie minimale (`project.read`, `storage.kv.read/write`), port `KvStore`, KV référence namespacé.
- P1 T5 : grants inchangés, revoke→403 TTL0, 0 nouveau type FGA. Gateway réseau, identité workload, KV persistant = P3.

## Non décidé ici

`APP-05` (UI publique, fond), `APP-03` (rétention), `APP-06` (quotas), produit secrets, transport identité workload.

## Related

- [[projects/manifesto/concepts/apparatus-capabilities-and-isolation]]
- [[projects/manifesto/concepts/component-instance-permissions]]
