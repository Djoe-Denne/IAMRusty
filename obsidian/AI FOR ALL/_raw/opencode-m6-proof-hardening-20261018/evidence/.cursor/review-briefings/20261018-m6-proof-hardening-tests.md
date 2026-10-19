---
written_at: 2026-10-18T15:44Z
reviewer: test-reviewer
scope: m6-proof-hardening
head_sha: 04172a4
dirty: true
verdict: PASS-static-live-PENDING
---

# N6b — revue tests statique m6 proof hardening

- Auteur indépendant : pool-7 / test-reviewer, fresh RO ; retour team_results.
- Snapshot : `0ca9b05..04172a4`, code N2 `5e6ae41`, N3 `52b1198`.
- **Verdict : PASS test-side**, 3 LOW + 1 INFO ; aucun BLOCKER/HIGH de sévérité. Les mentions HIGH/MEDIUM ci-dessous qualifient la confiance du reviewer, pas la sévérité.
- Preuve live m6/N5 **PENDING**, non certifiée. Aucun fichier/ledger/INDEX/code modifié par le reviewer, aucun test/runtime exécuté par lui.

## Findings — briefing persisté avant correction

### T-1 — LOW, confiance HIGH, LOCAL — borne de longueur non épinglée
- Sources : ensure.rs:91 et apparatus_m6_fixture_oracles.rs:382-398.
- Mutation qui échappe : retirer le contrôle longueur63 ; les scénarios actuels restent rouges avant appels, mais la catégorie « invalide » n'est plus épinglée pour un nom trop long.
- Correction proposée : nom `a` répété64 => « cible invalide », répété63 => « cible non autorisée », avec traces externes/FS vides.
- Validation ciblée : `cargo test --locked -p apparatus-operator --test apparatus_m6_fixture_oracles -- --test-threads=1`.
- Statut lead : à traiter après réception des autres revues, pas encore corrigé.

### T-2 — LOW, confiance MEDIUM, CROSS_FILE — câblage env réel statique seulement
- Source : fixtures/kind/mod.rs:51-55 ; les oracles appellent directement ensure_with.
- Mutation : var_os -> var().ok() ferait passer non-Unicode pour absence dans le wrapper réel ; les nouveaux oracles et le run N5 nominal ne la détecteraient pas.
- Le reviewer recommande GAP-accept (mutation env process interdite), ou micro-test serialisé de cluster_name si une décision explicite l'autorise. Ne pas muter l'environnement global spontanément.
- Statut lead : décision accept/defer bornée à persister après autres revues ; pas de preuve comportementale non-Unicode du wrapper réel revendiquée.

### T-3 — LOW, confiance HIGH, CROSS_FILE — identité Pod étranger non épinglée négativement
- Sources : fixtures/canary.rs:88-93 et apparatus_m6_proof_oracles.rs:110-189/231-297.
- Mutation : retirer starts_with("apparatus-system-canary-") échappe aux scénarios actuels ; nom/namespace étrangers pas testés.
- Correction proposée : endpoint IP/UID valides, targetRef.name="foreign-pod-abc" => Err « canary ready endpoints absent », trace ready/endpoints seulement. Ajouter namespace étranger si cette donnée est contrôlée dans le flux.
- Validation ciblée : `cargo test --locked -p apparatus-operator --test apparatus_m6_proof_oracles -- --test-threads=1`.
- Statut lead : à traiter après les autres revues, pas encore corrigé.

### T-4 — INFO, confiance HIGH, LOCAL — subnet référence non épinglé
- Source : tests/fixtures/kind/cluster.yaml:8 (ancres snapshot reviewer).
- Alignement10.244/16 non renforcé par test ; fichier référence sans consommateur code. Option include_str des deux YAML ou GAP-accept.
- Statut lead : décision bornée à persister ; aucun run cluster issu de cette référence.

## Registre couverture vérifiée par le reviewer

