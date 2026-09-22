---
title: >-
  Apparatus — index des ADR
category: references
tags: [architecture, components, visibility/internal]
status: accepted
feature_status: implemented
summary: >-
  Hub ADR Apparatus 0001–0008 Accepted. 0002/0003/0005/0008 Implemented
  (A-DEC 2026-09-22, Kind V1) ; 0001 Partial ; 0004/0006/0007 Implemented.
  Dette D-TRANSIT-TCB / D-ADMB / D-PROD. APP-05 ouvert.
created: 2026-09-10T06:32:00Z
updated: 2026-09-22T14:00:00Z
sources:
  - docs/adr/README.md
  - docs/adr/0001-apparatus-binding-owned-by-manifesto.md
  - docs/adr/0002-apparatus-contract-first.md
  - docs/adr/0003-apparatus-untrusted-plugin.md
  - docs/adr/0004-apparatus-capability-gateway.md
  - docs/adr/0005-apparatus-same-protocol-valid-verified.md
  - docs/adr/0006-apparatus-p2-reconciliation-in-process.md
  - docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md
  - docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md
  - docs/adr/0008-app01-reconciliation.md
  - docs/adr/0007-closeout.md
  - docs/adr/0008-closeout.md
  - docs/apparatus-p4-implementation-prompt.md
  - docs/adr/0401-manifesto-projets-composants-acl-cas.md
  - docs/adr/0406-crates-apparatus-p0-pas-le-host.md
provenance:
  extracted: 0.80
  inferred: 0.15
  ambiguous: 0.05
---

# Apparatus — index des ADR

Les **décisions** vivent dans le handbook git, pas dans les notes de conception :

- Index : `docs/adr/README.md`
- Template : `docs/adr/template.md`

Les ADR Apparatus 0001–0008 sont **Accepted**. `Accepted` fixe une cible ; il ne signifie pas `Implemented`. Les pages [[projects/manifesto/concepts/apparatus-platform]] et suivantes restent le rationnel, et la baseline antérieure aux ADR est conservée dans [[projects/manifesto/references/apparatus-implementation-plan#Recommandations du 9 septembre — baseline de traçabilité]].

## Vague 1

| ADR | Décision | Réalité |
|---|---|---|
| [0001](../../../../../docs/adr/0001-apparatus-binding-owned-by-manifesto.md) — [[projects/manifesto/decisions/0001-apparatus-binding]] | Binding = extension 1:1 de `ProjectComponent`, propriété métier Manifesto | Partial |
| [0002](../../../../../docs/adr/0002-apparatus-contract-first.md) — [[projects/manifesto/decisions/0002-apparatus-contracts]] | Contrats + Apparatus KV de référence avant Factory et host | Implemented |
| [0003](../../../../../docs/adr/0003-apparatus-untrusted-plugin.md) — [[projects/manifesto/decisions/0003-apparatus-untrusted]] | Code auteur hors des processus privilégiés | Implemented |
| [0004](../../../../../docs/adr/0004-apparatus-capability-gateway.md) — [[projects/manifesto/decisions/0004-apparatus-gateway]] | Gateway de capacités ; KV plateforme ; pas de bearer IAM | Implemented |
| [0005](../../../../../docs/adr/0005-apparatus-same-protocol-valid-verified.md) — [[projects/manifesto/decisions/0005-apparatus-protocol]] | Même protocole ; admission, `VALID`, `VERIFIED` et installabilité distincts | Implemented |
| [0006](../../../../../docs/adr/0006-apparatus-p2-reconciliation-in-process.md) — [[projects/manifesto/decisions/0006-apparatus-p2-reconciliation]] | Réconciliation P2 = contrôleur in-process Manifesto, sans infra réelle | Implemented |
| [0007](../../../../../docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md) — [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]] | Frontière P3 = BC Lazaret, distinct de Manifesto ; 0006 G et E restent | Implemented |
| [0008](../../../../../docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md) — [[projects/manifesto/decisions/0008-apparatus-p4-k8s]] | Moteur P4 = K8s hors Manifesto ; Cosign + Transit ; Adm-A ; operator+Jobs ; enveloppe ≠ image CRI ; 4 SA | Implemented |

Preuve P0 : [[projects/manifesto/concepts/apparatus-p0-contracts]] ; ADR-0002 **Implemented** (A-DEC 2026-09-22 ; M1–M6 ; Factory/host = P5/P6 après). Preuve P1 : [[projects/manifesto/concepts/apparatus-p1-persistence]]. ADR-0006 Accepted 2026-09-12 ; Réalité Implemented (T1–T7 + writer §D). Concept : [[projects/manifesto/concepts/apparatus-p2-reconciliation]]. ADR-0007 Accepted 2026-09-13 ; Réalité Implemented (A-DEC 2026-09-20 ; T1–T14b ; holes hors-jalon APP-05 encore ouvert, 0006 G/E, pas K8s-as-P3, pas de second protocole). Service : [[projects/lazaret/lazaret]]. Mesh : [[projects/aiforall/concepts/https-platform-mesh]]. Pointeur : [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]]. ADR-0008 Accepted 2026-09-20 ; Réalité **Implemented** (A-DEC 2026-09-22 ; T2–T12 + M1–M6 ; Kind V1 ; dette D-TRANSIT-TCB / D-ADMB / D-PROD). Closeout : `docs/adr/0008-closeout.md`. Pointeur : [[projects/manifesto/decisions/0008-apparatus-p4-k8s]].

## Conception (pas des ADR)

- [[projects/manifesto/concepts/apparatus-p0-contracts]] — crates livrées, distinctes de la cible Factory/host
- [[projects/manifesto/concepts/apparatus-p4-operator]] — crate operator Kind/zot (0008 Implemented, dette TCB)
- [[projects/manifesto/concepts/apparatus-platform]]
- [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]]
- [[projects/manifesto/concepts/apparatus-capabilities-and-isolation]]
- [[projects/manifesto/references/apparatus-factory-and-distribution]]
- [[projects/manifesto/references/apparatus-ui-and-protocol]]
- [[projects/manifesto/references/apparatus-implementation-plan]]
- [[projects/manifesto/references/apparatus-source-reconciliation]]

Arbitrages encore ouverts (`APP-02` … `APP-07`) : [[projects/manifesto/references/apparatus-implementation-plan#Questions encore ouvertes]]. **`APP-01`** = ADR-0008 (Accepted / Implemented ; dette TCB).

## Vague 2 — architecture actuelle (extrait Manifesto)

Photographie rétroactive du 12 septembre 2026. Extrait Manifesto ; index plateforme : [[projects/aiforall/decisions/index]]. Canon : `docs/adr/README.md`.

| ADR | Décision | Réalité |
|---|---|---|
| [0401](../../../../../docs/adr/0401-manifesto-projets-composants-acl-cas.md) — [[projects/aiforall/decisions/0400-services-runtime]] | Manifesto = projets, composants, membership SQL, CAS, ACL+outbox atomiques | Implemented |
| [0406](../../../../../docs/adr/0406-crates-apparatus-p0-pas-le-host.md) — [[projects/aiforall/decisions/0400-services-runtime]] | `apparatus-contracts` + KV réf. = P0 ; Factory / host hors livré — Lazaret P3 Implemented (0007) ; moteur P4 = 0008 Implemented (Kind V1) | Partial |

## Related

- Plateforme cluster (**pas** Apparatus) : [[projects/aiforall/decisions/0600-cloud-portable]] / [[projects/aiforall/decisions/0601-cluster-topology]] — P4 (0008) s'insère dans cette topologie ; 0600/0601 ne sont pas des ADR Apparatus.
