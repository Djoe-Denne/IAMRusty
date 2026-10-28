# 21 — Guidage MCP appliqué ; prompt de campagne prêt

## Résultat et stade
Application terminée, campagne future **non exécutée**. Travail direct, aucun
agent supplémentaire. Le prompt sera lancé par l'utilisateur en nouvelle session,
puis cette session sera analysée sur ses traces réelles.

Continuité : [préparation 19](19-guidage-mcp-implementation-preparation.md),
[design 18](18-proposition-guidage-mcp.md),
[grille évaluateur 20](20-campagne-session-grille-evaluateur.md).

## Changements appliqués
- `AGENTS.md` : contrat MCP local autonome en tête ; suppression de l'ancien
  bloc de routage redondant. Priorité locale de procédure explicitée sans
  prétendre dépasser les instructions supérieures ou permissions.
- `.agents/skills/aiforall-mcp-workflow/SKILL.md` : méthode à charger au besoin,
  97 lignes ; référence directe `fallbacks.md` pour les incidents/couvertures.
- `.cursor/rules/grepai-first.mdc` : adaptateur court du contrat, chemin historique
  conservé. Pas de copie de la procédure complète.
- **34 profils** raccordés au contrat/skill : 17 OpenCode, 12 Cursor, 5 Codex.
  Les 14 sections écrivains ont été condensées ; les autres indications métier
  utiles restent. Modèles, permissions et responsabilités n'ont pas été édités.
  Deux anciens rappels Cursor imposant/privilégiant QMD CLI ont été réconciliés
  avec la priorité MCP locale.
- `docs/rust-code-mcp.md` : points d'entrée et portée des procédures clarifiés.
- Prompt seul : `docs/missions/20261024-mcp-guidance-user-prompt.md`.
  Grille évaluateur séparée, jamais à joindre au prompt.

Le repli sans Context Mode reste soumis à accord explicite en cas de panne
réelle. Les autres replis sont bornés par couverture/invariants/permissions.
Le keepalive 8B est conservé ; aucun backend, modèle, index ou graphe touché.
Configurations OpenCode/Codex/MCP et fichiers globaux non édités.

## Vérifications observées
- Le harness OpenCode a annoncé le nouveau skill après création et a injecté
  la mise à jour d'AGENTS. Invocation `skill(aiforall-mcp-workflow)` réussie :
  corps local et référence `fallbacks.md` effectivement chargés. Ce n'est pas
  encore une preuve d'application correcte à une mission Rust.
- Analyse via Context Mode + Python/PyYAML : **46 frontmatters YAML valides**
  (24 profils OpenCode, 20 Cursor, skill, MDC).
- Python `tomllib` : **13 TOML valides** (configuration et 12 profils Codex).
- `git diff --check -- <AGENTS, MDC, guide, skill, prompt, profils concernés>` :
  exit 0, aucune erreur d'espacement sur les diffs suivis. Les nouveaux fichiers
  ont aussi été lus/validés pour métadonnées et références.
- Une seule définition locale du skill retrouvée dans les sources skills
  configurées/usuelles examinées : `.agents/skills/aiforall-mcp-workflow/SKILL.md`.
- Recalcul SHA256 des **989 entrées applicatives** du manifeste privé existant :
  aucune différence. Pas de code/manifeste/lock applicatif modifié par ce lot.
- Prompt : **215 mots**, sans mention d'outils, skills, index, graphe ou MCP.
  SHA256 `a06aeb3a44daf4d4a61cb549e0dfa9f9069b86e4e7a964dc8ef457b14c2d9e83`.
  Le premier comptage JS avait un échappement ambigu ; le nombre retenu est
  celui de Python `len(prompt.split())`, pas le résultat JS initial.

## Limites et suites
Pas de validation comportementale des trois clients ni d'utilisation Rust/HIR
dans ce lot documentaire. Le chargement du skill est prouvé ici dans OpenCode ;
Cursor/Codex disposent du repli par lecture autorisée si leur catalogue ne l'expose
pas. Ni panne réelle ni refactor Rust testés : ne pas les annoncer validés.

Le prompt exerce découverte métier, impact, comparaison d'implémentations,
tests et décisions documentées, sans dicter une séquence d'appels. La grille
fixe des verdicts séparés et ne récompense pas la simple disponibilité des outils.
L'évaluateur devra revalider les oracles historiques à la version réellement lue.

L'utilisateur peut maintenant copier uniquement le prompt dans une nouvelle
session et revenir avec son ID/lien ou un export. Pas besoin d'une nouvelle
initialisation, d'un restart ou d'un nouveau modèle. Aucun accord bloquant pour
la remise ; aucun Cargo/Docker/Kind/WSL lancé, aucune ressource Docker créée.
Le modèle résident n'a pas été interrogé ou déchargé pour ce travail documentaire.
