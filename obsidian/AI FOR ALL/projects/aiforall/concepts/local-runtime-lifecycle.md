---
title: >-
  Cycle de vie du runtime local (Docker/Kind/WSL)
category: concepts
tags: [concept, docker, kind, wsl, visibility/internal]
sources:
  - AGENTS.md (Cycle de vie du runtime local)
  - .cursor/rules/local-runtime-lifecycle.mdc
  - .agents/skills/local-runtime-lifecycle/SKILL.md
  - conversation opencode 2026-10-03 (clôture 0308)
summary: >-
  Runtime à la demande (E2E → Kind aiforall-local ; IT → testcontainers),
  arrêt propre + libération RAM en fin de session, propriété par bail
  exact-ID, jamais de suppression automatique.
provenance:
  extracted: 0.85
  inferred: 0.12
  ambiguous: 0.03
created: 2026-10-03T15:10:00Z
updated: 2026-10-07T00:00:00Z
---

# Cycle de vie du runtime local

Contract canonique : `AGENTS.md` → section « Cycle de vie du runtime local », `.cursor/rules/local-runtime-lifecycle.mdc` (canon court), skill projet `.agents/skills/local-runtime-lifecycle/SKILL.md`. Contexte de décision : session [[projects/aiforall/references/opencode-close-0308-2026-10-03]] (2026-10-03).

## Pourquoi ce contrat existe

Avant le 2026-10-03, chaque worker redécouvrait quand démarrer Docker/Kind, qui possède quel conteneur, et comment libérer la RAM ; la fin de session laissait des nœuds Kind et des fixtures vives (découvert pendant la clôture 0308 : 2 nœuds + 6 fixtures vives, 5,7 Go de `vmmemWSL`). Le contrat est une **politique de prompts**, pas un moniteur d'inactivité : une session killée ou déconnectée brutalement ne garantit pas le teardown.

## Règles

1. **À la demande** : Docker/Kind/WSL démarrent seulement pour une preuve E2E, un test local demandé ou une IT en cours ; jamais pour du travail documentaire/statique.
2. **Canal** : E2E et tests locaux → cluster `kind-aiforall-local` exclusivement ; IT → fixtures testcontainers du [[projects/aiforall/concepts/mesh-ext-authz-opt-in|harness existant]] (jamais Kind, jamais migration Compose). **Depuis le 2026-10-07** : dev/compile/IT natif Windows (`cargo` hôte, `target\` sur E:, cf. `AGENTS.md` Décision 2026-10-07) ; Docker pour les images Linux Kind et l'infra tierce testcontainers qui démarre seule.
3. **Propriété** : inventaire AVANT (état Docker/WSL, conteneurs runnings/stoppés), bail partagé par le parent (orchestrator) sur les IDs créés/redémarrés explicitement pour la tâche. Le nom ou l'image d'un conteneur ne prouve **pas** la propriété ; un ancien conteneur n'est jamais adopté.
4. **Workers** : rendent l'inventaire de nettoyage au parent ; n'arrêtent pas une ressource encore requise par le parent ou une validation en arrière-plan.
5. **Arrêt** : à succès ou échec, avant de rendre la main, arrêt gracieux `docker stop` des ressources possédées sans besoin actif (permission permanente pour ce stop réversible ; annonce des IDs et de la perte de mémoire volatile). Stop ≠ delete : jamais prune, `rm -f`, reset, suppression de volume/container/namespace/cluster sans permission destructive explicite et ciblée (voir [[skills/docker-destructive-guardrails|garde-fous Docker]]).
6. **Fuites** : Ryuk désactivé + CLI Docker absent du runner test font fuir les fixtures à chaque run IT ; l'inventaire final est vérifié, on ne présume pas que le harness a nettoyé. Le cleanup du fixture (`docker rm -f`) **ne doit pas** être réactivé sans décision explicite.
7. **RAM** : arrêter Docker Desktop puis `wsl --shutdown` uniquement sans autre workload, bail actif, propriété inconnue ou cluster protégé affecté (`kind-apparatus-p4-it`, `rancher-desktop`) ; sinon rapporter la RAM résiduelle et demander un arbitrage ciblé. WSL shutdown arrête **toutes** les distributions. Ne jamais utiliser le CLI Docker après `wsl --shutdown` (réveil du daemon).
8. **Preuve** : mesurer `vmmem` / RAM libre hôte avant/après ; jamais prétendre « 0 Go utilisés » sans mesure. Persister un petit ledger non secret des IDs possédés avant les opérations longues.

## Preuve de référence (2026-10-03)

+8,5 Go de RAM hôte récupérés en fin de session de clôture 0308 : 6 fixtures éteintes (~202 Mo), deux nœuds Kind arrêtés (`aiforall-local` 1,75 Gio + `apparatus-p4-it` 1,08 Gio, autorisation utilisateur explicite unique — **cette permission ne se généralise pas**), Docker Desktop stoppé, `vmmemWSL` absent, RAM libre 5,9 → 14,4 Go. Aucune donnée supprimée.

## Related

- [[projects/aiforall/skills/running-mesh-authn-e2e]] — canal E2E Kind.
- [[projects/aiforall/skills/running-it-tests-docker]] — canal IT (**obsolète 2026-10-07** : IT natif Windows).
- [[projects/aiforall/concepts/rustycog-git-submodule]] — autre caution de propriété (submodule).
