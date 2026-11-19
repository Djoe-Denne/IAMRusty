---
title: "ADR 0500–0502 — config, qualité, rustycog feature-gated"
category: decisions
tags: [platform, rustycog, ci, visibility/internal]
summary: "Config rustycog-config + RUN_ENV. Compose sans NATS/Redis/Kafka. fmt/Clippy/Sonar (coverage 80 % non tenu). rustycog = framework submodule, pas un service."
created: 2026-09-12T10:20:00Z
updated: 2026-11-19
sources:
  - docs/adr/0500-config-typee-et-compose-local.md
  - docs/adr/0501-qualite-fmt-clippy-sonar.md
  - docs/adr/0502-rustycog-framework-feature-gated.md
  - docs/adr/0507-ci-service-local-dag-shared-compiler-cache.md
  - docs/contracts/ci-iam-compile-once-adoption.md
  - docker-compose.yml
  - .github/workflows/ci.yml
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
---

# ADR 0500–0502 — plateforme et qualité

Canon : `docs/adr/0500`–`0502`. Hub : [[projects/aiforall/decisions/index]].

## 0500 — Config typée + Compose réel (Implemented)

Loader = `rustycog-config` (`build_config_with_env_prefix`). Sélecteur = **`RUN_ENV` seulement** (`test` / `production` / défaut `development`). **`RUST_ENVIRONMENT` n’est pas lu.**

Compose local : Postgres, OpenFGA (+ migrate), LocalStack SQS, les quatre `*-service`, jobs DB. **Pas** de NATS, Redis, Kafka, `sentinel-sync`, ni monolithe dans ce fichier.

## 0501 — fmt / Clippy / Sonar (Partial)

- `cargo fmt --all -- --check` : job CI + hook pre-commit.
- Clippy dans le job **sonar** en `-W`, **pas** `-D warnings` autonome.
- Projet Sonar = `Djoe-Denne_IAMRusty` (monorepo AIForAll).
- Gate `new_coverage` 80 % **ratifié mais non tenu** : ~73,3 % au 12 sept. (64,1 % au 9 sept.). Hotspots sécurité : 0.

Ne pas présenter le dépôt comme « gate vert ». Skill : [[projects/aiforall/skills/fixing-sonar-clippy-in-services]].

## 0502 — rustycog = framework, pas un service (Implemented)

Submodule git pin, crates feature-gated (`command`, `db`, `events`, `http`, `outbox`, `permission`, `testing`, `logger`, `config`, `core`, `server`). Un métier **consomme** rustycog, il ne le fork pas dans le service. Voir [[projects/aiforall/concepts/rustycog-git-submodule]] et [[projects/rustycog/rustycog]].

Feature `kafka` hors `full` Windows — opt-in, pas dans Compose (0500).

## 0507 — Service-local CI and IAM compile-once (Proposed / Partial)

Canon: `docs/adr/0507-ci-service-local-dag-shared-compiler-cache.md`.
Implementation contract: `docs/contracts/ci-iam-compile-once-adoption.md`.
The qualified IAM pilot passed five E2E scenarios and cleanup in hosted run
44493607389; the earlier five test replays passed separately. IAM-only main-CI
adoption is authorized, but a complete adopted canonical run remains required.
Other services and native/release/saved-image admission remain unchanged.
This is a pointer, not acceptance or a performance claim.

## Related

- [[concepts/structured-service-configuration]]
- [[projects/aiforall/skills/running-parallel-sonar-lanes]]