- Gates **exécutées** vérifiées dans logs : N2 staging6/6, N3 fixture9/9 et proof5/5, listingm6 exactement1 ; targetE visible chemin.exe. Présence unique tokio::test m6:50-52.
- PH-1 COVERED : manifests racine/reference-kv comme oracle indépendant ;21retraits, versions/options, features/net+sentinel, extras et verbatim ; parser borné fail-loud. Pas copie d'un producteur.
- PH-2/3 COVERED seam :15entrées refusées, trace ET file_reads vides ; non-Unicode explicitement invalide ; défaut/trim nominaux, argv/kubeconfig dédiés, cleanup possédé.
- PH-4/10 COVERED : panne de CHAQUEcommande spawn/exit ; interdits create/delete/install/force/calico.yaml/FELIX ; CNIabsence distincte de RBAC/API ; timeoutvirtuel180s/2s, polls=sleeps+1, borne180000<now<=182000, wall<5s, aucun apply.
- PH-5/9 COVERED client : un apply, erreur propagée, aucune suite/remédiation, objets simulés comparés avant/après (pas file de réponses), argv exact + mutation état si force.
- PH-8 COVERED : kubectl version --client avant chaque commande, kind version client-only, override fichier direct et absentFail après Docker seul.
- PH-16 COVERED : cluster autorisé absent via port,3commandes puis arrêt sans export ; aucun re-run réel bogus.
- PH-7 COVERED test-side : même observe_canary réel/scripté, ready/endpoints/positif404/revalidate/wget/classify/revalidate ;9scénarios précondition bloquent avant wget ;20diagnostics non probants rejetés dontHTTP200/404, refused, DNS/routage/outil/exec/transport ; execmarqueursstricts/GNU/code4/ligne timeout IPexacte/UID épinglé. Endpoint disparu/remplacé avant wget empêche wget. Aucun marqueur prématuré.
- PH-6 schéma COVERED / captures PENDING : h1/h2/h3 liste fermée ; négatifs Bearer/CRLF/AUTH/password/PEM ; appels réels m6 sur valeurs observées, hostname=Podname, hops réels ; panics M6FAIL clos ; H4 dans fluxcanary.
- PH-12/13 alignements config/docs revus, pas testés conformément au plan. PH-9serveur/PH-10branchesréelles/PH-11/PH-14/15 : limites accept/defer distinctes, pas absorbées par ce PASS.

## Qualité — six dimensions et limites

Lisibilité (PH/scénarios nommés), fiabilité (horloge virtuelle, zéroétatpartagé), diagnostic (catégoriesexactes/scénarios), design (corpspartagé, argv/trace/état), suspicionIA (dépendancesréelles et mutationstaging discriminante, pas théâtre), couverture (spawn/exit sur chaquecommande, borne readyz, stdoutwget exact) : aucune faiblesse bloquante signalée. Aucun test passe-sans-vérifier.

Limites : wrapper mod.rs→ensure_with seulement vérifié statiquement ; fallback WINDOWS_KIND forcé absent dans Script et vérifié par lecture, pas hermétique ; branche Exitreadyz de each_command non rejouée car timeout dédié la couvre. Captures H1-H4, inventaires et preuve live/auditfinal restent PENDING N5. Dette27Clippyprod hors périmètre de cette revue.

## Re-review N3-R1 — PASS, snapshot fd9c412

## Audit final preuves live — PASS, même reviewer pool7
- Lecture indépendante RO des trois runs réels : t6 9/9, m1 1/1, m6 1/1, zéro failed/ignored/filtered dans les logs ; exit0, HEADfd9c412, Windows E:, cleanup/witness sans erreur, baux libérés.
- H1–H4 chacun unique dans m6/cargo.log. H1 joint à la ligne libtest : extraction par sous-chaîne au marqueur puis fullmatch fermé, pas d'absence masquée ni log réécrit. Script archive_proofs.py vérifié ; manifeste **14/14 tailles et SHA256 concordants**, kubeconfig privé exclu.
- Liens catalogueH1=H2, trois digests distincts, nom Pod et hostnameH3 liés aux32premiershex ; AdmissionRecord lié au catalogue complet.403/200,hops0/1 cohérents avec les compteurs du test réel.
- H4 émis par observe_canary(RealCanary) après PodAPI indépendante x3, positif404/body attendu, exec GNU et timeout stricts, revalidations. C1 observations runtime fermées ; pas de certification DNS/dataplane/causalitéexclusiveNetworkPolicy ni de déterminisme futur.
- Snapshots bornés à la fenêtre : remplacement Podplugin et AdmissionRecord préexistants constaté ; autres identités listées stables, ce qui ne prouve pas leurs spécifications inchangées. Le caractère attendu du cycle de test n'établit pas sa permission ; O-LIVE-1 traité séparément par sécurité/lead.
- Ni re-run des21unités déjà revues, ni m5/gold path/quinze anciennes suites revendiqués. T2/T4/G-A/G-B acceptés inchangés, dette strictClippy production hors succès. Confidentialité/publication sélective soumise à décision sécurité/lead.

## Re-review statique N3-R1 — provenance conservée

Même reviewer/session pool7, diff6bea5e3..fd9c412, HEAD vérifié ; ROstrict respecté, aucune nouvelle exécution par le reviewer. **T1 CLOSED, T3 CLOSED, S1 COVERED, S2 COVERED**, aucun nouveau BLOCKER/HIGH ni mutant échappant identifié dans le diff ajouté. N5live PENDING, noncertifié. T2/T4acceptlead relus et nonrouverts faute risquenouveau.

