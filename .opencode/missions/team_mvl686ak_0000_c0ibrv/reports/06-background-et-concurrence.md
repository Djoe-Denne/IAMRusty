# 6 — Campagne lancée ; conflit du proxy partagé

## Objectif / état
Reconstruction AIForAll en arrière-plan et monitoring demandés par l'utilisateur.
Campagne démarrée ; isolation GrepAI par projet confirmée. Qualité d'enrichissement
non certifiable à ce stade à cause de rejets locaux du proxy partagé.

## Résultats vérifiés
- Premier appel MCP réussi en 4,1 s : phase `warming`, index persisté accessible.
- AIForAll : daemon PID **39740**, créé `2026-10-23T16:30:42.2775050Z`, parent
  manager 21224, log-dir `projects/aiforall/grepai-state/logs`, déploiement v5.
- Moniteur PID **43668**, créé `2026-10-23T16:30:50.6769940Z`, shell background
  `sh_16999d07e001SY09G6vlHkfua8`. Échantillonnage 15 s, heartbeat 60 s.
- Scan initial : 5 réindexés, 11 chunks, 2061 fichiers inchangés ignorés.
- Sept profils privés vérifiés PASS après correction du parseur de guillemets YAML.
- 24 tests ciblés PASS ; aucun nouveau build applicatif, Docker, Kind ou WSL.

État consultable :
`C:/Users/djden/AppData/Local/LiteLLM/rpg-aiforall-20261023/progress.md`
et `progress.json`, `job.json`. Pas de serveur preview créé.

## Constat nouveau et preuve de séparation
Le monitoring a signalé des HTTP 429. Les logs LiteLLM précisent :
`Deployment has all max_parallel_requests slots in use`.
Ce sont des rejets du plafond de concurrence local, pas une preuve de quota
ChatGPT épuisé. `max_parallel_requests:1` est configuré globalement.

Un autre consommateur actif a été observé : **ARTIST**, daemon PID **48508**,
parent manager 40380, log-dir `projects/artist/grepai-state/logs`. Les connexions
TCP vers le port LiteLLM 4000 ont des propriétaires distincts : PID 39740 et 48508.
Les instances GrepAI, leurs racines et index ne sont PAS mutualisés. Seul le proxy
LiteLLM est partagé. Ne pas arrêter le watcher ARTIST ni son manager.

Le pourcentage calculé à partir des réponses globales est donc **non fiable pour
AIForAll seul** ; le fichier de progression contient la réserve de compte partagé.
Le champ technique `foreignTraffic:false` du moniteur n'est pas une mesure et
ne doit pas être utilisé : le trafic ARTIST est maintenant confirmé.
Les 429 peuvent entraîner des replis locaux ; pas de quitus qualité.

## Options / recommandation
1. Conserver le proxy partagé et augmenter sa concurrence bornée pour les sept
   consommateurs. Nécessite un bref redémarrage coordonné dans cette installation
   sans DB : l'API native de modification des modèles refuse les modèles YAML.
2. Ne pas redémarrer : préserver tous les appels actuels, mais maintenir les 429
   et le risque d'enrichissement incomplet ; non recommandé comme résultat final.
3. Déployer des proxies dédiés : plus de processus et gestion OAuth à revoir ;
   hors correction minimale, non recommandé pour résoudre ce seul problème.

Recommandation : option 1, sans toucher à la séparation GrepAI. Concurrence 1 → 7,
pas de plafond de nombre de requêtes. Le redémarrage peut interrompre les appels
LLM en cours d'AIForAll et d'ARTIST et vide le cache mémoire, pas les credentials,
les index ni les caches persistés. Aucun redémarrage effectué sans arbitrage.

## Décision demandée
Autoriser le bref redémarrage du seul proxy LiteLLM partagé pour appliquer cette
concurrence, en acceptant l'interruption possible des appels LLM en cours ?
