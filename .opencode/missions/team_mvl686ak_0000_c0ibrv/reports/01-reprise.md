# 1 — Reprise et disponibilité GrepAI

## Objectif et étape
Achever le déploiement global LiteLLM natif et débloquer la disponibilité du grand
index AIForAll pendant l'enrichissement RPG. Étape : diagnostic de reprise.

## Résultats vérifiés / continuité
- Déploiement de sept profils, authentification, fixture watcher, cache et
  démarrage froid déjà prouvés : `.cursor/handoffs/20261022-mcp-optimization.md`,
  sections finales. Ces preuves sont réutilisées, sans relancer les générations.
- Reprise : proxy LiteLLM présent (listener Python 47400), ancien bridge Node
  39812 encore présent ; aucun arrêt entrepris.
- Le PID watcher 43348 persiste dans watcher.json mais n'apparaît plus dans
  l'inventaire courant des processus GrepAI : métadonnée obsolète à réconcilier.

## Échecs et inconnues
- Deux tentatives readiness échouées : 30 s natif puis 150 s helper.
- Le dispatcher subagent refusait emergency-thinker ; aucun travail expert admis.
- La cohérence lecture/écriture de l'index pendant l'enrichissement reste à établir.

## Choix et recommandation
Ne pas prolonger encore les timeouts. Utiliser la continuité Ensemble pour le
profil d'urgence, en conservant modèle et leadership ; intervention bornée sur
le lifecycle global. Aucun downgrade xhigh, aucune suppression d'index.

## Décision
Aucune décision utilisateur requise à ce stade ; poursuite du contrat autorisé.
