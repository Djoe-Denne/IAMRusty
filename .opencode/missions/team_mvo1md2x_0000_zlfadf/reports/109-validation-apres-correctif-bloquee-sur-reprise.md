# Le contrôle corrigé passe ; la reprise reste bloquée

## But et résultat actuel

La mission doit garantir qu'une notification reste durable lors d'une panne, puis est publiée après reconstruction de l'application et accessible uniquement à la bonne personne.

L'unique validation autorisée en106 est terminée : **un test exécuté, un échec, aucun ignoré**. Le contrôle de date corrigé et revu a été franchi. Le test a également passé les vérifications d'erreur, de tentatives et d'identité avant arrêt, rétabli la destination et reconstruit l'application. Il échoue ensuite : l'état « publié » n'est pas observé dans les 25 secondes du contrôle existant.

C'est donc **un nouveau blocage plus loin dans le scénario**, pas le défaut de comparaison temporelle précédent. Le correctif tests-only est conservé ; aucune production ou limite de temps n'a été modifiée. La fin des contrôles de replay et d'accès du cas panne n'est pas atteinte. La publication du SDK et la référence finale restent bloquées.

## Ce que les traces disent, et ce qu'elles ne disent pas

Les traces montrent un échec de publication pendant la panne, puis l'arrêt et la reconstruction de l'application. Le nouveau dispatcher démarre et continue ses requêtes, mais aucune nouvelle publication n'est visible avant le timeout.

Une piste de lecture du code est une course à l'arrêt : l'ancien dispatcher peut prendre en charge une intention pendant qu'on l'arrête. Une prise en charge en cours peut avoir un verrou temporaire de **30 secondes**, alors que le contrôle de reprise attend **25 secondes**. Une future annulée après l'envoi d'une mise à jour SQL peut aussi produire un log incomplet sur son résultat. Cela pourrait expliquer une attente après reconstruction, **mais l'état exact et les dates de la ligne à cette frontière ne sont pas présents dans ce run**. Ce n'est pas un défaut de production prouvé, ni une autorisation d'allonger le test.

La correspondance du code partagé a été vérifiée par empreinte avant l'examen symbolique : dispatcher identique entre racine canonique et source exécutée. Les symboles start/stop/dispatch_once/claim_batch ont été lus. Les fixtures et le test non mergés sont lus directement dans TI, car absents de l'index canonique. Aucun index modifié et aucun runtime relancé.

## Nettoyage indépendant

Quatre conteneurs créés : trois absents après teardown ; SMTP arrêté Exited(2), conservé ASK. Deux nouveaux volumes opaques restent ASK. Aucun conteneur possédé actif, aucun Cargo. Les onze conteneurs préexistants conservent leurs identités et états ; le workload étranger reste actif et préservé. Aucun arrêt global Docker/WSL ni suppression de volume. Vérification persistée à côté du résumé de campagne et reclassification SMTP append-only au registre.

## Choix proposé

L'autorisation106 de correction et d'une validation est consommée. Aucun autre test ou correctif de production n'est autorisé automatiquement.

**Recommandation : autoriser un diagnostic unique de la frontière arrêt/reconstruction.** Ajouter seulement des lectures SQL passives pour voir l'état, la prochaine date de tentative et le verrou avant l'arrêt, après l'arrêt et pendant la reprise, puis une seule exécution du même scénario. Conserver production, assertions, délais, backoff et sampler. Utiliser les mêmes fixtures et le même lanceur ; inspection du diff avant exécution et cleanup tracé. L'objectif est de distinguer une intention normalement verrouillée, un verrou abandonné ou une autre cause, pas d'obtenir un vert artificiel. Pas de nouvelle infrastructure, publication ou correction automatique. Durée non garantie ; l'observation pourrait rester inconclusive.

**Autre choix : conserver le blocage sans nouveau diagnostic.** Aucun changement ni exécution. Les corrections et preuves acquises restent conservées, mais la reprise complète n'est pas validée et la publication reste bloquée. Le risque fonctionnel non résolu n'est pas masqué.

## Annexe technique

- Campagne `vertical-validation106-ac5ddb9998204cdb`, AI1f2 + testSHA `6ac7be0e9e220a501241a0b48bf261c6fd7e14aa921ab098dbbfa29e7df071e1`, SDKd60 propre, index129 inchangé.
- Verdict libtest :0PASS/1FAIL/0ignored/6filtered,50.11s, Cargo101. Launcher1 pour échec du test, pas erreur préalable d'environnement. Aucune relance.
- Source test : appel wait_status(PUBLISHED) ligne556, après stop544 et reconstruction554. Panic fixture services/Telegraph/tests/fixtures/outbox_runtime.rs:147, `outbox reached expected durable state: Elapsed(())`.
- Chronologie : échec sortant08:43:47.495Z, stop08:43:53.548Z, nouveau dispatcher08:43:53.719Z, dernier polling08:44:18.672Z. Une mise à jour de claim est loggée à08:43:53.549Z avec0rows, insuffisant seul pour déduire l'état effectivement persisté lors d'une annulation.
- Preuves : TI/.tmp/vertical-validation106-ac5ddb9998204cdb/{summary.json,notification_created_routing_test.log,before.jsonl,after.jsonl,lead-cleanup-verification.json,fixture-*.jsonl}. Logs privés, pas à publier.
- Source SDK : OutboxConfig lock_timeout30s ; claim_batch132–188 inclut reprise publishing après locked_until et des awaits SQL ; start85–103, stop105–114, dispatch_once116–130. Hypothèse à vérifier, pas verdict causal.
- Localisation : transition de la première application vers la reconstruction dans le test et dispatcher partagé. Impact : reprise non prouvée après panne/arrêt. Limites : valeurs de ligne manquantes à cette frontière ; aucune conclusion globale/ADR Accepted/pin/pub.
