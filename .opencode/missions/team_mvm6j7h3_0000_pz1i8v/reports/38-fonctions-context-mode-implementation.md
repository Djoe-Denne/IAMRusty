# 38 — Implémentation intégrée, vérification à suivre

## Changements depuis le rapport37
- Politique commune créée sous C:/Users/djden/.agents/context-mode-script-policy.md,
  avec références depuis OpenCode/Codex et une nouvelle règle globale Cursor.
  Aucune règle ancienne réécrite, aucun MCP global reconfiguré.
- Skill local créé : .agents/skills/aiforall-context-functions. Catalogue court,
  module CommonJS sans dépendance, runner CLI à allowlist fixe et procédure de
  capitalisation avec quarantaine candidates/ et métadonnées explicites.
- Intégration par petits ajouts à AGENTS.md, au workflow et aux fallbacks MCP.
- Trois opérations : lectures comparatives annotées avec préflight, classification
  des résultats de processus sans sortie brute et collecte de lectures indépendantes.
- Tests Node écrits : positifs/négatifs et faux catalogue injecté pour vérifier
  qu'un candidat ou un chemin de module ne devient pas exécutable implicitement.

## Observé / non encore prouvé
Le harness a annoncé la disponibilité du nouveau skill après sa création :
découverte effective dans ce client. Les tests et le chargement du module depuis
Context Mode restent à exécuter ; le catalogue final n'est pas encore accepté.
Les fonctions ne remplacent pas les outils spécialisés et ne font pas sandbox OS.

## Suite et périmètre
Syntaxe puis tests Node, smoke réel Context Mode, vérifications des références et
du diff ciblé. Skill local-runtime-lifecycle chargé : tests unitaires hôte uniquement,
aucune raison de démarrer Docker/Kind/WSL. Aucun coût/performance à promettre.
Pas de décision bloquante : continuer la QA proportionnée, sans nouvelle session AI.
