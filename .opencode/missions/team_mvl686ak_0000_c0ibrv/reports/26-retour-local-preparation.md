# 26 — Retour de GrepAI en mode local

## Demande / périmètre
Utilisateur : repasser GrepAI en local, le graphe étant recalculé à chaque démarrage.
Retour des sept profils précédemment migrés : extraction RPG locale native,
embeddings Ollama existants conservés. Pas de nouveau calcul pour appliquer cela.

## Préflight observé
Les sept configurations ont `feature_mode: hybrid` et embedder `ollama`.
Aucun processus watcher GrepAI en cours. Les graphes des sept racines ont été
empreintés avant modification dans `.../LiteLLM/local-mode-before-20261023.json`.
Aucune configuration privée affichée ; credentials seulement empreintés.

## Changement borné
- Passer les sept configs et leurs trois modèles publics existants à `local`.
- Préserver les champs LLM inactifs et credentials, index, backups et cache.
- Corriger l'autostart LiteLLM : endpoint dormant ne doit plus déclencher le proxy
  quand feature_mode est local. Tests sans modèle ni runtime à démarrer.
- Recharger les managers MCP par marqueur de déploiement, catalogue seulement,
  sans appeler d'outil GrepAI utile ou lancer de watcher.

## Risques / limites
Le local ne corrige pas BuildFull : le prochain démarrage reconstruira encore le
graphe, mais sans appels Luna pour son extraction/résumés. Les snapshots enrichis
restent disponibles. Le proxy déjà en mémoire n'est pas détruit, pour préserver
son cache. Pas d'arrêt/désinstallation d'Ollama, d'OAuth ou de LiteLLM.
Source native vérifiée : `cli/watch.go:1128–1143`, mode local sélectionne
NewLocalExtractor. Scripts hors racine Serena, contrôlés directement.
