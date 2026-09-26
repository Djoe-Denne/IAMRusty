---
title: "ADR-0604 — J3 overlay démo monolithe kind invoke (Proposed)"
category: decisions
tags: [architecture, platform, kubernetes, visibility/internal]
status: proposed
feature_status: unimplemented
sources:
  - docs/adr/0604-j3-overlay-demo-monolith-kind-invoke.md
  - docs/platform-local-monolith-kind-implementation-plan.md
summary: >-
  Canon : docs/adr/0604. Écart J3 = overlay kind démo non canon :
  Deployment oodhive-monolith prouve plugins → /lazaret/invoke ;
  pas SuperSéde 0601 (canon cluster = 4+1). M2 stub reste 0603.
created: 2026-09-25T12:36:00Z
updated: 2026-09-25T12:36:00Z
provenance:
  extracted: 0.94
  inferred: 0.04
  ambiguous: 0.02
---

# ADR-0604 — J3 overlay démo monolithe kind

Canon : `docs/adr/0604-j3-overlay-demo-monolith-kind-invoke.md`. Hub : [[projects/aiforall/decisions/index]]. Plan (pas canon) : `docs/platform-local-monolith-kind-implementation-plan.md`. Cadre : [[projects/aiforall/decisions/0601-cluster-topology]], [[projects/aiforall/decisions/0603-tranche-locale]], [[projects/aiforall/decisions/0600-cloud-portable]]. Dual : ADR-0404.

## Statut : Proposed (2026-09-25)

Réalité **Unimplemented**. Overlay démo séparé de M2 ; preuve = plugins POST `/lazaret/invoke` (401 métier OK) ; faux amis extra-port / hostNetwork interdits. Ne SuperSède **pas** 0601, 0404, 0600, 0603, 0008. J4 = retirer overlay (plus tard).
