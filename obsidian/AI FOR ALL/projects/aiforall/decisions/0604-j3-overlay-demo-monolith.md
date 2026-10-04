---
title: "ADR-0604 — J3 overlay démo monolithe kind invoke (Proposed)"
category: decisions
tags: [architecture, platform, kubernetes, visibility/internal]
status: proposed
feature_status: partial
sources:
  - docs/adr/0604-j3-overlay-demo-monolith-kind-invoke.md
  - docs/platform-local-monolith-kind-implementation-plan.md
summary: >-
  Overlay kind démo : Deployment oodhive-monolith, plugins vers
  /lazaret/invoke. Statut Proposed, Réalité courante Partial ; preuve J3 historique conservée. Pas l’unité prod 4+1.
created: 2026-09-25T12:36:00Z
updated: 2026-10-04
provenance:
  extracted: 0.94
  inferred: 0.04
  ambiguous: 0.02
---

# ADR-0604 — J3 overlay démo monolithe kind

Canon : `docs/adr/0604-j3-overlay-demo-monolith-kind-invoke.md`. Hub : [[projects/aiforall/decisions/index]]. Plan (pas canon) : `docs/platform-local-monolith-kind-implementation-plan.md`. Cadre : [[projects/aiforall/decisions/0601-cluster-topology]], [[projects/aiforall/decisions/0603-tranche-locale]], [[projects/aiforall/decisions/0600-cloud-portable]]. Dual : ADR-0404.

## Statut : Proposed (2026-09-25)

Réalité **Partial** au 2026-10-04, Statut Proposed inchangé. Ancien Implemented (HEAD `2d88b0e`, accord de coding) et Job invoke HTTP401 conservés comme preuve **historique** ; sources `ops/deploy/apps/overlays/kind-demo-monolith/`, image J3 et helper présents. Root modifié non compilé/testé au complet ; pas de nouveau résultat J3/gold. Pas de Supersede 0601/0404/0600/0603/0008. Calico et Compose décrits au gold/J3 restent leurs limites historiques. [[projects/aiforall/decisions/0606-local-full-kind-isole]] isole le nouveau lab ; aucun transfert de preuve ou delete/recreate du legacy étranger. Gold : [[projects/aiforall/decisions/0605-gold-path-kind]].
