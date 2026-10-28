# Composant122 — reprise après expiration, une seule exécution

Autorité : choix utilisateur«Prouver après expiration» après121, contrat122. Cible déjà prévueAC3, pas nouveaucomportement produit ; modifier la temporalitéQA est explicitement autorisé. AC7/stop6s etIAM25s restent inchangés. ADR0313/0503Proposed,0301Accepted/Partial,0200Accepted/Implemented ; aucune promotionADR.

## Ownership et entrées

Même auteurpool14 après admissionnative/high autorisé. Base outbox_runtime_tests403e8c90/AI1f2/SDKd60clean/index129. Un seul fichier test modifiable, nouveau filtre distinct de117. Ne pas modifier l'ancien témoin ni ses helpers de façon à rendre son résultat périmé ; si partage de helpers, preuve d'équivalence exacte exigée. Préserver autres sources/staging/temporaires. Aucun codeSDK/production/configuration/dépendance/feature/nouveauharness ou écritureSQL fabriquant état/expiration.

## Protocole et oracles

1. Réutiliser vrai service/notification/outbox et fixtureSQL/SQS. Suspendre le publisher existant à son entrée, après claimSQL réel terminé. Observer publishing/identité/payload/tentatives/ancienlocked_by/locked_until nonnull. Défautslock30/poll2/backoff5/max20 inchangés. Annuler le propriétaire à une frontière locale explicitée, pas pendantSQL109 ; vérifier destructionpublisher/parent et jointure sousbudget6s, ycompriserreurs/préconditions.
2. t0UTC et t0monotone à **l'entrée de reconstruction**, AVANT observation/creationfactory/dispatcher. Construction incluse. Lire passivement la même ligne pour fixer une seule fois E : locked_until si publishing ; next_attempt_at si failed/pending ; état attendu/précondition et attempts<max à vérifier, pas d'identité supposée.
3. Calculer R=max(0,E−UTCt0), exigerR<=30s pour ce claim contrôlé. Fixer Dmonotone=t0monotone+R+25s <=t0+55s. Journaliser horloges, E,R,D et cohérence de conversion. Une observation tardive n'autorise pas à repartir de l'heure de lecture ; aucun reset si retry/SQLlent. État/horloge/date manquants ouRhorsborne = nonconcluant/échec de précondition, pasextension. Pas d'horloge artificielle.
4. Reconstruire sur le même pool/DB avec publisher concretLiveNONNoOp et vrai transport. Le setup et toutes mesures/appels métier de cette phase restent sousLAdeadlineD. Propriétaires possédés/joints, pas de tâche détachée. Un pilote localdispatch_once est acceptable mais doit être identifié commecomposant, pasapplicationentière.
5. Vraies sélections avant ET après E, cadence par défaut2s, pas simple sleep àexpiry puis appelisolé. Zéro reprise avantéligibilitémesurée ; après E nouveauclaim/tentative/worker observables, même row/event/aggregate/payload ; counts métier conservés. Compteurs started/completed/claims ou preuve équivalente pour rejeter inactivité et cyclepartiel. Ne pas confondre une sélection qui chevauche E avec une reprise prématurée ; vérifier l'heure et le verrou réellement persistés lors duclaim.
6. Établir PUBLISHED après succès du vrai publisher. Si nécessaire, wrapper strictement test-only du portEventPublisher lit/consigne le nouveauclaim avant de déléguer au vrai transport ; ne pas simuler le succès ni ajouter unhookSDK. Observer véritable messageSQS et corréler envelope/bytes/event_id/aggregate/version/payload ; pas d'impression de données sensibles. Les échecs/transitoires consomment le même budget, sans décalage deD ni augmentation d'essais.
7. Tous probes/receives/SQL sousD, aucun await réseau indépendant qui dépasse la fenêtre. Résultat de sélection/claim incomplet, broker/message manquant, publication nonobservée, identité changée ou horlogeincohérente ne peuvent pas devenirPASS. Prévoir erreurs et annulation avant assertions finales ; propriétaires arrêtés/joints sousbudget6s, reprise+cleanup<=61s depuist0. Pas de message ou de lecture aprèsD pour sauver un verdict.
8. Champs sûrs de trace : stage,IDs,état,tentatives,next/locks/updated,présence booléenneerreur,UTCreads/monotone/E/R/D/cycles/destructions ; pas payload/JWT/rawerror/ModelDebug. Conserver les assertions de payload enmémoire avec messages anonymisés.

## Limites explicites

Un PASSferme seulement la brancheAC3 contrôlée aprèsexpiration. Pas de valeur109reconstruite, défautdeperte/authz/AC7inventé, exactly-once/SLA55s/wholeappwatchstop. La verticale métier, vrai SyncEventHandler/OpenFGA/replay/droits/403200, publicationSDK etpin restent gatesdistinctes NONautoriséespar122. Conserver117etsonpronostic ; pas retry-until-green ouverticalsupplémentaire.

## Préparation et gate

Lireplan/autorisation/avis121, vérifierbase etefforthighnativeaprèsadmissionavantedit. Fmtduseulfichier et `cargo test --locked -p telegraph-service --test outbox_runtime_tests --no-run` Windows24uniqueaprèspréflight/bail ; AUCUNtest/SQLruntime/Docker/Kind/WSLàcettephase. Livrerdiffcontre403e8c90/sourcepatchSHA/filtreexact/protocoleE-R-D/codesetlimites,preuveancien117inchangé,puisSTOPinspectionlead.

Leadarchive/intègreetinspecteoracles/délais,préflightruntimebailfrais,admetUNrun122surfiltreexactvialanceurexistant. Aucunautrerunautorisé. Ressourcesanciennnespasadoptées,créationsfraîchestracées,cleanupmêmeéchec/0possédéactif,ASKetnodeétrangerpréservés,pasarrêtglobalDocker/WSL.
