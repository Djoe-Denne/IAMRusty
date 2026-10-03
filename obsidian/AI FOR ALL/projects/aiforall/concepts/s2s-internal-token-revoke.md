---
title: >-
  S2S internal token/revoke (hop IAM interne)
category: concepts
tags: [concept, iam, security, mesh, visibility/internal]
sources:
  - services/IAMRusty/http/src/handlers/auth.rs
  - services/IAMRusty/http/src/rate_limit.rs
  - ops/scripts/mesh-authn-kind-e2e-cases.sh
  - docs/adr/0308-mesh-authn-jwt.md
  - conversation opencode 2026-10-03 (clôture 0308)
summary: >-
  POST /iam/internal/{provider}/token et DELETE .../revoke : cert client
  SAN envoy-mesh + gate internal-header, sans JWT utilisateur ; tout
  autre pair mTLS → 401. Prouvé Kind 2026-10-03 (35 OK).
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
created: 2026-10-03T15:10:00Z
updated: 2026-10-03T15:10:00Z
---

# S2S internal token/revoke (hop IAM interne)

Décision verrouillée du 2026-10-02 (amendement ADR), **codée et prouvée au runtime le 2026-10-03** : [[projects/aiforall/decisions/0308-mesh-authn-jwt]]. Le work precedent le développement l'avait raté du fait d'une prise en main tardive des preuves runtime.

## Contrat

- `POST /iam/internal/{provider}/token` et `DELETE /iam/internal/{provider}/revoke` exigent **cumulativement** :
  1. une connexion mTLS dont le certificat client porte le SAN DNS `envoy-mesh` (`auth.mesh.trusted_gateway_san`, [[projects/aiforall/concepts/mesh-gateway-principal-trust]]) ;
  2. l'en-tête interne déjà utilisé (`IAM_INTERNAL_SERVICE_TOKEN`, gate `require_internal_service_token` dans `services/IAMRusty/http/src/rate_limit.rs`) ;
  3. **pas** de JWT utilisateur sur ce hop.
- Un `mesh-client` (autre cert de la même CA, avec ou sans Bearer) est refusé **401** sur les deux routes ; `Bearer` + gate + cert gateway sans `x-principal-*` est aussi refusé (hatte le principal n'est jamais inféré depuis un Bearer).
- Langage test : les 2 routes composent avec les cas e2e positifs (témoignage `/test` exact) et le citoyen`(token créé → lire count=1 ; revoke → 200 ; lire → 404 + count=0 ; révoke ré-éxcécuté → 404 idempotente)`. Le bundle évalue l'état : un delete accepté supprime réellement le token en base (count 1→0).

## Ce qui a été ajouté en couverture (2026-10-03)

- Gate interne absente **et** erronée → 403 ×4 (2 routes × 2 erreurs) — leak jq d'`access_token` vérifié zéro.
- 4 certs applicatives (`iam/hive/telegraph/manifesto-service`) → 401 ×8 sur les deux routes : la gate SAN ne fait pas de faveur à un autre service de la plateforme.
- Bearer-sans-principal sur cert gateway → 401 ×2.
- Intégrité après refus : `count=1` après tous les refus (y compris DELETE) — aucun effet de bord avant la gate.

## Pourquoi ce hop existe

Un service backend (ex. Hive org signer, or seq : [[projects/hive/hive]]) a besoin d'un access token machine pour appeler la plateforme au nom de l'utilisateur sans lui demander son JWT (ex. orchestration avec grant OpenFGA). Ce contrat est la **seule** porte S2S : pas d'autre route `/internal/` ouverte.

## Related

- [[projects/aiforall/skills/running-mesh-authn-e2e]] — wrapper e2e (unique preuve).
- [[projects/hive/hive]] — client org signer (x-iam-internal-token).
- [[projects/aiforall/references/opencode-close-0308-2026-10-03]] — preuve d'exécution.
