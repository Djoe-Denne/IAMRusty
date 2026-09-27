---
title: "ADR-0604 — J3 overlay démo monolithe kind invoke (Proposed)"
category: decisions
tags: [architecture, platform, kubernetes, visibility/internal]
status: proposed
feature_status: implemented
sources:
  - docs/adr/0604-j3-overlay-demo-monolith-kind-invoke.md
  - docs/platform-local-monolith-kind-implementation-plan.md
summary: >-
  Overlay kind démo : Deployment oodhive-monolith, plugins vers
  /lazaret/invoke. Statut Proposed, Réalité Implemented. Pas l’unité prod 4+1.
created: 2026-09-25T12:36:00Z
updated: 2026-09-27T09:20:00Z
provenance:
  extracted: 0.94
  inferred: 0.04
  ambiguous: 0.02
---

# ADR-0604 — J3 overlay démo monolithe kind

Canon : `docs/adr/0604-j3-overlay-demo-monolith-kind-invoke.md`. Hub : [[projects/aiforall/decisions/index]]. Plan (pas canon) : `docs/platform-local-monolith-kind-implementation-plan.md`. Cadre : [[projects/aiforall/decisions/0601-cluster-topology]], [[projects/aiforall/decisions/0603-tranche-locale]], [[projects/aiforall/decisions/0600-cloud-portable]]. Dual : ADR-0404.

## Statut : Proposed (2026-09-25)

Réalité **Implemented** (HEAD `2d88b0e`, accord de coding ; Statut inchangé). Overlay démo séparé de M2 : Deployment `oodhive-monolith`, image `aiforall-oodhive-monolith:j3`, `just deploy-j3`. Preuve plugins → `/lazaret/invoke`. Faux amis extra-port / hostNetwork interdits. Ne SuperSède **pas** 0601, 0404, 0600, 0603, 0008. Le HTTP 200 métier du gold path est [[projects/aiforall/decisions/0605-gold-path-kind]], pas cette ADR. J4 = retirer l’overlay (plus tard). « Calico hors J3 » reste vrai pour cet overlay ; Calico v3.29.7 sur `aiforall-local` appartient au gold path.
