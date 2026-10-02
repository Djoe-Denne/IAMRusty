---
title: Mesh ext_authz opt-in (ADR-0308)
category: concepts
tags: [architecture, iam, jwt, security, visibility/internal]
sources:
  - docs/adr/0308-mesh-authn-jwt.md
  - docs/adr/0307-workload-identity-port.md
  - deploy/mesh/envoy.yaml
  - deploy/mesh/compose.yaml
  - ext-authz/src/check.rs
  - scripts/mesh-authn-e2e.sh
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/23daa0aa-3a66-4665-8db4-80c94e4e20a7/23daa0aa-3a66-4665-8db4-80c94e4e20a7.jsonl
summary: >-
  AuthN mesh opt-in : Envoy + ext-authz. Partial. Compose §7 + mTLS hops
  prouvés le 30 sept. Kind encore un cluster backend, pas Implemented.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-09-29T14:45:00Z
updated: 2026-10-01T16:45:00Z
---

# Mesh ext_authz opt-in (ADR-0308)

Canon : `docs/adr/0308-mesh-authn-jwt.md`. Décision : [[projects/aiforall/decisions/0308-mesh-authn-jwt]]. Confiance service : [[projects/aiforall/concepts/mesh-gateway-principal-trust]]. HTTPS T14b (autre sujet) : [[projects/aiforall/concepts/https-platform-mesh]].

`Accepted` (2026-09-27) ratifie **HTTP ext_authz** branché sur Envoy. Pas le filtre JWT Envoy. Pas de sidecar. `Réalité : Partial`. Pas Implemented.

## Contrat

- Le gateway valide le JWT puis émet `(iss, sub)`.
- Tout `X-Principal-*` client est supprimé et recréé. Jamais de confiance client.
- En overlay Compose, les hops sont mTLS `platform-mesh` et le service applique §7 (SAN `envoy-mesh`).
- Staleness max de révocation : pas de TTL chiffré (0304 §17).

## Déjà dans le dépôt (opt-in)

- Crate `ext-authz/` : `POST /` et `POST /check`. Strip puis recreate `x-principal-iss` / `x-principal-sub`. `EXT_AUTHZ_AUDIENCE` obligatoire. Cache JWKS + poll.
- Compose : `ext-authz` et `envoy-mesh` ont `profiles: ["mesh"]`.
- `deploy/mesh/envoy.yaml` : routes `/iam/` `/hive/` `/telegraph/` `/manifesto/` vers clusters homonymes ; Lua strip ; `allowed_upstream_headers` iss/sub seulement ; `failure_mode_allow: false`.
- Overlay `deploy/mesh/compose.yaml` : client cert requis + `TRUSTED_GATEWAY_SAN=envoy-mesh`.
- `kind-mesh/` : apply manuel, un seul cluster `backend`. HelmRelease Flux Envoy Kind commenté.

## Preuve 29 sept. (partielle, dépassée)

E2e Envoy Windows / HTTP clair : deny sans JWT, recreate iss/sub. `x-principal-foo` encore transmis. Pas de compose profile, pas de mTLS. Conservé comme photo. [[journal/2026-09-29]]

## Preuve 30 sept. (Compose isoprod)

`bash scripts/mesh-authn-e2e.sh` : 48 OK, 0 FAIL, pile démontée. Allow + deny + contournement + témoin clé passerelle. Détail : [[projects/aiforall/skills/running-mesh-authn-e2e]] [[journal/2026-09-30]].

## Encore ouvert

- Mesh par défaut ; Kind §7 ; Flux Envoy ; routes publiques IAM ; S2S `.authenticated()` direct ; TTL staleness.

## Related

- [[projects/aiforall/concepts/jwt-issuer-vs-consumer]]
- [[projects/aiforall/references/cursor-chat-mesh-authn-2026-09-30]]
