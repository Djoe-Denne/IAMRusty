# Prouver la reprise après le verrou prévu par le contrat

## Objectif et problème concret

Une notification doit rester durable pendant une panne puis pouvoir être publiée par une nouvelle instance, avec la même identité et les bons droits. Le témoin117 a prouvé qu'après annulation d'un propriétaire, un claim déjà enregistré peut rester verrouillé30secondes. Pendant les25secondes observées, le nouveau dispatcher fonctionne mais ne peut pas reprendre cette intention. L'envoi après expiration n'a pas encore été testé dans ce cas contrôlé.

La consultation architecturale fresh est terminée en lecture seule, avec effort natif high confirmé. Elle distingue deux garanties : l'arrêt des tâches locales en6secondes et la reprise d'un étatSQL après expiration de son verrou. **Le contrat prévoit déjà cette seconde garantie ; il ne prévoit ni libérationSQL immédiate à l'arrêt ni engagement produit de25secondes.**

Le contrôle PUBLISHED25s depuis la reconstruction ne couvre donc pas tous les états permis par un verrou30s. Son échec109 reste un échec réel du test exécuté, sans PASS rétroactif. Le mécanisme117 n'établit pas les valeurs historiques109 et le succès110 ne les remplace pas.

## Avis sur la portée

Aligner une preuve sur l'éligibilité réellement mesurée puis un budget borné relève de la spécificationQA d'AC3 déjà prévue, pas d'une nouvelle architecture produit. Aucune ADR nouvelle n'est nécessaire pour ce seul alignement. Mais **changer le départ ou la durée du contrôle est une vraie modification de son critère temporel** : elle nécessite une autorisation explicite, aucune ne restant après117.

Changer la production pour libérer immédiatement des claims à l'arrêt serait au contraire une nouvelle garantie persistée et concurrente. Cela demanderait un autre mandat et une conception contre les doublons et les effetsSQL/réseau en vol. Ce correctif n'est ni autorisé ni recommandé automatiquement par les preuves actuelles.

## Recommandation minimale

Autoriser une préparation tests-only puis **une seule preuve de composant post-expiration**, avec les fixtures/interfaces existantes et inspection préalable. Conserver117 et ses assertions : compléter la branche absente, pas transformer son pronostic en succès d'un autre sens.

Le budget proposé est fixé une fois au début de reconstruction, construction comprise :

- éligibilité E mesurée sur la ligne durable : locked_until pour publishing, next_attempt_at pour failed/pending, tentatives sous plafond ;
- résiduel R = max(0,E−UTC du début de reconstruction), au plus30s dans ce cas contrôlé ;
- deadline monotone D = début reconstruction + R +25s, donc **au maximum55s** depuis ce début ; aucune remise à zéro si SQL, poll ou retry prennent du temps ;
- conversion UTC/monotone bornée et consignée. État ou horloge manquant, résiduel hors borne : précondition/diagnostic non concluant, jamais délai étendu ;
- cleanup stop inclus reste6s : reprise + cleanup au plus61s depuis reconstruction. Ce n'est ni la durée totale fixtures/test ni un engagement de service.

Oracles à conserver : vrai claimSQL terminé avant annulation, destruction/jointure ancien propriétaire, même DB non recréée et mêmes row/event/aggregate/payload/comptes métier ; sélections réelles avant et après E, aucune reprise avant E ; reprise par une nouvelle instance après E avec tentative/worker observables ; vrai transport non-NoOp, état published après succès et bytesSQS corrélés. Aucun UPDATE fabriquant l'expiration ou libérant un verrou, aucun changement de production/configuration/dépendance/backend/harness. Poll2s/backoff5s/lease30s/budget fini inchangés. Propriétaires arrêtés et joints même en échec.

Cette autorisation couvrirait **ce composant seulement**, pas une nouvelle exécution de la verticale métier ou une publication. Même un PASS futur ne fermerait que la brancheAC3 contrôlée ; création et comptes1/1/1, panne réelle, premierFAILED au bon instant, même événement, vraiSyncEventHandler/OpenFGA sans grant, replay ledger neuf, tuple unique,403 sans mutation et200 destinataire restent des gates métier distinctes. Ne pas les revendiquer sur un témoin qui ne les exécute pas.

## Options et conséquences

**Compléter la preuve après expiration — recommandé.** Une modification de test examinée puis un seul run, sans nouveau produit ou runtime de preuve. Le nouveau budget de mesure est exposé ci-dessus ; durée totale non garantie. Le résultat peut encore être rouge ou non concluant. Aucun retry, correctif automatique ou publication.

**Conserver le blocage.** Aucun nouvel edit/test. Les corrections et preuves restent conservées ; la reprise contrôlée après expiration demeure absente et publication/pin bloqués. Aucun risque accepté tacitement.

## Annexe canon et provenance

- Consultation pool18/architecte fresh, tâchetask_mvqmhthd_0004_sscmxe, high natif accepté avant analyse. Aucun fichier,ADR,commit,Cargo,test,SQLruntime ou ressource créé/modifié.
- Copies canoniques examinées : ADR0313/0503 Proposed/Unimplemented, ADR0301 Accepted/Partial, ADR0200 Accepted/Implemented ; le code local ultérieur ne les promeut pas implicitement.
- Contrat outbox§3 lignes73–77 : publishing récupérable après expiration, instance neuve/sameDB/event_id/budget fini, singletonwriter sans fencing. AC3 ligne180 : preuve après expiration, sans supposer un poll immédiat.
- AC7/§5.2 lignes115–117 et196 : budget6s local et propriétaires/descendants réellement terminés, effets distants exclus ; contratHTTP69–80/ADR0503:27. Aucune exigence de rollbackSQL ou libération delease immédiate.
- §6.2 ligne160 impose25s au commitIAM ; le waitPUBLISHED25s après rebuild est l'oracle exécuté du test, pas un SLA de ces textes.
- Code dispatcher140–159 sélectionne selon next_at/locked_until et attempts<max ;168–184 claim persisté avantpublish ;197–205 marquage après résultat ; stop110–113 ne libère pas les claims ; setup-app236–249 peut abandonner start.
- Résultat117 :13cycles réels0claim, verrou encore4,984s au-delà de la fenêtre,1PASS/0FAIL/0ignored/5filtered35,65s. Envoi après expiration/wholeapp/SQLannulé109 non prouvés. Tousruns autorisés consommés.
- Cleanup117 demeure historique vérifié :0possédé actif/Cargo0,13préexistants préservés ; nouveaupréflight requis si nouvel essai autorisé. Aucun inventaire actuel déduit de cette lecture.
- Localisation/Impact : frontière AC3 versus AC7 et critère de preuveQA. Limites : avis de canon/source, pas nouvelle preuve comportementale ni acceptation globale/ADR/pin/publication.
