---
title: >-
  Lancer les IT en Docker (Windows hôte)
category: skills
tags: [skill, docker, testcontainers, rust, visibility/internal]
sources:
  - conversation opencode 2026-10-03 (clôture 0308)
  - AGENTS.md (Compilation locale)
  - IAMRusty/justfile
  - rustycog/rustycog-testing/src/common/database.rs
  - rustycog/rustycog-config/src/lib.rs
summary: >-
  Recette séquencée pour IT Rust dans Docker depuis Windows : image
  runner, préfixes alignés, réseau fixture, noms fixes, fuite Ryuk,
  un cargo à la fois. S2S IAM prouvé 23/23 (2026-10-03).
provenance:
  extracted: 0.80
  inferred: 0.16
  ambiguous: 0.04
created: 2026-10-03T15:10:00Z
updated: 2026-10-03T15:10:00Z
---

# Lancer les IT en Docker (Windows hôte)

Contexte : [[projects/aiforall/concepts/local-runtime-lifecycle]]. Cycle de vie du runtime : `AGENTS.md`. Découverte étape par étape lors de la clôture 0308 ([[projects/aiforall/references/opencode-close-0308-2026-10-03]]). Ces commandes ne concernent **pas** l'e2e Kind ([[projects/aiforall/skills/running-mesh-authn-e2e]]).

## Image runner

`rust:1.94-slim-bookworm` sans CLI Docker ; monter `/var/run/docker.sock`, `TESTCONTAINERS_HOST_OVERRIDE=host.docker.internal`, `TESTCONTAINERS_RYUK_DISABLED=true`, cache cargo+target dans des volumes Linux dédiés (jamais le `target\` hôte, jamais le même volume qu'un build d'image release). Un seul `cargo` à la fois.

## Alignement des hosts (piège le plus coûteux)

Le fixtcharge `rustycog-testing` lit `config/test.toml` sous `DATABASE_`, l'app sous son propre préfixe (`IAM_`, `HIVE_`). Le `PORT_CACHE` de `rustycog-config` est indexé `(host, db, user)` : deux préfixes avec des hosts différents → le port random publié par le fixture n'est jamais trouvé par l'app → `connect_timeout` > fenêtre de polling.

- **IAM** (réseau docker par défaut) : `RUN_ENV=test` + `DATABASE_DATABASE__HOST=host.docker.internal` **et** `IAM_DATABASE__HOST=host.docker.internal` dans le même runner.
- **Hive** : `--network host` + `HIVE_DATABASE__HOST=127.0.0.1` + `DATABASE_DATABASE__HOST=127.0.0.1` + `OPENFGA_OPENFGA__HOST=127.0.0.1` (publish_env OpenFGA force `127.0.0.1` codé en dur). Voir IAMRusty/justfile (`RUN_ENV=test` déjà posé côté hôte).

## Noms fixes et fuite de fixtures

Les fixtures utilisent des noms fixes (`test-db`, `openfga_test-fga`) : un conteneur arrêté qui traîne → erreur 409. Avec Ryuk désactivé et le cleanup `docker rm -f` inaccessible depuis l'image runner sans CLI Docker, **chaque run fuit ses conteneurs de fixture**.

- Ne pas réactiver le cleanup CLI du fixture : il contient `rm -f` (interdit par `.cursor/rules/infra-safety.mdc`).
- Entre deux suites : récupérer l'ID `docker ps -a --filter name=test-db`, le **renommer** (`docker rename`, réversible) vers `adr0308-it-retained-<id>` plutôt que le supprimer ; ou arrêter gracieusement puis suppression sur permission explicite du user.
- Vérifier l'inventaire (`docker ps -a`) à la fin, ne pas présumer que le harness a nettoyé.

## Séquence recommandée

```powershell
# (extrait, à adapter — voir .summary de la session pour les formes exactes)
docker run --rm `
  -v <repo>:/usr/src -v /var/run/docker.sock:/var/run/docker.sock `
  -v aiforall-it-cargo:/usr/local/cargo/registry -v aiforall-it-target:/tmp/target `
  -e CARGO_TARGET_DIR=/tmp/target `
  -e TESTCONTAINERS_HOST_OVERRIDE=host.docker.internal `
  -e TESTCONTAINERS_RYUK_DISABLED=true `
  -e RUN_ENV=test `
  -e DATABASE_DATABASE__HOST=host.docker.internal `
  -e IAM_DATABASE__HOST=host.docker.internal `
  rust:1.94-slim-bookworm cargo test -p iam-service --test internal_provider_token
```

Preuves 2026-10-03 : `internal_provider_token` 13/13, `revoke_provider_token` 10/10, suites Hive 19/19 (migration incluse), ext-authz 12/12.

## Related

- [[projects/aiforall/concepts/local-runtime-lifecycle]] — qui possède quoi et quand tout arrêter.
- [[projects/aiforall/skills/running-mesh-authn-e2e]] — l'e2e Kind (autre canal).
- [[projects/rustycog/references/openfga-real-testcontainer-fixture]] — fixture OpenFGA réelle.
