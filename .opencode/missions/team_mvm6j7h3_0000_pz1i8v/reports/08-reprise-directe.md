# 08 — Reprise directe : régression Windows prouvée

## Mandat et étape
Continuation directe demandée par l'utilisateur, sans Ensemble ni nouveaux agents.
Les résultats et réserves du [rapport 06](06-workspace-resultats.md) restent valides.

## Delta vérifié
Le test `windows_vfs_prefix_must_resolve` échoue sur le helper upstream :
`None` au lieu de `Some("src\\lib.rs")`, après compilation réussie (exit test 101).
Preuves privées : `C:/Users/djden/AppData/Local/rust-code-mcp/logs/native-20261024-134424-185-j3.*.log`.
Ce résultat établit un défaut de résolution VFS Windows, pas encore son effet
sur tous les appels HIR du workspace.

## Correction en cours et limites
Helper partagé pour canonicaliser fichier/racine et rejeter les chemins hors
workspace, utilisé aussi par le loader Cargo. Références exactes inspectées dans
usages, impls, extract et audits. Les sources externes ne sont pas couvertes par
la connexion Serena AIForAll : édition fichier ciblée, sans réactivation partagée.
Régression positive + contrôle négatif `..`/hors racine avant déploiement.
Ni graphe persistant ni embeddings remplacés à ce stade. Trois jobs Cargo.

## Suite autonome
Valider paths et fixture HIR, corriger identité/cache et exclusions sans abaisser
la protection des secrets, déployer côte à côte, sauvegarder avant rebuild graphe.
Puis terminer routage et profils Serena, retirer GrepAI uniquement d'AIForAll,
vérifier QMD/Obsidian et la reprise/incrémentalité. Aucun Docker/Kind/WSL requis.
Pas de nouvelle décision utilisateur requise dans le périmètre approuvé.
