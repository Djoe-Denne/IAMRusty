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
  0308 Accepted / Partial. Source caches trust monotone60 et vide autoritatif écrite,
  SDK22 purs PASS selon parent ; IAM/mesh courant non prouvé. IT puis E2E full.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-10-01T16:45:00Z
updated: 2026-10-04
---

# ADR-0308 — Mesh AuthN JWT, mode passerelle

Canon : `docs/adr/0308-mesh-authn-jwt.md`. Hub crypto : [[projects/aiforall/decisions/0304-access-jwt-trust]]. Runtime : [[projects/aiforall/concepts/mesh-ext-authz-opt-in]]. Confiance service : [[projects/aiforall/concepts/mesh-gateway-principal-trust]].

`Accepted` ratifie ext_authz HTTP (pas filtre JWT Envoy, pas sidecar). `Réalité : Partial`. **Pas Implemented.**

## Décision §7 (2026-09-30, `trust_envoy`)

En overlay mesh, un service ne revérifie pas le JWT. Il lit `x-principal-iss` / `x-principal-sub` seulement sur une connexion mTLS dont le SAN DNS client est la passerelle (`auth.mesh.trusted_gateway_san`). Bearer ignoré. Autre pair mTLS, TLS sans cert ou HTTP clair → 401. SAN vide = JWT in-process (défaut).

Conséquence : posséder la clé `envoy-mesh` permet d’injecter n’importe quel principal.

## Amendement 2026-10-02

Prime sur `compose_then_kind`. IT : testcontainers du harness, hors Kind, pas migration Compose. E2E : Kind après IT finale ; script mesh historique `ops/scripts/mesh-authn-kind-e2e.sh`. Isoprod hors IT : dépendances in-Kind. Nouveau full dédié autorisé : [[projects/aiforall/decisions/0606-local-full-kind-isole]]. Flux commenté ne mesure pas Implemented. Journal historique : [[journal/2026-10-02]].

Sources `ops/deploy/apps/overlays/kind-mesh/` et profil full séparé présentes. Les résultats signup/login/JWKS/S2S du 2026-10-02 sont historiques, pas preuve du root courant modifié. Routes publiques exactes/transactionnelles, trust et trafic Envoy doivent être prouvés après intégration.

## Gates de preuve qui bloquent encore Implemented

SDK sélectionné `ca2e35fcd56279e9e52625d0df9381f240f3390d`, publié selon parent ; `2290d45`/`ba69c9e` et LKG sans TTL sont historiques. Sources monotone60/vide/métadonnées/trust écrites, poll60/local2 conservé ; SDK22 purs PASS ≠ IAM/publisher/mesh. Compilation root, units IAM, IT SQL/crypto/concurrence puis E2E Envoy/S2S/spoof/outage/vide encore requis. Le signataire 0306 n’est pas ce trou.

Ne bloquent plus : e2e Compose, Flux commenté, Kind sans services, routes publiques hors Envoy, « mesh non défaut » (l’opt-in est la décision).

Prompt : [[projects/aiforall/references/mesh-authn-close-2026-10-02]].

## Related

- [[projects/aiforall/concepts/jwt-issuer-vs-consumer]]
- [[projects/aiforall/skills/running-mesh-authn-e2e]]
- [[journal/2026-09-30]]