### Preuves exécutées vérifiées par le reviewer
- Archive n3-r1 : fixture10/10 (6b6b1f6c...164de), proof11/11 (05b567...accaf), clippystrict2oracles Finished sanswarningtest-side, listingm6exact1 (1a1302...b984), patch737aea...12f4 ; SHA256 concordants auMANIFEST. Comptages : fixture9+T1=10, proof5+S1x4+T3+S2=11. Aucune suite régressée.

### Fermetures et mutation mindset
- **T1 CLOSED** : fixture_oracles.rs:437-453 ;63nonautorisé/64invalide+catégoriesexactes+trace/file_readsvide. Retirerborne ou changer>en>= rend un des deux cas rouge.
- **T3 CLOSED** : proof_oracles.rs:565-594 ; Podétranger et namespaceétranger dansEndpoints => Errreadytargetabsent, trace ready/endpoints seulement, wgetURLsvide. Retirerprefixguard fait appeler Podétranger (assertscript échoue) ; retirernamespaceguard faitpoursuivre contrairementàErrattendu.
- **S1 COVERED** : canary.rs:165/187-250, verify_live_pod avantpositif et danschaquerevalidate avant/aprèswget. PodAPI kind/name/ns/UID, deletionTimestampnull, ReadyTrue, IPprimaireOUmembrepodIPs ; RealCanary::pod chain.rs:443-453 getPodJSON sansignore-not-found=>APIéchecErr. Vraimêmeobserve_canary, pasfluxdupliqué.
- S1avantpositif :10fautes Pod ip/uid/name/ns/kind/notReady/noReady/terminating/APIunavailable/malformed, catégorieexacte ettrace ready/endpoints/pod, pasforward/wget. RetirerleschecksIP/UID/Ready/deletion fait échouer lescas correspondants.
- S1scénarioEndpointB/PodA : EndpointsstableIP.99 avec targetRefAconservé, positifA404 et timeoutBvalides scriptés ; refusIPmismatchavantforward/wget, PodAinchangéasserté. L'anciennecombinaison fausseH4 est bloquée par sourcePodindépendante.
- S1temporel : lecturesPod2avantwget et3aprèswget pouruid/ip/notReady/deletion/APIunavailable ; endpointsinchangésassertés, arrêtimmédiatlecturefautive, paswgetsi lecture2. Phase1déjàcouverte. Nominaldualstack podIPIPv6 etpodIPsIPv6+IPv4observée=>H4+3lectures+URLexacte ; mutation&&→|| ou suppressionbrancheIPssecondaire=>rouge.
- Tracesanciensscénarios misesàjour podx3 nominal etpositivepréconditions ;20diagnosticsnonprobants conservés. Pasd'attestationlive ; PodAPI/forward/wgetréels PENDING N5.
- **S2 COVERED** : evidence.rs:3-27 helperassert_api_equal booléen fermé, pasDebug ; deuxsitesréelsm6 PluginImage/AdmissionDigest :246-250/284-288 utilisenthelper (staticpresence, anciensassert_eqsupprimés), forceégalitéinchangée. Testproof:596-631 égalitéOK puiscatchpanic2variantes/2sentinelles, catégorieexacte etabsenceDEUXopérandes. Messagequiéchoitopérande ou comparaisoninversée=>rouge.

### Gaps bornés additionnels — pas de bloqueur ouvert
- **G-A INFO** : câblage des2callsitesm6 vershelper fermé vérifié statiquement seulement ; revertingcallsiteversassert_eq échappe autesthelper. Sous contrainte actuelle m6=1test, pasdepreuvecomportementale nonnominale ducallsite ; ne pas affirmer impossibilitéabsolue d'autresstrategies. LeadGAPaccept : ownerlead, impactmutationdecâblage noncaptéeparoracles, raisoncallsitesrelusdirectement/éviterextensionharness, réouverturetouterégression/refactoprotocolepanicm6 ; auditlivefuture requis.
- **G-B INFO** : podIPs malformées ignorées si podIPcorrespond ; robustesse seulement, corrélationIPréelle toujoursrequise. LeadGAPaccept : ownerlead, impactvalidationpasstrictetouschampsaccessoires, raisonAPIpoddeconfianceetIPeffectiveexactementcorrélée, réouvertureélargissementfrontièred'identité/trustmodelouconsommationdeceschamps. Aucunecorrectionattendueparreviewer.
- T2LOW/T4INFOacceptlead inchangés ; C1BusyBoxbody404/GNUlivePENDINGN5. Schéma/capturesH1-H4/inventaires/cleanup réels restent auditdistinct aprèsN5 ; detteClippy27src horspérimètre.
