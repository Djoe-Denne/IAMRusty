# 133. Reprendre la validation complète sans réintroduire le mauvais délai

## Objectif et problème concret

Terminer la mission : prouver le parcours notification, message réel, attribution
du droit et accès HTTP, puis préparer la livraison du SDK corrigé.

La reprise après expiration est maintenant prouvée sur le composant (rapport132).
Mais le test complet attend encore la publication25s après reconstruction, alors
que le produit conserve légitimement un verrou SQL30s. Relancer ce test tel quel
peut reproduire une incompatibilité de mesure plutôt qu'un défaut de production.

## Ce qui a été vérifié depuis la reprise

- Plan et ledger relus ; aucune preuve acquise n'a été rejouée.
- Le test vertical actuel, lignes544–556, conserve stop, restauration de file,
  reconstruction puis attente fixe25s.
- Le protocole fondé sur l'éligibilité persistée a réussi sur le composant :
  observations30,244s avant deadline54,990s, même identité et vrai message SQS.
- Une recherche indépendante en lecture seule examine en parallèle les conditions
  de publication du SDK ; aucun push, pin, test ou runtime supplémentaire lancé.

## Décision requise et recommandation

L'approbation précédente du budget après expiration était explicitement limitée
au composant. La demande de reprendre la mission ne doit pas être utilisée pour
étendre silencieusement ce contrat de vérification.

**Recommandation : autoriser le même protocole borné dans le test complet**, pour
sa seule reprise sortante après panne : mesurer une fois l'éligibilité stockée,
inclure la reconstruction dans l'attente, puis fixer la deadline à ce résiduel
(maximum30s) plus25s, donc maximum55s. Aucun reset à chaque lecture ou retry.
Conserver les identités, comptes, message SQS réellement reçu, vrai handler et
OpenFGA, replay à ledger neuf, tuple unique, HTTP403 sans mutation et200 du
destinataire. Garder IAM25s, lease30s, backoff/poll/max essais et arrêt6s.

Ce choix autorise préparation test-only par l'auteur existant, compilation ciblée,
revue indépendante du changement temporel puis une validation verticale ciblée
après intégration et préflight. Pas de nouveau harness ni correctif de production,
pas de suite globale ou rerun jusqu'au vert. Temps total non estimé ; seule la
borne de reprise est chiffrée. La publication reste conditionnée aux résultats.

L'autre choix est de garder le test et son budget actuels sans exécution : aucun
risque de relance inutile, mais la validation verticale et la livraison restent
bloquées. Il ne serait pas correct de prétendre qu'un PASS composant les remplace.

## Annexe de traçabilité

Source TI : AI1f2dd30/SDKlocald60, verticalSHA6ac7. Contrats AC1–AC7 inchangés ;
ADR0313/0503Proposed,0301Partial. Rapport129 = recommandation,132 = preuve composant.
Task de recherche SDK : task_mvqyq8pr_0000_eaa72e, indépendante sans mutation.
Localisation : oracle de reprise de `notification_created_routing_test.rs` ;
impact proposé : QA seulement ; limites : historique109 inconnu, aucune nouvelle
preuve verticale ou publication à la date de ce rapport.
