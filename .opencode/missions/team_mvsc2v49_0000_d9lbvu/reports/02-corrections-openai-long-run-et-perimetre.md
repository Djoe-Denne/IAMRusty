# Configuration long run — préciser le chantier avant démarrage

## En bref

Le plan actuel vise la clôture de la sécurité de l'authentification. Les nouvelles consignes demandent uniquement OpenAI, une exécution long run, la sécurité prioritaire, une architecture respectée, du code propre et une QA solide sans suringénierie. Elles mentionnent aussi « finir l'orchestrator » en respectant le plugin Ensemble, ce qui peut désigner un autre chantier.

Recommandation : ne pas mélanger les deux objectifs. Confirmer si Ensemble reste l'outil de coordination de la mission d'authentification, ou si l'objectif devient le développement de son orchestrateur. Dans le second cas, le plan et les commandes Rust d'authentification devront être remplacés avant approbation.

## Corrections enregistrées dans le seul brouillon de politique de cette mission

- Fournisseur autorisé : OpenAI uniquement. Modèle existant conservé : `openai/gpt-6.1-sol`, variantes low, medium, high et xhigh, sans modification de leurs métriques.
- Version 2 et seuil de départ 50 inchangés ; high et xhigh atteignent ce seuil. Classification toujours désactivée ; aucun choix de modèle ou score injecté aux tâches.
- `autonomy.mode = long_run` avec les priorités utilisateur : sécurité, respect de l'architecture et de l'existant Ensemble, code propre, QA proportionnée, recherche internet lorsque pertinente, arbitrages courants autonomes dans les limites approuvées.
- QA proportionate conservée, avec un critère explicite contre la suringénierie. Les 14 commandes de référence restent inchangées tant que l'objectif n'a pas changé. Light ou reinforced restent des alternatives si l'utilisateur souhaite les proposer ; aucune nouvelle série de questions QA n'est nécessaire ici.
- Garde explicite : clarifier la mention de l'orchestrateur avant exécution, sans changer silencieusement l'objectif enregistré.

## Choix à préciser

- **Clôturer l'authentification** : conserver le chantier étudié et son plan ; respecter Ensemble comme outil de coordination. Les risques d'authentification restent ceux déjà documentés.
- **Finaliser l'orchestrateur Ensemble** : changer de chantier ; réévaluer le plan, la QA et les commandes contre le plugin avant start. Aucune correction d'authentification ne serait implicitement incluse.
- **Laisser en pause** : aucun développement ne démarre et les risques constatés restent ouverts.

Aucune durée ni coût total n'est estimé. Le long run ne supprime pas les limites de sécurité, permissions, publication ou runtime et ne prend effet qu'après start utilisateur.

## Vérifications et limites

JSON relu et parsé : fournisseur unique OpenAI, quatre variantes natives existantes, niveau proportionate, 14 commandes, 12 critères, sept instructions long run ; aucune clé inattendue au-delà du champ autonomy prévu par le contrat long run. Le schéma et l'admissibilité natifs seront représentés par config ; cette lecture n'est pas une approbation.

Aucun réglage global, plan applicatif, source, test ou runtime modifié. Aucun worker ou classificateur démarré. Ce rapport trace la correction, sans réécrire le rapport précédent.

Après clarification, demander `/ensemble-mission config` pour afficher la nouvelle proposition exacte, puis attendre le `/ensemble-mission start` de l'utilisateur. Le hash de la proposition précédente ne couvre pas ces corrections.
