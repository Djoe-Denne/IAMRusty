# Rapport 100 — diagnostic vertical unique autorisé

## Décision humaine

Après l'explication en langage simple du rapport99, l'utilisateur choisit **« Autoriser (Recommended) »** à la question : observations SQL passives et une seule exécution du même test vertical encore rouge, sans changer production, assertion ou délais. Cette réponse est l'autorisation ; les demandes de reprise et questions interrompues précédentes n'en étaient pas une.

## Contrat borné

- Source de référence : AI1f2dd30af52a707ceb4abc2fbea3f963307c246f / SDKd60a88b2f2f499768a807941a1b4c80c9a366b3f local non publié. Rouge97 et preuves HTTP/NoOp restent immuables.
- Même auteur test-engineer pool12, même contexte. Avant édition, source isolée actualisée/vérifiée par le lead ; pas reprise sur son ancien HEADd36 ni modification de production par l'auteur tests.
- Un seul fichier test vertical : observations passives des champs état/attempts/updated_at/next_attempt_at/présenceerreur/identités/UTCavant-aprèslecture et tempsmonotone. Aucune écritureSQL diagnostique, payload/JWT/secret dans les traces ou nouvelle dépendance.
- Observer la première transitionFAILED après notificationSQL dans la future déjà possédée, **sous la même deadline absolue25s** et seulement pour la brancheoutage. Pas tâche détachée ni nouveau budget25s.
- Conserver sampler0/1/3/6/12s, visibilitéINBOUND5s, backoffeffectif, reprise, replay, FGA, identité et l'assertion de date future. Une UTC capturée au point de l'assertion sert à la mesure et à cette même comparaison, pas une heure ancienne choisie pour passer.
- Confirmer la configuration effectivement utilisée dans setup. Une valeur de configuration par défaut seule n'est pas encore preuve de son usage.

## Gate avant le seul run

Auteur prépare diff/compilation `--no-run`/fmt sansDocker et rend la proposition exacte. Lead contrôle absence de baisse d'oracle et sourcehash, puis préflight runtime/bail frais avant un seul test exécuté via fixtures/lanceur déjà existants. Pas nouveau harness/backend/feature, runNoOp ou répétition jusqu'au vert.

Les observations manquantes/timeoutdiagnostique constituent une limite de diagnostic, pas un PASS ou bugprod automatique. Un test vert ne suffira pas à effacer le rouge97 ou à publier leSDK : analyser les valeurs et proposer séparément toute correction éventuelle.

## Ressources et publication

Les anciennesfixtures arrêtées/volumesASK et nodeétranger restent exclus du nouveau bail. InventairefullID contemporain, nouvellescréations tracées, cleanup mêmeéchec/zéroownedrunning à vérifier ; aucunprune/force/globalWSLshutdown. Aucunruntime n'est lancé par cette autorisation documentaire. Publication/pin et clôtureglobale restent bloqués jusqu'à résultats et décisions nécessaires.
