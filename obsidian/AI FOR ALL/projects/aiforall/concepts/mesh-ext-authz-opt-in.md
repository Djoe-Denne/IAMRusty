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
  AuthN mesh opt-in, Partial. E2e = Kind (OK 2026-10-02). IT hors Kind.
  Flux commenté ne bloque plus Implemented.
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
created: 2026-09-29T14:45:00Z
updated: 2026-10-02T14:55:00Z
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
- `kind-mesh/` : routes `/iam/` `/hive/` `/telegraph/` `/manifesto/`, workloads réels, `TRUSTED_GATEWAY_SAN=envoy-mesh`, Postgres in-cluster sans hostPort 5432. Pas dans `overlays/kind`. HelmRelease Flux commenté, hors critère d’Implemented.

## Preuve 29 sept. (partielle, dépassée)

E2e Envoy Windows / HTTP clair : deny sans JWT, recreate iss/sub. `x-principal-foo` encore transmis. Pas de compose profile, pas de mTLS. Conservé comme photo. [[journal/2026-09-29]]

## Preuve 30 sept. (Compose, plus la preuve e2e)

`bash scripts/mesh-authn-e2e.sh` : 48 OK, 0 FAIL, pile démontée. Photo historique. Depuis l’amendement 2026-10-02 ce script n’est plus la preuve e2e. [[journal/2026-09-30]]

## Preuve 2 oct. (Kind)

`bash scripts/mesh-authn-kind-e2e.sh` : KIND MESH E2E OK. Deny sans JWT, `sub`, `user_id` Telegraph, `issuer` Hive, contournement `mesh-client` refusé, signup / login / JWKS via Envoy. [[journal/2026-10-02]] [[projects/aiforall/skills/running-mesh-authn-e2e]]

## Canaux (2026-10-02)

- IT : pas Kind. Third parties sur Compose. Le code les lance encore en testcontainers. ^[extracted]
- E2E : entièrement Kind.
- Local isoprod hors IT : Postgres, Redis et le reste via Kind. Redis n’est pas déployé dans `kind-mesh`. ^[extracted]
- Le mesh opt-in n’est pas un trou. Flux commenté non plus. ^[inferred]

## Encore ouvert pour Implemented

- S2S `POST /iam/internal/{provider}/token` et `revoke` (`.authenticated()`) = 401 en direct. Le signataire 0306, lui, utilise `x-iam-internal-token`.
- Gitlink rustycog `2290d45` non commité ; le working tree ajoute le 401 hors SAN `envoy-mesh`.
- Staleness : pas de TTL (0304 §17). Ne pas en inventer un pour clôturer.

Prompt de clôture : `docs/0308-mesh-close-implementation-prompt.md`. [[projects/aiforall/references/mesh-authn-close-2026-10-02]]

## Related

- [[projects/aiforall/concepts/jwt-issuer-vs-consumer]]
- [[projects/aiforall/references/cursor-chat-mesh-authn-2026-09-30]]
