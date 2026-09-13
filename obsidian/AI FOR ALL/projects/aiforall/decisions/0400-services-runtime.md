---
title: "ADR 0400–0406 — services, dual runtime, P0 Apparatus"
category: decisions
tags: [architecture, platform, microservices, visibility/internal]
summary: "IAM = IdP ; Manifesto projets/ACL/CAS + ticker P2 Partial. Dual runtime. Apparatus Factory/host hors livré (0406)."
created: 2026-09-12T10:20:00Z
updated: 2026-09-13T10:25:00Z
sources:
  - docs/adr/0400-iamrusty-identite-hexagonale.md
  - docs/adr/0401-manifesto-projets-composants-acl-cas.md
  - docs/adr/0402-hive-organisations.md
  - docs/adr/0403-telegraph-notifications-event-driven.md
  - docs/adr/0404-runtime-microservices-et-monolithe.md
  - docs/adr/0405-readiness-crate-partagee.md
  - docs/adr/0406-crates-apparatus-p0-pas-le-host.md
  - docs/adr/0006-apparatus-p2-reconciliation-in-process.md
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
---

# ADR 0400–0406 — services et runtimes

Canon : `docs/adr/0400`–`0406`. Hub : [[projects/aiforall/decisions/index]]. Hexagone commun : [[projects/aiforall/decisions/0100-hexagone-rustycog]].

## 0400 — IAMRusty = IdP (Implemented)

OAuth, tokens, émetteur JWT. Pas OpenFGA. Checker mémoire uniquement pour `AppState`. Hub : [[projects/iamrusty/iamrusty]].

## 0401 — Manifesto = projets / composants / ACL / CAS (Implemented)

Membership SQL, CAS, ACL + outbox atomiques. Golden path scaffold. Voir [[projects/manifesto/concepts/immediate-membership-acl]], [[projects/manifesto/concepts/membership-restore-and-cas]]. Index Apparatus : [[projects/manifesto/decisions/index]].

## 0402 — Hive = organisations (Implemented)

Orgs, invitations, membership. Divergence connue : OpenAPI plus large que le registre de commandes. Hub : [[projects/hive/hive]].

## 0403 — Telegraph = notifications event-driven (Implemented)

HTTP étroit (lecture notifications). Consommateur d’events IAM ; SQS+SMTP allumés en IT (0202). Pas d’outbox (0301). Hub : [[projects/telegraph/telegraph]].

## 0404 — Dual runtime (Implemented)

Standalones **et** `oodhive-monolith` (préfixes identiques). Le monolithe compose les routeurs, **n’appelle pas** `run()`, **ne remplace pas** les microservices. `sentinel-sync` hors dual. Voir [[projects/aiforall/references/modular-monolith-runtime]] et [[projects/aiforall/skills/running-aiforall-runtime-modes]].

## 0405 — crate `readiness` partagée (Implemented)

Health `/ready` assemblé dans `setup`, pas un second composition root.

## 0406 — Apparatus P0 seulement (Partial)

`apparatus-contracts` + KV de référence = P0 livré. Factory, host, gateway réseau = **hors livré**. Le ticker P2 in-process Manifesto existe (Partial, ADR-0006) — le canon 0406 dit encore « contrôleur / workers » non livrés : **périmé** pour le worker in-process, vrai pour K8s/Factory. ^[ambiguous] Voir [[projects/manifesto/concepts/apparatus-p2-reconciliation]] et [[projects/manifesto/decisions/0006-apparatus-p2-reconciliation]].

## Related

- [[concepts/architecture-coherence-across-services]]
- [[projects/aiforall/aiforall]]
- [[projects/manifesto/concepts/apparatus-p2-reconciliation]]
- [[projects/manifesto/decisions/0006-apparatus-p2-reconciliation]]
