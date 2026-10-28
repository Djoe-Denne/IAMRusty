# Preuve après expiration autorisée avec budget explicite

Après le rapport121, l'utilisateur choisit **« Prouver après expiration »** : préparation tests-only puis une seule preuve de composant, sans nouvelle verticale complète ou publication. Le changement de critère temporel est explicite : deadline fixée au début de reconstruction, attente résiduelle d'éligibilité au plus30s puis25s pour publier, donc55s maximum. Aucun reset aux retries. IAM25s et arrêt6s inchangés.

## Portée

Compléter la brancheAC3 prévue par le contrat, sans nouveau produit, libération de claims, modification de verrou/backoff/config ou nouvelle architecture. Le témoin117 et ses archives restent inchangés : ils prouvent l'absence de reprise avant expiration, pas l'envoi après. Le rouge109 et le vert110 restent séparés ; le prochain PASS éventuel ne prouvera pas les valeurs historiques109 ni les autres gates métier.

Même test-engineer pool14, continuation native sansfresh, high autorisé à confirmer après admission. Source de départ du fichiercomposant403e8c90 intégrée chez le lead et l'auteur. Seul `services/Telegraph/tests/outbox_runtime_tests.rs` modifiable pour un nouveau test et des helpers privés nécessaires ; anciens tests et oracles inchangés. Préserver le vertical temporaire1105f02 chez l'auteur sans l'exécuter et le vertical produit6ac7 chez le lead. SDKd60propre/index129/ADR/ACinchangés.

PlanQA intégré avant admission : `.cursor/qa-plans/20261027-component-post-expiry122-once.md`. Réutiliser fixturesSQL/SQS, EventPublisher injecté/dispatcher existants ; aucun hook de production, dépendance, feature ou harness d'exécution nouveau. Une enveloppe de publisher purement test-only peut observer puis déléguer au vrai transport, si nécessaire pour établir le nouveau propriétaire avant effacement du lock, sans remplacer l'envoi réel.

Auteur : formatage et compilation cible --no-run seulement, aucun test/runtime. Lead : inspection/archives/intégration, vérification de la borne et des oracles, puis préflight/bail frais et **une seule exécution** de ce nouveau filtre. Aucun budget de retry, verticale, suite globale, correctif ou publication implicite. Une précondition d'horloge/état hors borne reste non concluante/rouge ; ne pas augmenter le délai pour passer.

## Garanties conservées

Vrai claim terminé avant annulation, propriétaires détruits/joints sous6s, même DB/row/event/aggregate/payload/comptes métier ; activité réelle avant et après éligibilité E, absence de reprise avant E ; reprise par nouveau worker/tentative observable, vrai transportLiveNONNoOp, étatpublished après succès et bytesSQS corrélés. Pas de SQL synthétique de verrou/échéance. R mesuré une fois depuisUTCt0, conversionmonotone consignée, deadlineconstructionincluse D=t0+R+25<=t0+55. Reprise+cleanup<=61s depuis t0, pas durée totalefixtures niSLAproduit.

Résultats117 etcleanup restent historiques ; nouveau préflight/inventaire requis avant run122, nouvelles identités tracées/cleanup mêmeéchec/zéropossédé actif. Préexistants etASK préservés ; aucun arrêt global Docker/WSL.

## Admission de préparation

Tâche `task_mvqr4705_0006_qhvjvh` publiée sans contextefresh, dépendant de la consultation architecturale terminée. Le dispatcher réaffecte le même pool14/session/branche/arbre, working vérifié une fois. Huit inputs sont copiés et leurs empreintes vérifiées avant admission ; baseline composant403e8c90 identique au lead et à l'auteur. Confirmation transmise avant réglagehigh/édition. Deux coquilles sans portée dans la description native sont corrigées par message au même auteur ; aucun identifiant ou résultat supplémentaire n'en est déduit. Préparation seulement, aucun run122 consommé.
