# Rapport 97 — deux IT exécutées, résultat partiel et cleanup vérifié

## Résultats source finale

Run **http-final-it-4d513ab1f63f499b**, AI1f2dd30/SDKd60 figés, lanceurboundedSHA bd3fcb6... ; stdout/stderrlauncher puissummary/logsréels relus viaContextMode. Première erreurd'accoladeduscript d'analyse lead corrigée dansMCP, paspanneinfra.

- `outbox_runtime_tests::disabled_and_factory_degraded_noop_preserve_intentions_and_http_then_rebuild_drains` : **1PASS/0ignored/4filtered/exit0**,20.93s. Assertions0suspendu/2live etownerHTTPhealth sontatteintes avec lesoraclesdurables existants.
- `notification_created_routing_test::user_email_verified_routes_same_event_through_sentinel_and_real_authorization_after_publisher_recovery` : compilationOK puis **0PASS/1FAILED/0ignored/6filtered/exit101**,22.80s. Échec précis source506 : `failed.next_attempt_at > chrono::Utc::now()`. Pas d'échec setup/port0/compilation revendiqué ; ne pas réattribuer au SQSReceive/visibilité sanscause prouvée.

Le test possède un join SQL+samplerpassif dans la deadline25s (470–486), puis consulte le statutfailed(500–506). Besoin de distinguer invariantscheduling au momentdel'échec vs tempsd'observation ; **aucune modification d'oracle/backoff/deadline/calibration autorisée à ce stade**, aucun rerunaveugle. Le simple constat temporel ne prouve ni bugprod ni testflaky tantquediagnosticnonfait. Lespreuvesverticales précédentes restent archivées dans leurportée ; ce run final n'est pasPASSglobal.

## Cleanup recoupé en direct

Huitfixturescréées/tracées, sixretirées pargardes existantes, **zéropossédé running** vérifié viainventairefullID+inspectexact aprèsfin. DeuxSMTPExitCode2 arrêtés préservés :
- 9cdfda900a7173c8f140ff5dc1439a3dafea40bebc9eccdca1cab4706d5e7d09
- c0cbdb885ba930de3bffd7917f597957bd29988282f9fb27b1e6af3ec2b37c54

Leadappend reclassified **ask** pour cesdeuxidentités nonzero, sanssuppression. Quatrevolumesnouveauxopaquesaskpréservés (noms danssummary/proof), aucunvolume/imageeffacé. HuitconteneurspréexistantsTOUSmêmesIDs+états qu'avant, ycomprisnodeétranger40201running etseptanciensSMTPExited2. Aucununknownnewcontainer, postcount10.

Preuve `.tmp/http-final-it-4d513ab1f63f499b/lead-cleanup-verification.json` : created8/retired6/ownedRunning0/foreignBeforeUnchangedtrue/cargoRemaining0. Registreappend **`.worktrees/ti/.tmp/docker-resources-20261026.log`**. Desktop/WSL nonarrêtés globalement carnodepréexistant/workloadétranger ; RAMnonmesurée, pas«zéroRAM»inventé. Ressourcesactivespossédéesaucune ; retenusASKpasbailruntimeactif.

## Suite et gates

DiagnosticRO borné de l'assertiontemps à faire sursources/logs, pas correction de production outestparhypothèse. T5informée d'unPASSITpartiel etéchecverticalfinal, pas texteglobalvert. PublicationSDK/pinfinal/clôturemission suspendues ; C2/C3clôturescibléesT4restentvalables, pasdouvertureautomatiqueparcetautretest. Toute expérience suivante exige hypothèseprécise, oraclepréservé etnouveaupréflight/bail ; pasnouveauharness ouextensionQAimplicite.
