---
title: >-
  Chat Cursor mesh AuthN (30 sept. 2026)
category: references
tags: [reference, cursor, mesh, jwt, visibility/internal]
sources:
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/23daa0aa-3a66-4665-8db4-80c94e4e20a7/23daa0aa-3a66-4665-8db4-80c94e4e20a7.jsonl
summary: >-
  Distillat d’un seul chat : trust_envoy, rustycog 2290d45, e2e 48 OK,
  pièges Hive/FORCE/build-artifacts. Pas un dump du JSONL.
provenance:
  extracted: 0.93
  inferred: 0.06
  ambiguous: 0.01
created: 2026-10-01T16:45:00Z
updated: 2026-10-01T16:45:00Z
---

# Chat Cursor mesh AuthN (30 sept. 2026)

Source unique : transcript `23daa0aa-3a66-4665-8db4-80c94e4e20a7` (375 lignes JSONL). Pas tout l’historique Cursor. Journal : [[journal/2026-09-30]].

## Décisions

- `trust_envoy` + changement rustycog / ADR-0308 §7. [[projects/aiforall/decisions/0308-mesh-authn-jwt]]
- `compose_then_kind`. Kind `envoy-mesh.yaml` : un cluster `backend`.

## Contrat observable

Envoy : `/iam/` `/hive/` `/telegraph/` `/manifesto/` → clusters homonymes ; mTLS ; ext_authz HTTP ; Lua strip `x-principal-*` ; headers amont iss/sub seulement ; `failure_mode_allow: false` ; pas de filtre JWT Envoy.

Services : `TLS_REQUIRE_CLIENT_CERT=true` + `TRUSTED_GATEWAY_SAN=envoy-mesh`.

E2E : allow (IAM id=sub, Hive owner=sub, Manifesto 201 owner=sub, Telegraph 200) ; deny sans JWT ; contournement 401 ; témoin clé `envoy-mesh` = 200.

## Fichiers

`deploy/mesh/envoy.yaml`, `deploy/mesh/compose.yaml`, `scripts/mesh-authn-e2e.sh`, `scripts/mesh-authn-e2e-cases.sh`, rustycog `mesh_principal.rs` / `mesh_gateway_auth.rs`, `IAMRusty/setup/src/app.rs`, `Manifesto/config/development.toml`, `Hive/migration` `hivemigration`, `docker-compose.yml` `DROP … WITH (FORCE)`.

## Gaps volontairement ouverts

Mesh non défaut ; Kind sans §7 ; Flux commenté ; routes publiques IAM non exposées ; S2S `.authenticated()` direct 401 ; staleness 0304 §17 Non décidé ; healthchecks Telegraph/Manifesto unhealthy. Rien commité dans AIForAll.

## Related

- [[projects/aiforall/references/cursor-history-2026-09]]
- [[projects/aiforall/concepts/mesh-gateway-principal-trust]]
- [[projects/aiforall/skills/running-mesh-authn-e2e]]
