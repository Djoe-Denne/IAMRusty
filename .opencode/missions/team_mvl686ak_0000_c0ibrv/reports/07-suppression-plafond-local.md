# 7 — Suppression du plafond local, décision d'application

## Objectif / décision utilisateur
L'utilisateur refuse toute limite locale ajoutée : ChatGPT doit appliquer ses
propres limites et HTTP 429. L'option de concurrence locale 7 du rapport 6 est
abandonnée ; elle n'a pas été appliquée.

## Changement effectué
`C:/Users/djden/AppData/Local/LiteLLM/config.yaml` : suppression de
`max_parallel_requests:1`, sans remplacement par un autre plafond. Luna xhigh,
authentification et cache restent configurés. Aucun 429 fournisseur contourné.

## Limite d'application
Le processus LiteLLM déjà actif conserve sa configuration en mémoire. Les APIs
de mise à jour des modèles de la version installée exigent une DB et refusent
les modèles définis dans YAML ; aucun endpoint natif de reload config n'est
exposé dans son OpenAPI. Un redémarrage du proxy est nécessaire pour appliquer
le fichier. Aucun redémarrage exécuté dans ce checkpoint.

## Impact et options
- Redémarrer uniquement LiteLLM : applique la suppression, vide son cache mémoire
  et peut interrompre les requêtes LLM d'AIForAll et d'ARTIST en cours.
- Attendre l'arrêt naturel : ne perturbe pas ces requêtes, mais conserve le
  plafond dans le processus jusqu'à son prochain démarrage.

Recommandation : bref redémarrage coordonné du proxy, sans toucher aux daemons
GrepAI, aux index ou aux credentials. La campagne et le moniteur sont actifs ;
le moniteur détectera le changement de log et devra être repris avec la même
identité watcher, sans reconstruire à nouveau ni effacer ses compteurs.

## Décision demandée
Autoriser ce bref redémarrage du seul proxy partagé, avec interruption possible
des appels LLM en cours et perte du seul cache mémoire ?
