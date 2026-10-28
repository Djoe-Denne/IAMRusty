# 09 — Corrections ciblées et routage projet

## Étape et faits vérifiés
Travail direct, sans agent supplémentaire. Suite du [rapport 08](08-reprise-directe.md).

- Paths : 2 régressions positives/négatives passent ; HIR : 9 tests passent,
  dont appels importés entre fichiers, méthodes, traits, closures et const.
- Sécurité : 22 tests passent ; 2 tests de chunks vérifient la disparition des
  valeurs sensibles de tous les champs sérialisés et la couverture module-only.
- Audit réel, sans embedding : **914/914 fichiers candidats parsables/indexables**,
  aucun rejet. Les répertoires sensibles restent bloqués ; récupération `.rs`
  bornée avec masquage conservateur des chaînes/commentaires et rescan fail-closed.
- Serena : remplacement de corps, renommage inter-fichiers et insertion symbolique
  réels, contrôlés par exécution Python (42/42/43). Probes créées puis retirées.

Preuves privées : `logs/native-*.result.json`, `workspace-source-audit-v2.json`,
`serena-edit-proof.json` sous `C:/Users/djden/AppData/Local/rust-code-mcp/`.

## Intégration en cours
- Identité Windows : normalisation de stockage/parcours vers le chemin installé
  en `/`, identité Merkle historique conservée ; tests d'alias ajoutés.
- Suppression de fichiers : invalide aussi la metadata, pour permettre une
  recréation identique sans perdre ses chunks.
- Replay de politique source unique : garde les vecteurs des fichiers non affectés,
  récupère les exclusions et remplace les vues désormais sensibles par leurs
  chunks expurgés. Pas de ré-embedding global ; sauvegarde avant replay réel.
- Préparation v2 en cours : checks ciblés, puis build et binaire côte à côte.
  Pas encore de preuve de déploiement/reconstruction du workspace réel.

## Configuration et profils
Priorités Serena directement ajoutées aux profils écrivains OpenCode/Cursor,
renvois de découverte/review migrés vers rust-code-mcp. Règle locale et AGENTS
explicitent l'exception AIForAll ; aucun global modifié.

GrepAI : OpenCode et Cursor désactivés localement, Codex `enabled=false`, backend
retiré uniquement de `1mcp/projects/aiforall/mcp.json`. Le catalogue OpenCode a
effectivement retiré GrepAI. Les anciens index et le lifecycle global ne sont
pas supprimés. QMD/Obsidian inchangés, contrôles de lecture encore à effectuer.

Sauvegarde des 6 configurations préalables et SHA256 :
`backups/migration-v2-20261024-135804/manifest.json` sous l'installation privée.

## Limites / suite
Les PASS unitaires et de parsing ne prouvent pas encore les résultats du graphe
réel ni la persistance des nouveaux chunks. Vérifier après déploiement : appels
HIR connus, alias sans ré-embedding, deltas edit/rename/delete/recréation sur
fixture isolée, reprise après restart et intégrité applicative. Aucun Docker créé.
