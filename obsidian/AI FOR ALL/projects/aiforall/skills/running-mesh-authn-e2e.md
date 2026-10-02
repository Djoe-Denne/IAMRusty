---
title: >-
  Lancer l’e2e mesh AuthN
category: skills
tags: [mesh, docker, jwt, visibility/internal]
sources:
  - scripts/mesh-authn-e2e.sh
  - scripts/mesh-authn-e2e-cases.sh
  - docker-compose.yml
  - Hive/migration/Cargo.toml
  - Hive/Dockerfile
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/23daa0aa-3a66-4665-8db4-80c94e4e20a7/23daa0aa-3a66-4665-8db4-80c94e4e20a7.jsonl
summary: >-
  E2e Compose : build-artifacts d’abord, FORCE sur DROP DATABASE,
  binaire hivemigration, secret IAM/Hive partagé. Pas cargo Windows.
provenance:
  extracted: 0.92
  inferred: 0.06
  ambiguous: 0.02
created: 2026-10-01T16:45:00Z
updated: 2026-10-01T16:45:00Z
---

# Lancer l’e2e mesh AuthN

Contrat : [[projects/aiforall/decisions/0308-mesh-authn-jwt]]. Preuve du 30 sept. : [[journal/2026-09-30]]. Compiler dans Docker, jamais `cargo` Windows pour ce flux.

## Commande

`bash scripts/mesh-authn-e2e.sh` (cas dans `scripts/mesh-authn-e2e-cases.sh`). Preuve retenue : exit 0, 48 OK, 0 FAIL, pile démontée.

## Ordre de build

Construire `build-artifacts` **avant** les images services. Sinon `COPY` part d’une `local/build-artifacts` périmée.

## Pièges découverts par l’e2e

- **Hive `hivemigration`** : collision avec le binaire Lazaret `migration` dans `target/release` partagé. Hive appliquait les migrations Lazaret → table `organizations` absente. Le Dockerfile copie `hivemigration` vers `/app/migration`.
- **Secret IAM↔Hive** : Hive refuse de booter si `iam_service.api_key` est vide (`StaticCredential` fail-closed). Compose : `IAM_INTERNAL_SERVICE_TOKEN` et `HIVE_IAM_SERVICE__API_KEY` = même secret de dev (ne pas recopier la valeur).
- **`create-databases` FORCE** : `DROP DATABASE … WITH (FORCE)` dans `docker-compose.yml`. Un pod Kind `oodhive-monolith` qui tient le Postgres Compose via le port hôte 5432 bloquait le DROP.
- **Healthchecks** Telegraph / Manifesto restent unhealthy (`curl /health` sans préfixe de service). Non bloquant, non corrigé.

## Ce que l’e2e prouve

Allow + deny + contournement :8443 / :8080. Telegraph 200 = authn, pas identité (user neuf sans notifications). Témoin clé `envoy-mesh` : [[projects/aiforall/concepts/mesh-gateway-principal-trust]].

## Related

- [[projects/aiforall/concepts/mesh-ext-authz-opt-in]]
- [[projects/aiforall/concepts/https-platform-mesh]]
