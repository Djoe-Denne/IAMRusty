---
title: "ADR-0601 — 4+1 Deployments et namespaces aiforall-* (Proposed)"
category: decisions
tags: [architecture, platform, kubernetes, visibility/internal]
status: proposed
feature_status: unimplemented
sources:
  - docs/adr/0601-cluster-trust-namespaces-standalones.md
  - docs/platform-cloud-v1-implementation-contract.md
summary: >-
  Canon : docs/adr/0601. Unité cluster V1 = 4+1 Deployments (iam, hive,
  manifesto, telegraph + lazaret) + sentinel-sync B/C. Namespaces
  aiforall-* identiques. Pas ns-per-tenant. Ferme le Non décidé K8s
  de 0404 sans l'éditer. 0008 intacte.
created: 2026-09-20T16:20:00Z
updated: 2026-09-20T16:20:00Z
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
---

# ADR-0601 — topologie cluster et confiance

Canon : `docs/adr/0601-cluster-trust-namespaces-standalones.md`. Hub : [[projects/aiforall/decisions/index]]. Plateforme : [[projects/aiforall/decisions/0600-cloud-portable]]. Dual runtime (photo) : [[projects/aiforall/decisions/0400-services-runtime]]. P4 : [[projects/manifesto/decisions/0008-apparatus-p4-k8s]].

## Statut : Proposed (2026-09-20)

Réalité **Unimplemented**. Ne pas présenter comme Accepted. **Complète** le Non décidé 0404 « 1 vs 4 Deployments » **sans** SuperSéder 0404. Split admit/signer : **reste dans 0008**.

## Paquet figé (cible)

1. **4+1** Deployments + `sentinel-sync` sur kind/remote. Monolithe = laptop/démo seulement.
2. Namespaces `aiforall-{platform,gateway,apparatus,plugins,data,secrets,gitops}` identiques sur chaque cluster. Lazaret **seul** dans `aiforall-gateway`. Plugins untrusted **uniquement** dans `aiforall-plugins`.
3. Tenancy = org Hive + FGA. **Ns-per-tenant interdit V1.**
4. 4 SA conceptuels (`build`, `admit-sign`, `controller`, `gateway`) ; colocation admit+sign OK jusqu'au split 0008. SPIRE pas V1.
5. 1 Postgres / 6 DB ; queue adaptateur SQS-like ; Redis KV Lazaret B/C (Compose A = KV postgres, écart 0500 non « corrigé »).

Related : [[projects/lazaret/lazaret]], [[projects/aiforall/concepts/https-platform-mesh]].
