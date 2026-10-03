---
title: >-
  Clôture 0308 — conversation OpenCode 2026-10-03
category: references
tags: [reference, mesh, jwt, docker, kind, visibility/internal]
sources:
  - conversation opencode 2026-10-03 (clôture 0308)
  - docs/adr/0308-mesh-authn-jwt.md
  - scripts/mesh-authn-kind-e2e.sh
  - scripts/mesh-authn-kind-e2e-cases.sh
  - ext-authz/src/config.rs
  - deploy/apps/overlays/kind-mesh/platform-services.yaml
summary: >-
  Distillat de la session d'orchestration corrigée : S2S 0308 vert,
  polling 60/2 s, IT Docker durcies, migration role_permissions Hive,
  isolation des certs, runtime éteint +8,5 Go. Pas de commit.
provenance:
  extracted: 0.70
  inferred: 0.22
  ambiguous: 0.08
created: 2026-10-03T15:10:00Z
updated: 2026-10-03T15:10:00Z
---

# Clôture 0308 — conversation OpenCode 2026-10-03

Journal : [[journal/2026-10-03]]. Canon : [[projects/aiforall/decisions/0308-mesh-authn-jwt]]. Politique persistée : [[projects/aiforall/concepts/local-runtime-lifecycle]]. Harnais : [[projects/aiforall/concepts/orchestrator-agent-harness]].

## Ce que la session a prouvé

1. L'orchestration OpenCode ( GLM Flash + agents `.opencode/` ) a tenu son contrat sans re-dérivation d'architecture : le checkpoint routage a fait implémenter par hard-implementer, corriger par implementer/mechanical-worker, valider par correctness/test/security/rust-perf reviewers, exécuter par container-runtime-debugger et k8s-operator. Les revues sont restées au courant du diff via `.cursor/review-briefings/*.md` et INDEX.md à jour.
2. E2E `exit 0` 35 OK — 4 cas positifs S2S `token`/`revoke` gateway cert sans JWT (200), Bearer sans principal refusé, gate interne 403, `mesh-client` et 4 certs applicatives 401, token post-révoke 404, hive `create_organization` + owner avec `issuer` (bloc prioritaire monitoré).
3. IT Docker durcies : IAM 13 + 10 OK ; ext-authz 12/12 ; rustycog 4/4 ; hooks `docker friendly` jamais fait `rm -f`.
4. Polling JWKS confirmé : défaut binaire **60 s**, overlay Kind **2 s**, `staleness` de révocation 60 s plafond. L'ancien 300 s a disparu de `ext-authz/src/config.rs`; les reviews l'ont attrapé (bloc Rust-Perf).
5. Migration Hive `m20261003_000014_role_permission_organization_scope` : unicité `(organization_id, permission_id, resource_id)` ; `down` restaure l'ancien index (peut échouer sans perte de données) ; tests : deux organisations, dup intra-org refusée, round-trip up/down avec conservation des IDs, downgrade sous collision refusé WITHOUT loss. 19/19 validés en Docker, migration appliquée au pod de la couche.
6. **Delta rustycog (3 fichiers) toujours dans le working tree ; pas de push ni bump de gitlink à la fin de session.**

## Ordre d'exécution réellement tenu (pour reprendre)

1. Revue initiale et mise à jour INDEX.md **avant** exécution (pas après).
2. Builds images et IT en Docker bustés (un cargo à la fois, cache Linux volume) → `kind load docker-image` → `rollout restart hive`.
3. E2E wrapper `bash scripts/mesh-authn-kind-e2e.sh` depuis Git for Windows bash.
4. Arrêt runtime : fixtures propres d'abord, puis nœuds Kind, puis Docker Desktop, puis `wsl --shutdown` avec mesure avant/après.

## Leçons et pièges (nouveaux)

- La revue sécurité initiale a attrapé **les certificats passerelle montés dans les pods applicatifs** (S-1 HIGH) — corrigé par projections items-listes strictes (CA + `<svc>.crt/.key`); le garde-fou a résisté, il est testé par mutation (`--static-only`, mutants après lignes blanches, aplombes, emprunts).
- Le seul point encore **bloquant** pour l'ADR reste le paquet git : trois fichiers rustycog à publier sur Djoe-Denne/rustycog (main), bump gitlink dans AIForAll, puis docs canoniques (ADR 0308 → Réalité Implemented) et wiki/pages dejà préparées ne nécessitent pas de redéploiement.
- Slug de récupération IT : les deux préfixes doivent être alignés explicitement (`DATABASE_DATABASE__HOST` avec `IAM_DATABASE__HOST`; `HIVE_DATABASE__HOST`, `OPENFGA_OPENFGA__HOST` en `--network host`).

## Related

- [[projects/aiforall/decisions/0308-mesh-authn-jwt]] — reste la source canonique.
- [[projects/aiforall/skills/running-mesh-authn-e2e]] — recette e2e complète des pièges existants.
- [[projects/aiforall/skills/running-it-tests-docker]] — recette IT Docker séquencée, durcie pendant cette session.
- [[projects/aiforall/skills/preserving-role-permission-org-scope]] — la migration Hive de cette session.
- [[projects/aiforall/concepts/local-runtime-lifecycle]] — le contrat runtime local (new).
