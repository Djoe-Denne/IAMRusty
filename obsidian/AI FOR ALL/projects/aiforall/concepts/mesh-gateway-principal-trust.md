---
title: >-
  Confiance principal passerelle (mesh rustycog)
category: concepts
tags: [architecture, jwt, rustycog, mesh, visibility/internal]
sources:
  - docs/adr/0308-mesh-authn-jwt.md
  - rustycog/rustycog-http/src/mesh_principal.rs
  - rustycog/rustycog-http/src/middleware_auth.rs
  - rustycog/rustycog-config/src/lib.rs
  - deploy/mesh/compose.yaml
  - IAMRusty/setup/src/app.rs
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/23daa0aa-3a66-4665-8db4-80c94e4e20a7/23daa0aa-3a66-4665-8db4-80c94e4e20a7.jsonl
summary: >-
  Mode mesh rustycog : trusted_gateway_san non vide = principal Envoy,
  SAN DNS obligatoire, pas de JWT in-process. Vide = défaut 0304.
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
created: 2026-10-01T16:45:00Z
updated: 2026-10-01T16:45:00Z
---

# Confiance principal passerelle (mesh rustycog)

Décision : [[projects/aiforall/decisions/0308-mesh-authn-jwt]]. Opt-in Envoy : [[projects/aiforall/concepts/mesh-ext-authz-opt-in]]. JWT in-process : [[projects/aiforall/concepts/jwt-issuer-vs-consumer]].

## Contrat rustycog

`AuthConfig.mesh.trusted_gateway_san` (vide = JWT in-process). Pin sibling poussé : `2290d45` « feat(http): trust gateway principal in mesh mode ». Fichiers : `mesh_principal.rs`, `middleware_auth.rs`, tests `rustycog-http/tests/mesh_gateway_auth.rs`. Le gitlink AIForAll pointe ce SHA ; non commité dans AIForAll.

IAM `setup` copie `config.auth.mesh` dans `http_verifier_auth` pour que `/api/me` suive le même mode.

## Overlay Compose

`deploy/mesh/compose.yaml` : `*_SERVER__TLS_REQUIRE_CLIENT_CERT=true` et `*_AUTH__MESH__TRUSTED_GATEWAY_SAN=envoy-mesh` pour iam, hive, telegraph, manifesto.

## Témoin de confiance

Un client qui présente le certificat `envoy-mesh` peut spoof `x-principal-*` vers `iam-service:8443` et obtenir 200. La clé passerelle est le secret d’usurpation. Un cert `mesh-client` de la même CA est refusé.

## Related

- [[skills/using-rustycog-http]]
- [[projects/aiforall/concepts/rustycog-git-submodule]]
- [[projects/aiforall/skills/running-mesh-authn-e2e]]
