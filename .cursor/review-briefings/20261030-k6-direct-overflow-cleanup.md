# Correctif d'exactitude — dépassement de flux et cleanup échoué

## Décision et base

Revue indépendante PASS sécurité sur14d55e9..e9af3b6, aucun finding sécurité démontré. Suggestion correctness retenue par le lead pour fiabiliser le diagnostic avant exécution. Rapport reçu `result_mvu6482f_000d_6d08fx`, persisté `20261029T2321Z-security-k6-direct-chromium-ffe4a27.md`. Source canonique e9af3b6, HEADdocffe4a27 ; aucune exécution native de ce delta.

## Défaut statique à tester

Child lève OUTPUT_LIMIT pendant la lecture, tronque les buffers à64KiB, puis un cleanup échoué peut remplacer le code par CLEANUP_FAILED. direct_chromium classe alors le préfixe et peut donner confidentialityPASS faute d'indicateur persistant de dépassement. Le résultat global reste non réussi et aucun natif n'est publié ; ne pas inventer une fuite ou un PASS fonctionnel.

## Correctif borné

Même auteur, ops/tests/k6/** seulement : état de dépassement conservé indépendamment du code final de Child, sans modifier bornes, priorité cleanup, supervision, identité, arrêt gracieux, sandbox ou contrôles privacy. Sur dépassement, observation directe retenue et aucun confidentialityPASS, y compris quand cleanup échoue. Ne pas recopier le superviseur ou élargir le pipeline.

Régression ROUGE d'abord : simultané overflow+cleanupfailure avec préfixe apparemment inoffensif/classable et suffixe non capturé ; puis GREEN. Réutiliser fixtures existantes ou fakes Python de commandes/processus, sans créer un vrai survivant impossible à arrêter. Vérifier que l'indicateur est réellement positionné par le chemin de lecture, pas seulement mocké dans le test du consommateur. Les cas overflow seul, cleanup seul, flux normal et sentinelles restent discriminants ; tests Windows natifs permis, pas runtime/Cargo/CI/Chrome.

Base auteur14d55e9 avec cinq fichiers déjà capturés dans e9af3b6 : isoler le nouveau delta par rapport à e9af3b6, pas aux anciens changements staged. Pas rebase/reset/commitlong ou ledger partagé. Rendre commandes réelles, preuves, diff exact et limites. La source restera ensuite gelée pour intégration/validation ciblée ; aucune revue globale supplémentaire imposée.
