# 5 — Préparation du lancement en arrière-plan

## Objectif / autorisation
L'utilisateur autorise explicitement la reconstruction RPG en arrière-plan avec
monitoring et, si possible, pourcentage. AIForAll seul, Luna xhigh conservé.

## Mise en place
- Démarrage exclusivement par le lifecycle partagé 1MCP, pas de watcher manuel.
- Un moniteur natif écrit un état Markdown/JSON toutes les 15 s, vérifie le PID
  et sa date de création, puis entretient le même bail toutes les 60 s.
- Les heartbeats fournissent l'identité attendue : si le daemon est mort ou
  remplacé, aucun nouveau rebuild n'est démarré par le moniteur.
- Arrêt du moniteur à readiness native ou sur erreur ; fin du bail de campagne,
  puis retour au délai idle partagé habituel. Pas de relance automatique.

## Vérifications
24 tests ciblés PASS : daemon registration/readiness, marqueurs Git, propriété,
arrêt idle, anti-redémarrage du heartbeat, autostart LiteLLM et calcul du progrès.
Un contrôle de déploiement a décelé un défaut du **vérificateur**, pas de secret :
GrepAI avait retiré les guillemets YAML de sa clé locale. L'identité de la clé et
le routage ont été confirmés sans l'afficher ; vérificateur corrigé.

## Pourcentage / limites
GrepAI daemon n'exporte pas le compteur par nœud de son UI. Le pourcentage sera
donc une plage estimative fondée sur les réponses LiteLLM depuis le lancement
et 15000–30000 requêtes attendues. Le proxy est partagé : cache et autres clients
peuvent être inclus. Ce n'est pas une mesure de tokens ou un progrès exact.
100 % seulement après la readiness native ; les erreurs/replis sont distincts.

## Décision / suite
Lancement autorisé, pas de nouvelle question requise. Pas de nouveau plafond,
pas de modification du binaire GrepAI, aucun Docker/Kind/WSL. Le succès runtime
et les identités de la campagne restent à constater au lancement.
