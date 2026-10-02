---
title: >-
  ADR-0308 — Mesh AuthN JWT, mode passerelle
category: decisions
tags: [architecture, iam, jwt, mesh, visibility/internal]
status: accepted
feature_status: partial
sources:
  - docs/adr/0308-mesh-authn-jwt.md
  - docs/adr/0304-jwt-acces-plateforme-rs256-jwks.md
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/23daa0aa-3a66-4665-8db4-80c94e4e20a7/23daa0aa-3a66-4665-8db4-80c94e4e20a7.jsonl
summary: >-
  0308 Accepted / Partial. Ext_authz valide le JWT ; en mesh le service
  fait confiance au principal Envoy (SAN passerelle), sans revérifier.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-10-01T16:45:00Z
updated: 2026-10-01T16:45:00Z
---

# ADR-0308 — Mesh AuthN JWT, mode passerelle

Canon : `docs/adr/0308-mesh-authn-jwt.md`. Hub crypto : [[projects/aiforall/decisions/0304-access-jwt-trust]]. Runtime : [[projects/aiforall/concepts/mesh-ext-authz-opt-in]]. Confiance service : [[projects/aiforall/concepts/mesh-gateway-principal-trust]].

`Accepted` ratifie ext_authz HTTP (pas filtre JWT Envoy, pas sidecar). `Réalité : Partial`. **Pas Implemented.**

## Décision §7 (2026-09-30, `trust_envoy`)

En overlay mesh, un service ne revérifie pas le JWT. Il lit `x-principal-iss` / `x-principal-sub` seulement sur une connexion mTLS dont le SAN DNS client est la passerelle (`auth.mesh.trusted_gateway_san`). Bearer ignoré. Autre pair mTLS, TLS sans cert ou HTTP clair → 401. SAN vide = JWT in-process (défaut).

Conséquence : posséder la clé `envoy-mesh` permet d’injecter n’importe quel principal.

Kind plus tard (`compose_then_kind`). L’overlay `kind-mesh` a encore un seul cluster `backend`.

## Gaps qui restent

Mesh non défaut ; Kind sans services réels ni §7 ; Flux Envoy Kind commenté ; routes publiques IAM (signup, login, JWKS) refusées sans JWT ; S2S direct vers une route `.authenticated()` (ex. Hive → IAM `/internal/*` :8080) = 401 ; staleness révocation Non décidé (0304 §17).

## Related

- [[projects/aiforall/concepts/jwt-issuer-vs-consumer]]
- [[projects/aiforall/skills/running-mesh-authn-e2e]]
- [[journal/2026-09-30]]
