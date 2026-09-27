---
title: "ADR-0605 — gold path local Kind (Proposed)"
category: decisions
tags: [architecture, platform, kubernetes, visibility/internal]
status: proposed
feature_status: implemented
sources:
  - docs/adr/0605-gold-path-kind-j3-dns-attach.md
  - docs/platform-local-gold-case-implementation-guide.md
  - docs/platform-local-prod-mimic-implementation-guide.md
summary: >-
  Gold path Kind = HTTP 200 via le monolithe J3 seul Lazaret.
  Service ClusterIP du même nom que le Pod. Statut Proposed, Réalité Implemented.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-09-27T09:20:00Z
updated: 2026-09-27T09:20:00Z
---

# ADR-0605 — gold path local Kind

Canon : `docs/adr/0605-gold-path-kind-j3-dns-attach.md`. Hub : [[projects/aiforall/decisions/index]]. S’appuie sur [[projects/aiforall/decisions/0604-j3-overlay-demo-monolith]] sans en faire l’unité prod. Canon cluster 4+1 : [[projects/aiforall/decisions/0601-cluster-topology]].

## Statut : Proposed (2026-09-25)

Réalité **Implemented** (accord de coding ; le Statut reste Proposed) : writer d’attache, locator DNS, Service operator, Job Adm-A (`SA admit-sign`), `just prove-gold` → invoke in-cluster HTTP 200. Dette : Transit sur le même OpenBao `-dev` (D-TRANSIT-TCB).

- Le monolithe dans le cluster `aiforall-local` est le seul Lazaret du parcours. Curl hôte `127.0.0.1:8080` n’est pas la preuve.
- L’operator crée un Service ClusterIP **du même nom que le Pod**. Lazaret ne parle pas à l’API kube. Hostname `plugin-{32 hex du digest}`. URL `http://plugin-….apparatus-plugins.svc:8080`.
- Le digest comme clé DNS vaut pour ce gold path Kind. Le figer en modèle prod est interdit : [[projects/manifesto/decisions/0009-0011-gates-preprod]].
- `insert_managed_binding` remplit `digest` et `declared_capabilities`. Pas de seed SQL, pas de nouvelle route `/components`.
- Cluster IT `apparatus-p4-it` hors de ce parcours. Calico v3.29.7 est pinné sur `aiforall-local` seulement ; 0604 « Calico hors J3 » reste vrai pour l’overlay démo.

sentinel-sync reste un process à part, non nesté dans `oodhive-monolith` (ADR-0404). [[projects/sentinel-sync/sentinel-sync]]

## Related

- [[projects/aiforall/decisions/0603-tranche-locale]]
- [[projects/manifesto/decisions/0008-apparatus-p4-k8s]]
- [[projects/lazaret/lazaret]]
- [[journal/2026-09-27]]
