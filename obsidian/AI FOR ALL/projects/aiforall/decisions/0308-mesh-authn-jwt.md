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
  0308 Accepted / Partial. Amendement 2026-10-02 : e2e Kind, IT hors
  Kind, isoprod third parties in-kind. Pas Implemented.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-10-01T16:45:00Z
updated: 2026-10-02T14:55:00Z
---

# ADR-0308 — Mesh AuthN JWT, mode passerelle

Canon : `docs/adr/0308-mesh-authn-jwt.md`. Hub crypto : [[projects/aiforall/decisions/0304-access-jwt-trust]]. Runtime : [[projects/aiforall/concepts/mesh-ext-authz-opt-in]]. Confiance service : [[projects/aiforall/concepts/mesh-gateway-principal-trust]].

`Accepted` ratifie ext_authz HTTP (pas filtre JWT Envoy, pas sidecar). `Réalité : Partial`. **Pas Implemented.**

## Décision §7 (2026-09-30, `trust_envoy`)

En overlay mesh, un service ne revérifie pas le JWT. Il lit `x-principal-iss` / `x-principal-sub` seulement sur une connexion mTLS dont le SAN DNS client est la passerelle (`auth.mesh.trusted_gateway_san`). Bearer ignoré. Autre pair mTLS, TLS sans cert ou HTTP clair → 401. SAN vide = JWT in-process (défaut).

Conséquence : posséder la clé `envoy-mesh` permet d’injecter n’importe quel principal.

## Amendement 2026-10-02

Prime sur `compose_then_kind`. IT : third parties hors Kind (norme Compose ; code = testcontainers). E2E : Kind seul, preuve `ops/scripts/mesh-authn-kind-e2e.sh`. Isoprod hors IT : Postgres, Redis et le reste in-kind. Flux commenté ne mesure pas Implemented. Journal : [[journal/2026-10-02]].

`kind-mesh` a les quatre routes, les workloads, §7, les chemins publics sans Check, Postgres sans hostPort 5432. Signup / login / JWKS passent par Envoy.

## Gaps qui bloquent encore Implemented

S2S `/internal/{provider}/token|revoke` = 401 hors passerelle. Gitlink `2290d45` non commité. Staleness sans TTL (0304 §17) — ne pas chiffrer pour clôturer. Le signataire 0306 n’est pas le trou S2S.

Ne bloquent plus : e2e Compose, Flux commenté, Kind sans services, routes publiques hors Envoy, « mesh non défaut » (l’opt-in est la décision).

Prompt : [[projects/aiforall/references/mesh-authn-close-2026-10-02]].

## Related

- [[projects/aiforall/concepts/jwt-issuer-vs-consumer]]
- [[projects/aiforall/skills/running-mesh-authn-e2e]]
- [[journal/2026-09-30]]
