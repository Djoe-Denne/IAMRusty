# Rapport 98 — arbitrage d'une expérience diagnostique temporelle

## Objectif et état

La compatibilitéHTTP C1/C2/C3 et RPconcurrence sont corrigées/revues dans leur portée. NoOp/dégradé final1PASS ; verticale finale1FAILED sur l'assertion `next_attempt_at > Utc::now()` (rapport97). **Pas de PASSglobal, publication/pin toujours bloqués.** Aucun correctif production/test après cet échec ; Cargo0 etownedrunning0 vérifiés97, retenusASKpréservés.

DiagnosticRO même test-engineer reçu ; lead recoupe les logs897/974–991/998–999 etsourcesdispatcher33–49/230–268, fixtures136–147, sampler322–334. Aucun nouveauCargo/runtime/édition par ce diagnostic.

## Faits établis et limite d'attribution

- Second IAM :21:17:11.966 ; notification SQL observée17.845 (5.879s).
- Publisher SQS échoue17.997, UPDATEfailed observé18.002 (valeurs paramètres nonloggées).
- Le test joint SQL au samplerpassif0/1/3/6/12s ; la suite ne reprend qu'après12.010s. Lectureoutbox23.979–24.057, soit≈6s après UPDATEfailed. Le délaiSQL25s reste inchangé.
- SDKdefaultpoll2s/basebackoff5s ; claim sélectionne légitimementFAILED quandnext_attempt_at<=now etbudgetrestant. FAILEDpeut doncêtreDÛ avantleprochainclaim sansbug. `wait_status` renvoie étatFAILED sansgarantiefraîcheur.

Contre-exemple sourcecompatible : failedvers18s, prochaineéchéancevers23s, poll22strop tôt, lecture24s avantprochainclaimvisible : assertionfuture-at-readfaussemalgrébackoff5scorrect. Cela prouve que l'oracle actuel n'est pas universellement impliqué par le contrat durable/retry. **Cela ne prouve pas que ce run a réellement écrit23s** : attempts/updated_at/next_at/UTCcomparé ne sontpas capturés. Bugprodbackoff non établi et non exclu ; pas réattribution exclusive à Receive/visibilité/HTTP. Ancienvert etrouges restentarchivés.

## Recommandation à autoriser

**Une seule exécution diagnostique du même test vertical**, précédée d'un diff test-only inspecté et d'un préflight/bail frais, via fixtures/lanceurexistants. Pas de nouveauharness, backend, dépendance ou feature. Quatrefixturespotentiellesenimagescached commeledernierbinaire ; seulsnouveauxIDspossédésstop, Exited0retirésparguardexistant, SMTPnonzero/volumesopaquesASKpréservés ; nodeetancienscontainersintacts. Pas de démarrageKind/globalWSLshutdown.

Instrumentationpassive test-only bornée par **la même deadline25s** : aprèsnotificationSQL, capturer premièretransitionFAILED de la ligneexacte (IDs/status/attempts/updated_at/next_at/présenceerreur/UTCavant-aprèslecture/tempsmonotone), changementsseulement, sansécritureSQL. Capturer mêmesvaleursaupointdurable et justeavantassert, avec uneUTCutiliséeparl'assert. Confirmerconfigeffectivedupoller/backoffdanssetupsource, pasdevaleurprésumée.

**Conserver l'assertion actuelle et le sampler12s, visibilité5s, backoff, délai25s, replay/FGA/oracles métier inchangés.** Aucunefixprod ou remplacement `>now` par `>updated_at` ; cedernierseul seraitinsuffisant. Échec/vertdu run ne devientpasacceptationautomatique : il sertàdiscriminer earlyFAILEDschedulingcorrectpuisdûtardif vs valeursréellementincorrectes/autrescauses.

Aprèslesfaits seulement, proposer le caséchéant unoracle équivalent ouplusfort validécontreimmediate-retry/stalescheduling, puis demander l'arbitragenécessaire avant lemodifier. Ne pas relancer jusqu'auvert ni augmenterlatemporalitépourréussir.

## Choix et compromis

- **Autoriser ce diagnostic (recommandé)** : unrun ciblé supplémentaire etquelquesrequêtesSQLpassives, assertion inchangée ; fournit valeursmanquantes sansamoindrirgaranties. Risque de perturbationobservateur documenté, borné et non utilisé pour réécrire rétroactivement97.
- **Conserver le blocage** : aucunrun/édition ; livrerdocumentationétatpartiel etpreuvesvalides, verticale nonvalidéedéclarée, paspublication/pin/clôtureglobale.

Décisiondemandée : autoriser uniquement cette instrumentationtest-only et **unrun diagnostique**, pas uncorrectif production/ouaffaiblissementd'oracle. QAproportionnée/routagecatalogue etAC1–AC7 inchangés.

## Reprise après redémarrage du serveur

La question interactive a été interrompue sans réponse : aucune autorisation déduite de cette interruption ou des demandes de reprise. Contrôle de reprise : AI1f2dd30 et SDKd60 inchangés, SDK propre, seul M rustycog attendu, zéro processus Cargo. Aucun nouveau test ni instrumentation lancé ; la décision ci-dessus reste à recueillir. Le dernier inventaire Docker vérifié reste celui du rapport97, pas une observation actualisée après redémarrage.
