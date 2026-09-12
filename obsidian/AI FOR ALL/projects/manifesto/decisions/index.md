---
title: "Apparatus — index des ADR"
category: references
tags: [architecture, components, visibility/internal]
status: accepted
feature_status: partial
summary: "Hub ADR Apparatus Accepted ; Réalité Partial. P0 = contrats + KV de référence, P1 = persistance Manifesto, pas Factory ni host."
created: 2026-09-10T06:32:00Z
updated: 2026-09-12T10:20:00Z
sources:
  - docs/adr/README.md
  - docs/adr/0001-apparatus-binding-owned-by-manifesto.md
  - docs/adr/0002-apparatus-contract-first.md
  - docs/adr/0003-apparatus-untrusted-plugin.md
  - docs/adr/0004-apparatus-capability-gateway.md
  - docs/adr/0005-apparatus-same-protocol-valid-verified.md
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

Les ADR Apparatus ci-dessous sont **Accepted** depuis la ratification orchestrée du 10 septembre 2026. `Accepted` fixe une cible ; il ne signifie pas `Implemented`. Les pages [[projects/manifesto/concepts/apparatus-platform]] et suivantes restent le rationnel, et la baseline antérieure aux ADR est conservée dans [[projects/manifesto/references/apparatus-implementation-plan#Recommandations du 9 septembre — baseline de traçabilité]].

## Vague 1

| ADR | Décision | Réalité |
|---|---|---|
| [0001](../../../../../docs/adr/0001-apparatus-binding-owned-by-manifesto.md) — [[projects/manifesto/decisions/0001-apparatus-binding]] | Binding = extension 1:1 de `ProjectComponent`, propriété métier Manifesto | Partial |
| [0002](../../../../../docs/adr/0002-apparatus-contract-first.md) — [[projects/manifesto/decisions/0002-apparatus-contracts]] | Contrats + Apparatus KV de référence avant Factory et host | Partial |
| [0003](../../../../../docs/adr/0003-apparatus-untrusted-plugin.md) — [[projects/manifesto/decisions/0003-apparatus-untrusted]] | Code auteur hors des processus privilégiés | Partial |
| [0004](../../../../../docs/adr/0004-apparatus-capability-gateway.md) — [[projects/manifesto/decisions/0004-apparatus-gateway]] | Gateway de capacités ; KV plateforme ; pas de bearer IAM | Partial |
| [0005](../../../../../docs/adr/0005-apparatus-same-protocol-valid-verified.md) — [[projects/manifesto/decisions/0005-apparatus-protocol]] | Même protocole ; admission, `VALID`, `VERIFIED` et installabilité distincts | Partial |

Preuve P0 (contrats, pas persistance) : [[projects/manifesto/concepts/apparatus-p0-contracts]]. Preuve P1 (persistance, non commitée) : [[projects/manifesto/concepts/apparatus-p1-persistence]]. Les 5 ADR restent `Partial` : décisions `Accepted` intactes, preuves P0.1/P1 ajoutées le 12 sept. (review + contre-review docs, compteurs 36/42/78).

## Conception (pas des ADR)

- [[projects/manifesto/concepts/apparatus-p0-contracts]] — crates livrées, distinctes de la cible Factory/host
- [[projects/manifesto/concepts/apparatus-platform]]
- [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]]
- [[projects/manifesto/concepts/apparatus-capabilities-and-isolation]]
- [[projects/manifesto/references/apparatus-factory-and-distribution]]
- [[projects/manifesto/references/apparatus-ui-and-protocol]]
- [[projects/manifesto/references/apparatus-implementation-plan]]
- [[projects/manifesto/references/apparatus-source-reconciliation]]

Arbitrages encore ouverts (`APP-01` … `APP-07`) : [[projects/manifesto/references/apparatus-implementation-plan#Questions encore ouvertes]].

## Vague 2 — architecture actuelle (extrait Manifesto)

Photographie rétroactive du 12 septembre 2026. Extrait Manifesto ; index plateforme : [[projects/aiforall/decisions/index]]. Canon : `docs/adr/README.md`.

| ADR | Décision | Réalité |
|---|---|---|
| [0401](../../../../../docs/adr/0401-manifesto-projets-composants-acl-cas.md) — [[projects/aiforall/decisions/0400-services-runtime]] | Manifesto = projets, composants, membership SQL, CAS, ACL+outbox atomiques | Implemented |
| [0406](../../../../../docs/adr/0406-crates-apparatus-p0-pas-le-host.md) — [[projects/aiforall/decisions/0400-services-runtime]] | `apparatus-contracts` + KV réf. = P0 ; Factory / host / gateway hors livré — voir 0001–0005 | Partial |
