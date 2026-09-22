---
title: >-
  ADR-0004 — gateway de capacités et KV plateforme
category: decisions
tags: [architecture, security, visibility/internal]
status: accepted
feature_status: implemented
sources:
  - docs/adr/0004-apparatus-capability-gateway.md
  - docs/adr/0007-closeout.md
  - apparatus-contracts/src/ports.rs
summary: >-
  Toute I/O via gateway ; KV namespacé par binding ; jamais de bearer IAM
  au plugin. Réalité Implemented (A-DEC 2026-09-20) : gateway = Lazaret
  T5–T14b. Non décidé : APP-05 A/B, APP-03, APP-06.
provenance:
  extracted: 0.88
  inferred: 0.12
  ambiguous: 0.00
created: 2026-09-12T09:30:00Z
updated: 2026-09-20T15:55:00Z
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

## Réalité : Implemented

- Taxonomie minimale (`project.read`, `storage.kv.read/write`), port `KvStore`, KV référence namespacé. P1 T5 : grants inchangés, revoke→403 TTL0, 0 nouveau type FGA.
- Gateway = Lazaret T5–T14b (`POST /invoke`, connecteurs nommés, KV Postgres+Redis, OpenBao T12, `kv_purge`, session mTLS T11b). Preuve V1 privé : T7 `t7_public_project_without_consent_does_not_open_call`.

## Non décidé ici

### APP-05 — lecture anonyme / ops publiques (non tranché)

Ticket dans le canon 0004. Ce n’est **pas** une ADR. **Statut : Non décidé.** P3 n’a pas préempté.

Question : surface Apparatus anonyme / UI publique / jobs sur projet public, un jour ?

V1 déjà livré : public n’ouvre pas invoke / KV / secrets (T7). Options **non choisies** : **A** — ratifier V1 privé durable (won't do anonyme) ; docs + fail-closed ; cheap ; préempte 0007 L189. **B** — lame publique limitée plus tard (nouvelle ADR produit) ; identité anonyme ; ops explicitement publiques sans hériter invoke (0004 L48 rejeté) ; UI/P5, jobs, OpenFGA distinct, tests de non-fuite. Pointeur : 0007 point 13 / L288.

- `APP-03` (rétention / purge).
- `APP-06` (quotas chiffrés, SLO).

Produit secrets et transport identité : ADR-0007 (T12 / T3+T11b), plus ici.

## Related

- [[projects/manifesto/concepts/apparatus-capabilities-and-isolation]]
- [[projects/manifesto/concepts/component-instance-permissions]]
- [[projects/manifesto/references/0007-closeout]]
- [[projects/manifesto/decisions/0008-apparatus-p4-k8s]]
