---
title: >-
  Lancer l’e2e mesh AuthN
category: skills
tags: [mesh, docker, jwt, visibility/internal]
sources:
  - ops/scripts/mesh-authn-e2e.sh
  - ops/scripts/mesh-authn-e2e-cases.sh
  - docker-compose.yml
  - services/Hive/migration/Cargo.toml
  - services/Hive/Dockerfile
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/23daa0aa-3a66-4665-8db4-80c94e4e20a7/23daa0aa-3a66-4665-8db4-80c94e4e20a7.jsonl
summary: >-
  Preuve e2e = Kind (mesh-authn-kind-e2e.sh). Compose n’est plus la
  preuve. Pièges hivemigration, FORCE, build-artifacts.
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
created: 2026-10-01T16:45:00Z
updated: 2026-10-02T14:55:00Z
---

# Lancer l’e2e mesh AuthN

Contrat : [[projects/aiforall/decisions/0308-mesh-authn-jwt]]. Preuve du 30 sept. : [[journal/2026-09-30]]. Compiler dans Docker, jamais `cargo` Windows pour ce flux.

## Commande

Preuve retenue (2026-10-02) : `bash ops/scripts/mesh-authn-kind-e2e.sh` sur le contexte `kind-aiforall-local`. Exit 0, KIND MESH E2E OK. Postgres du mesh est in-cluster, sans port hôte 5432.

`bash ops/scripts/mesh-authn-e2e.sh` reste un harnais Compose (48 OK le 30 sept.). Ce n’est plus la preuve e2e.

## Ordre de build

Construire `build-artifacts` **avant** les images services. Sinon `COPY` part d’une `local/build-artifacts` périmée.

## Pièges découverts par l’e2e

- **Hive `hivemigration`** : collision avec le binaire Lazaret `migration` dans `target/release` partagé. Hive appliquait les migrations Lazaret → table `organizations` absente. Le Dockerfile copie `hivemigration` vers `/app/migration`.
- **Secret IAM↔Hive** : Hive refuse de booter si `iam_service.api_key` est vide (`StaticCredential` fail-closed). Compose : `IAM_INTERNAL_SERVICE_TOKEN` et `HIVE_IAM_SERVICE__API_KEY` = même secret de dev (ne pas recopier la valeur).
- **`create-databases` FORCE** : `DROP DATABASE … WITH (FORCE)` dans `docker-compose.yml`. Un pod Kind `oodhive-monolith` qui tient le Postgres Compose via le port hôte 5432 bloquait le DROP.
- **Healthchecks** : `/{préfixe}/health` (Hive, Telegraph, Manifesto, IAM). L’ancien `curl /health` marquait des conteneurs unhealthy.
- **Kind default-deny** : sans `ops/deploy/apps/overlays/kind-mesh/networkpolicy.yaml`, le DNS vers Postgres et le JWKS IAM expirent. L’e2e alpine a besoin d’un egress 80/443 pour `apk`.
- **Certificat Envoy** : le SAN est `envoy-mesh`, pas le FQDN du Service. Depuis le namespace gateway, appeler `https://envoy-mesh:10000`.

## Ce que l’e2e Kind prouve

Deny sans JWT, `sub` sur `/iam/api/me`, `user_id` Telegraph, `issuer` du membre Hive, contournement `mesh-client` en 401, signup / login / JWKS sans Bearer. Témoin clé `envoy-mesh` : [[projects/aiforall/concepts/mesh-gateway-principal-trust]].

## Related

- [[projects/aiforall/concepts/mesh-ext-authz-opt-in]]
- [[projects/aiforall/concepts/https-platform-mesh]]
