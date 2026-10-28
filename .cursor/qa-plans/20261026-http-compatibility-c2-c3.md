# QA bornée — HTTP C-2/C-3 et oracles RunningApp

## Contrat et entrées

QA proportionnée de la mission approuvée, complément des plans C-1/E2. AI36357f633e40c825830c124596a8a6dd674de655 ; SDK local non publié9e3714948e7b815ee24e507d25a9c6d1e3dc911d ; gitlink index129dcc1 non changé. Contrat HTTP§2/§4/§6 et ADR0503Proposed. Paquets T4 finaux2006/2015 persistés avant corrections ; C-1 original clos ciblé, C-2/C-3 restent bloquants.

## Lot tests, propriétaire test-engineer

Tests/fixtures existants uniquement, dans source SDK isolée9e ; aucun code production, cible/manifeste/dépendance/feature nouveau ou baisse d'oracle autorisé par ce plan. Avant mutation, lead fournit sources/bundle et copies locales des briefs. Ne pas utiliser le SDK utilisateur frère ni celui du lead comme écrivain partagé.

1. **C-3** : vraie connexion H2 au serveur owned, observer SETTINGS_ENABLE_CONNECT_PROTOCOL puis Extended CONNECT réellement reçu par le router ; témoin legacyHTTP de comparaison si utile. Employer clients/dépendances/helpers existants. GET-H2 ne suffit pas ; ne pas prétendre preuve d'ownership d'upgrades arbitraires à partir du seul setting. Si aucun oracle réalisable sans nouveau feature/harness, rendre le besoin exact au lead avant extension. Conserver compilation réussie, observations et cleanup avant assertion rouge. Cible module #[cfg(test)] owned_server.rs ou testHTTP existant.
2. **C-2** : expected=config.actual_port() puis chemin standalone réellement adopté avec configport0 nonTLS ; health joignable sur expected, stop joint, ownerport0/local_addrs direct continue de fonctionner. Examiner aussi l'usage équivalent du host. Réutiliser fixture existante si nécessaire, sans nouvelle plateforme. Aucun helper test-only remplaçant le caller ; si extraire un petit helper de startup commun est nécessaire, le proposer au lead pour auteur production AVANT mutation. L'absence de seam ne justifie pas un test qui vérifie seulement une copie de la normalisation attendue.
3. **Cardinalités** : corriger les assertions déterministement périmées dans outbox_runtime_tests222/253 et notification_created_routing_test453–456 : parents métier0 suspendu/2 live. Ajouter ou maintenir une preuve explicite de HTTP possédé/joignable, en réutilisant health/start/stop joints de RunningApp ; ne pas simplement retirer les compteurs. Fixtures/tests autorisés, aucun arrêt ou assertion AC4/AC5/replay/FGA/recovery/timing amoindri.

## Exécutions et barrières

Commencer par les témoins réalisables sur la source non corrigée ; pas correction production par l'auteur tests. Rouge fonctionnel ≠ erreur compilation/environnement. Réutiliser les13témoins C-1 acquis ; pas rejeu global pour remplir ce plan.

Unique Cargo Windows24jobs, préflight et bail annoncés ; loopback/certs existants seulement au premier lot. **Pas de démarrage Docker/Kind/WSL ou IT à cette étape.** Les scénarios IT existants AC4/AC5 gênés par les anciens compteurs seront exécutés après intégration des corrections, préflight lifecycle et inventaire/bail fixtures convenus avec le lead. Ni nouvelle campagne verticale de calibration ni nouveau harness autorisé implicitement.

Rendu : sourceSHA/base/commit/tests-only, noms/commandes/codes/counts/cleanup, rouges immuables, gaps bornés. Si seam production ou déblocage feature nécessaire, préciser proposition/risque/oracle sans l'écrire. Commit tests et intégration réels avant auteur correctif isolé consommateur. Pas de publication/pin final. Même T4 reprendra C-2/C-3 sur delta intégré et preuves pertinentes ; pas de pipeline additionnel systématique.

## Correction de synchronisation de l'observateur C-3

DiagnosticRO pool12 sourcé dans briefing `20261026-http-c3-setting-observer-diagnostic.md` : l'atomic copié après pollConnection n'est pas causalisé par la réponseprobe. Ne pas le remplacer par un simple verdict routage200 ni baisser l'attendu settingtrue.

Après intégration du candidatproduction uniquement, auteurtests remplace ce sampling par demande mpsc/oneshot au driver qui lit le getter directement, après réceptionprobe ; timeout1s, aucune boucleuntiltrue/sleep/yieldartificiel. Valeurassertée = freshAfterProbe ; conserver CONNECTmethod/version/protocol/status/marker etcleanup. Première évaluation peut garder oldatomic/freshAfterCONNECT en télémétrie sans employer la valeur tardive pour masquerunfalseinitial.

Lead matérialise deux snapshotsproduction explicites (candidat corrigé et base3333sanssetting) avec **les mêmes tests corrigés** pour prouver la discrimination. Aucun prodeditparl'auteurtests ou nouveau harness/deps/feature. Un timeoutd'observation se distingue d'un settingfalse, pas rouge métier synthétique. Provenanceexacte/testsidentiques/logs immuables, pas de nouveau13replayglobal. Oracle SETTINGSstrict inchangé ; seul le mécanisme de mesure est réparé.
