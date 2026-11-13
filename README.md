# AIForAll

[![Quality Gate](https://sonarcloud.io/api/project_badges/measure?project=Djoe-Denne_IAMRusty&metric=alert_status)](https://sonarcloud.io/dashboard?id=Djoe-Denne_IAMRusty)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=Djoe-Denne_IAMRusty&metric=coverage)](https://sonarcloud.io/component_measures?id=Djoe-Denne_IAMRusty&metric=coverage)
[![Reliability](https://sonarcloud.io/api/project_badges/measure?project=Djoe-Denne_IAMRusty&metric=reliability_rating)](https://sonarcloud.io/dashboard?id=Djoe-Denne_IAMRusty)
[![Security](https://sonarcloud.io/api/project_badges/measure?project=Djoe-Denne_IAMRusty&metric=security_rating)](https://sonarcloud.io/dashboard?id=Djoe-Denne_IAMRusty)
[![Maintainability](https://sonarcloud.io/api/project_badges/measure?project=Djoe-Denne_IAMRusty&metric=sqale_rating)](https://sonarcloud.io/dashboard?id=Djoe-Denne_IAMRusty)
[![Duplicated Lines](https://sonarcloud.io/api/project_badges/measure?project=Djoe-Denne_IAMRusty&metric=duplicated_lines_density)](https://sonarcloud.io/dashboard?id=Djoe-Denne_IAMRusty)
[![CI](https://github.com/Djoe-Denne/IAMRusty/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/Djoe-Denne/IAMRusty/actions/workflows/ci.yml)
[![E2E](https://img.shields.io/endpoint?url=https%3A%2F%2Fdjoe-denne.github.io%2FIAMRusty%2Fbadge.json)](https://djoe-denne.github.io/IAMRusty/)

[Rapport E2E / Allure](https://djoe-denne.github.io/IAMRusty/) ·
[Métriques E2E JSON](https://djoe-denne.github.io/IAMRusty/status.json) ·
[SonarQube Cloud](https://sonarcloud.io/dashboard?id=Djoe-Denne_IAMRusty)

Les E2E (34 contrôles health/OAuth navigateur/TLS-SAN et contrat parent) tournent
automatiquement après les builds sur `push/master`, en parallèle des couvertures
UT/IT. Le badge E2E correspond au **dernier rapport publié**, dont le SHA est
affiché sur le site ; il ne représente pas la couverture de code Sonar.

Plateforme Rust de bounded contexts (identité, organisations, projets, notifications) sur le SDK [rustycog](https://github.com/Djoe-Denne/rustycog) (git submodule).

**Handbook (implémentation + parcours métier) :** [`docs/README.md`](docs/README.md)

## Services

| Service | Préfixe | Port compose | Rôle |
|---|---|---|---|
| [IAMRusty](services/IAMRusty/README.md) | `/iam` | 8080 | Identité, OAuth, JWT |
| [Telegraph](services/Telegraph/README.md) | `/telegraph` | 8081 | Emails et notifications in-app |
| [Hive](services/Hive/README.md) | `/hive` | 8082 | Organisations, membres, invitations |
| [Manifesto](services/Manifesto/README.md) | `/manifesto` | 8083 | Projets, composants, membership |
| [sentinel-sync](workers/sentinel-sync/README.md) | — | (hors compose) | Events → tuples OpenFGA |
| [oodhive-monolith](runtime/monolith/README.md) | `/iam`…`/manifesto` | (hors compose) | Un listener, quatre routeurs |

Infra compose : PostgreSQL **5432**, LocalStack SQS **4566**, OpenFGA **8090** (HTTP) / **8091** (gRPC) / **3000** (playground).

Crates d’événements : `iam-events`, `hive-events`, `manifesto-events`, `telegraph-events`. Readiness : crate `readiness` (`/ready`).

## Démarrage

```bash
git clone --recurse-submodules <url>
cd AIForAll
docker compose up -d
```

Logs : `docker compose logs -f`. Stop : `docker compose down`.

Outils (profil `tools`) :

```bash
docker compose --profile tools run --rm truncate-db
docker compose --profile tools run --rm verify-emails
```

JWT local : HMAC partagé, `iss=iamrusty`, `aud=aiforall`. Détail : [`docs/platform/authn-jwt.md`](docs/platform/authn-jwt.md).

## Flux

- IAM publie `user_signed_up` / `password_reset_requested` / `user_email_verified` vers **`telegraph-events`**.
- Hive, Manifesto, Telegraph publient vers **`sentinel-sync-events`** (AuthZ).
- `sentinel-sync` n’est **pas** démarré par le compose — le lancer à part après bootstrap du store OpenFGA.

## Monolithe

```bash
cargo run -p oodhive-monolith
```

Mêmes préfixes qu’en standalone. Voir [`docs/platform/runtime.md`](docs/platform/runtime.md) et [`runtime/monolith/README.md`](runtime/monolith/README.md).

## Tests et format

La version de Rust et les composants `rustfmt` / `clippy` sont épinglés dans
[`rust-toolchain.toml`](rust-toolchain.toml). Rustup sélectionne automatiquement
cette version dans le dépôt ; la CI lit ce même fichier via l'action locale
`.github/actions/setup-rust`. Pour changer de version, modifier uniquement ce
fichier, reformater puis vérifier avant de pousser.

Éviter les overrides `cargo +stable`, `RUSTUP_TOOLCHAIN` ou `rustup override` qui
prennent priorité sur ce fichier. `rustup show active-toolchain` permet de
vérifier la version effectivement sélectionnée, sans changer le défaut global.

Les artefacts de développement Windows restent sur `E:\cargo-target\AIForAll`.
La CI définit `CARGO_TARGET_DIR` dans son workspace Linux pour ne pas réutiliser
ce chemin Windows.

```bash
rustup show active-toolchain
cargo test -p hive-service
cargo fmt --all
cargo fmt --all -- --check
```

Hook fmt : [`docs/CARGO_FMT_PRE_COMMIT.md`](docs/CARGO_FMT_PRE_COMMIT.md).

Nouveau service : [`docs/guides/nouveau-service.md`](docs/guides/nouveau-service.md). Submodule rustycog : [`.agents/skills/rustycog-submodule/SKILL.md`](.agents/skills/rustycog-submodule/SKILL.md).

## Sources du harness agents

La boucle QA des profils OpenCode (plan, écriture, revue) s’appuie sur ces dépôts. Le pin produit reste celui de ce repository.

- [Petr Kindlmann](https://github.com/petrkindlmann), [qa-skills](https://github.com/petrkindlmann/qa-skills) — risque impact × probabilité, plan et stratégie de test, génération et revue de tests.
- [Nao](https://github.com/naodeng), [awesome-qa-skills](https://github.com/naodeng/awesome-qa-skills) — analyse de gaps (`test-gap-analysis`).
- [Dragan Spiridonov](https://github.com/proffesor-for-testing), [agentic-qe](https://github.com/proffesor-for-testing/agentic-qe) — flotte QE (plan, gaps, investigation).
- [AgenticTesting](https://github.com/AgenticTesting), [OpenTestingAI](https://github.com/AgenticTesting/OpenTestingAI) — séparation faits, inférences et décisions. Le modèle ISO/TMMi de ce dépôt n’est pas repris.
- [Microsoft](https://github.com/microsoft), [Playwright](https://github.com/microsoft/playwright) — boucle planner, generator, healer ([Test Agents](https://playwright.dev/docs/test-agents)).
