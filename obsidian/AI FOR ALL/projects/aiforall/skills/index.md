---
title: AIForAll Skills Index
category: navigation
tags: [index, skills, platform]
summary: >-
  Procédures AIForAll : modes runtime, validation explicite et préparation OAuthConnect HTTPS/OpenBao ; legacy et preuves live restent séparés.
sources:
  - ops/deploy/README.md
  - docs/missions/20261108-ops-oauthconnect.md
provenance:
  extracted: 0.94
  inferred: 0.05
  ambiguous: 0.01
updated: 2026-11-08
---

# AIForAll Skills

- [[projects/aiforall/skills/deploying-oauthconnect-local]] — profil GitHub+simulated HTTPS/OpenBao, sources privées et reprise d'epoch ; pas E2E implicite.

- [[projects/aiforall/skills/running-mesh-authn-e2e]] — e2e Compose 0308 ; `hivemigration`, FORCE, `build-artifacts` d’abord.
- [[projects/aiforall/skills/running-it-tests-docker]] — IT Rust en Docker depuis Windows : préfixes alignés, réseau fixture, fuite Ryuk, noms fixes. (**obsolète 2026-10-07** : dev/IT natif Windows.)
- [[projects/aiforall/skills/preserving-role-permission-org-scope]] — migration Hive : unicité (org, perm, resource) sans perte.
- [[projects/aiforall/skills/running-aiforall-runtime-modes]]
- [[projects/aiforall/skills/fixing-sonar-clippy-in-services]]
- [[projects/aiforall/skills/running-parallel-sonar-lanes]]
- [[projects/aiforall/skills/running-apparatus-p0-tests]]
- [[projects/aiforall/skills/running-apparatus-p1-tests]]
- [[projects/aiforall/skills/running-apparatus-p2-tests]] — T1–T7, `--test-threads=1` ; ADR-0006 Implemented.
- [[projects/aiforall/skills/running-apparatus-p4-tests]] — operator T6+T10+T11, Docker+Kind, `--test-threads=1`.
- [[projects/lazaret/skills/running-apparatus-p3-tests]] — T3–T7 Lazaret ; ADR-0007 Implemented.
- Repo handbook (not a vault page): `docs/README.md` — JWT, nouveau service, parcours métier. Agent skill `.agents/skills/aiforall-new-service/SKILL.md`.
