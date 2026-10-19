---
reviewer: security
written_at: 2026-10-18T15:47Z
branch: master
head_sha: 04172a46baa68a6ca9698b43d601ac52b3e87774
dirty: true
scope_slug: m6-proof-hardening
verdict: GO_WITH_COMMENTS
security_decision: REVIEW REQUIRED
---

# N6s — rapport final statique, fresh RO

Le préliminaire ci-dessous est conservé pour provenance et remplacé par les qualifications finales : **S-1 MEDIUM, confiance HIGH**, intégrité/causalité H4, correction et relecture avant utilisation ; **S-2 HARDENING**, sink panic API préexistant, confiance HIGH pour le sink mais aucune fuite actuelle démontrée. Aucun CRITICAL/HIGH ni escape production démontré. Verdict GO_WITH_COMMENTS, décision sécurité **REVIEW REQUIRED**, pas GO captures/live.

Snapshot diff-first0ca9b05..04172a4, surtout52b1198 ; contrat/QA vérifiés directement. Reviewer pool8 n'a exécuté ni Cargo/test/runtime ni mutation de code/ledger/briefing/INDEX/git. Dirty = userwork hors scope préservé.

## Rapport final — S-1 (remplace SECURITY-PRE-1)

- Catégorie other, cross-file ; actif = intégrité preuve H4/PH7, pas authz production. Frontière : contenu Endpoints d'un cluster partagé vers décision locale de preuve.
- Capacité attaquante limitée : update du seul Endpoints, pas Pod/NetworkPolicy ; pas capacité supposée du plugin ordinaire. Endpoints altéré/stale doit rester identique aux trois lectures ; réconciliation serveur concurrente peut faire échouer la tentative.
- Scénario complet : Podcanary vivantIP A, targetRef légitime name/UID/ns conservé, addresses.ip remplacé par B PodCIDR inutilisée. Positif forward Podparnom =>404réelA ; wgetB =>timeout attendu ; relecturesEndpoints falsifié stables =>H4possible même siA joignable sans isolation. Aucun essai live réalisé.
- Preuve : metadata.uid du **Pod réel** jamais lu/comparé ; IP jamais comparée à status.podIP/podIPs. Relecture du même Endpoints n'est pas source d'identité indépendante.
- Fix recommandé : lire Pod référencé et vérifier name/ns/UID/IP(s)/Ready et absence de suppression avant positif, puis avant/après wget ; toute ambiguïté bloqueH4. Garder forward Pod précis, pas Servicealéatoire.
- Régressions requises : Endpointsstable mais IP Poddifférente/UIDréeldifférent/PodnonReady ; pasde wget si discordance connue avantnégatif. Inclure nominaltrompeur EndpointsB+positifA+timeoutB rejeté, remplacementPod avecEndpointsinchangé et discordancepostwget. Pas de panne provoquée du partage.

## Rapport final — S-2 (remplace SECURITY-PRE-2)

- Catégorie secret-leak/HARDENING, local ; sources m6:245-249/282-286, ancien0ca9b05:224/261. Sinkde réflexion d'opérandes nonvalidés démontré, pas présence de vraicredential actuelle dansceschamps.
- Surface : stdout API Podimage/AdmissionRecorddigest vers Debug d'assert_eq puis paniclibtest archivé. Valeurs de ressources partagées altérées peuvent contenir sentinelle credential ; pas de lecture d'unsecretinterneprocess démontrée.
- Fix recommandé : conserver comparaisons avec assertionsbooléennes/messagesfixes sans Debug desopérandes, préserver FAILED. Validation par helper/assertion réellementutiliséparm6, valeurs sentinelles synthétiques image/CR ; diagnostic contient catégorieFAIL mais pas sentinelle. Ne pas fabriquer vrai token/clé pourtester.

## Deux passes et vérifications finales

PASSA : ensure valide var_os avantDocker/stat/probe ; aucune contournementtrouvé, erreurs sansécho. ArgvhostpasShell, URLwgetIPtypée et argumentpositionnelcité, port8080fixe et UIDparsée. Overridebinairelocal explicite pasentréeplugin/RCEdistante. KubeconfigdirPID+compteurexclusif, KUBECONFIGdédié, aucuneenvglobale/profilmuté ; Dropseulscheminspossédés, killbrutalpasgarantiecleanup. SSA sansforce et arrêtsansremédiation. Forwardlocalhost/no_proxy/HTTPtimeout/enfantpropre/Dropkillwait ; GNU/C/tries/markersstricts et erreursambiguësrefusées, S1subsiste. H1–3fermés/valeursobservées/H4tardif ; panicsprincipauxfermés, S2reste. Pasnouveauunsafe/deps/lock/CI/capacitéagent ; stagingsdepsdéjàexistantes.

PASSB/threatmodel : actifs preuveH4/transcripts/périmètre ; attaquants envhostile, writerEndpointslimité, état/outillageinattendu, pas contrôle magique kubectl/poste. Entrées OsStr/API/exec ; abusS1 traverse frontières en combinant contrôlesisolés corrects mais identitéindépendante manquante. Autorisations utilisateur/plugin/service et IDOR5707f9e horsdiff.

Sortiestransitives : H1 admit→ORAS/Cosign→cli_transport inspecté ; stdin/configprivée, stdout/stderr pipés nonhérités, erreursstatusstatiques, m6ignoreerreurbrute ; aucun printf vraie sortie/argv/env/auth/config trouvé succèsouéchec. H3tokenheaderjamaismarqueur/panic, signerHTTPsansdump, axum-test17.3.0error méthode/URLpasAuthorization. TransportCLI inchangé.

ArchiveN3 :22fichiers,21entréesMANIFEST bytes/SHA concordants ; logs/manifeste sansJWT/PEMprivé ni hits secretsfixtures exacts/base64 (ycompris user:password). Patch = sentinelleheaderPEM synthétique, pasclé. Contrôleciblé, pasgarantieuniverselle.

Limites/anti-goals : aucune preuve live nouvelle ; IT/E2E/captures/inventaires/cleanup PENDING. Pas authzauditglobal/full/protégés/m5/goldpath/campagnecredentials/transportCLI. DetteClippy27src n'est pas vulnérabilité déduite. S1exigerelectureaprèsfix, auditcapturesséparéaprèsexécution. Pasdequestionbloquantlecorrectiftest-side.

## Préliminaire conservé

## Re-review S-1/S-2 — 2026-10-18T16:21Z

## Audit final preuves live — 2026-10-18T17:20Z
- Même reviewer pool8, RO, HEADfd9c412 vérifié ; trois cargo.log/result.json/before.json, registre18, snapshots4, contrat/QA/ledger/protocole relus directement. Aucun Cargo/runtime/cleanup/mutation par reviewer.
- **PASS preuves exécutées N4/N5 et H1–H4**, **PASS confidentialité du corpus examiné sous publication sélective**. **FAIL toute affirmation de préservation intégrale des objets préexistants**. **INCONCLUSIVE / REVIEW REQUIRED sur conformité globale de portée GO**. S1/S2 code restent CLOSED ; aucun nouveau CRITICAL/HIGH démontré.
-9/1/1 tests, exit0, zéro failed/ignored/filtered, HEAD/targetE correspondants ; H1–H4 uniques dans cet ordre lignes30–33. H1 préfixélibtest : extraction au marqueur puis schéma intégral, jamais ancrage de ligne seul.
- CatalogueH1=H2, enveloppe/CRI distincts, hostname=Pod dérivé du catalogue, AdmissionRecord même identité ; VALID/Scheduled,403/200,hops0/1,404/timeoutGNUborné. Assertion runtime réelle, pas pcap/body/stderr bruts ni causalité exclusive Netpol/ServiceDNS/mesh.
- SHA256 cargo.log indépendants : t6 `100536bdb8e6257954db3308ca87681519347a96c81e3b860af8e8f488a3ccfe`, m1 `cfbaa348544937d36defaca8147898d1b7caa4baeb1cbe0124a1df30d50874c5`, m6 `8174156cb84b68b0fa106c0e396e0243a23ffeea0dc893a60d85d9b8b7caa138`.

### O-LIVE-1 — MEDIUM, REVIEW REQUIRED : autorité sur remplacements K8s
- **Disposition finale lead, après revue : ACCEPT explicite utilisateur, 2026-10-18.** Après explication des remplacements et risques même en local partagé : « Alors on accepte ce comportement. » Constat du reviewer inchangé, réserve close par arbitrage humain pour ce comportement du harness local ; aucune autorisation rétroactive supposée, aucun nouveau run/cleanup/usage production. Owner lead ; réouverture environnement distant/production, périmètre élargi, données à préserver ou concurrence incompatible. Cette disposition ne transforme pas le verdict sécurité initial en assertion de préservation intégrale.
- ConfianceHIGH remplacement/MEDIUM absenceGOpluslarge ; opérationnel cross-file, pas nouvelle vulnérabilité production attribuée au diff.
- Sources : m6_e2e_chain.rs:187–196 ; src/controller.rs:597–606,629–636 ; snapshots avant/après. Harness déduit noms du digest puis supprime Pod et AdmissionRecord préexistants, sans preuve de propriété ni précondition UID. Suppression Pod grace_period_seconds0 ; noms déterministes ne confèrent aucune autorité.
- AdmissionRecord UID7c9312fe…→0cfef8de… créé17:14:03Z ; Pod UID52881667…→3cc98b5a… créé17:14:05Z. Autres UID listés conservés, pas garantie de specs/labels inchangés ; aucun nom ajouté/disparu dans snapshots system6→6/plugins4→4.
- Impact : état antérieur remplacé, perturbation possible de ressources partagées ; aucune panne utilisateur réelle ni exploitation distante démontrée. GO visible «cycle des fixtures» couvre deux prédécesseurs Docker et nouveaux helpers, pas explicitement ces ressources K8s non possédées.
- **Arbitrage humain ou provenance GO requis avant clôture générale** ; acceptation présente ne réécrit pas l'autorisation au moment du run. Aucune nouvelle suppression/restauration/recréation automatique. Futur run partagé : accord précis des UID ou garde fail-closed/ownership vérifié dans lot approuvé ; régression future collision étrangère/UID changé avantDELETE, pas de PoC destructive ici.

### Confidentialité et publication
- Neuf fichiers ciblés scannés sandbox : aucun hit credentials fixtures connus exact/base64(user:password inclus), JWT/Bearer signé, PEM privée ou kubeconfigprivé ; aucune argv/env/auth/configprivée identifiée. Scan ciblé, pas certification universelle du dossier.
- Kubeconfigprivé temporaire contenait client-key-data/client-certificate-data ; aucune valeur publiée, aucun indice de fuite publique, aucune rotation alléguée. Exclu du manifeste14fichiers et **supprimé par le lead après libération des consommateurs RO** ; kubeconfig partagé inchangé.
- Publier/copier uniquement allow-list auditée logs/marqueurs/résultats/snapshots metadata/manifeste, jamais archive récursive runtime/.tmp. Originaux bruts conservés privés ; aucune copie assainie confondue avec original.
- Registre parsé141lignesdont119mission ; généalogie6fixtures avec predecessorID/priorstate,29helperscréésremoved,2fixturesfinalesexited etnodepréexistantrunning ;6volumes+2imagesask conservés. Ce constat archived n'était pas inspect17:20 ; auditinfra ponctuel distinct ci-après, aucun droit de purge.
- Deux passes : schémas/liens/échecs non masqués ; contrepied fauxcompteurH1/collisionnom/UIDstable≠specstable/kubeconfigprivédansarchiverécursive. AuditIT/E2E nonplusPENDING pourcesruns ; reste arbitrageO-LIVE-1 et clôture documentaire. DetteClippy/gapsdifférés non effacés.

## Re-review statique S1/S2 — provenance conservée

Même reviewer pool8/sessionindépendante, HEADfd9c412, diffdirect6bea5e3..fd9c412 (6fichiers408+/17-), contrat/QA/briefinginitialrelus, codegelé. **PASS statique ciblé : S-1 CLOSED, S-2 CLOSED ; aucun nouveau finding sécurité.** Aucun Cargo/runtime/mutation par reviewer ; cesfermetures ne remplacent ni IT/E2E ni auditdescapturesfutures.

### S-1 CLOSED — vérification indépendante à trois phases
- canary.rs:45/165/175-181/187-249 : PodAPI avantpositif et lorsdes2revalidations avant/aprèswget, lecturesdistinctesd'Endpoints. kind/name/ns/UID/ReadyTrue/nonDeletion/IPdanspodIPouPodIPs corrélés àcible ; API/parsing/mismatch=>catégoriefixe.
- RealCanary::pod chain.rs:443-453 fait getPodnommé JSON réel, pas Endpoints réinterprété ; nomdéjàvalidé, argv séparés, namespacefixe et kubeconfigdédié.
- Attaqueoriginale EndpointsstableB/PodréelA/positifA/timeoutB : BcomparéauPodA=>IPmismatch AVANTpositif/forward/wget/H4. Un writerEndpointsseul ne fournitplusles2sources.
- Régressionsdirectesproof:13-14/66-101 deuxjeuxindépendants ;473-498 casexact B/A (seulEndpoints ettimeoutB modifiés, PodAinchangéasserté) ;382-470matricekind/name/ns/UID/IP/Ready/deletion/API/malformed ;501-542remplacement/pertelectures2/3Endpointsstables ;545-563dualstack ;566-594étrangersrefuséstôt.
- Mutationmindset : supprimercomparaisonIP/UID/Ready/deletion, vérificationinitiale ourevalidation devient rouge parmatrice/trace/arrêt. RemplacerPodparEndpoints n'a niJSONPodindépendantniadaptateurréelconforme. Aucun H4 desvariantesrejetées.
- Résidu borné maintenu : lecturesAPI séquentielles, pas snapshotatomique ni viecontinue ; n'atteste ni causalitéexclusiveNetpol/DNS/Service/mesh. Pasnouveaufinding, pascertificationlive.

### S-2 CLOSED — deux vrais sinks neutralisés
- evidence.rs:3-25 et m6:246-250/284-288 : deuxcallsréelshelper, égalités post-triminchangées, aucune conversion d'échecensuccès. ApiObservation2catégoriesFAILstatiques ; assertbooléen sansformatopérandes, track_callersourcepascontenu ; pas nouveauDebugtransitif.
- proof:597-630 appelle mêmehelper, nominalégalOK puisinégalcapturérequis2catégories ; messageexact etabsencedeuxsentinellessynthétiques. Mutationretoursanscomparaison=>expect_errrouge ; assert_eq/échod'opérande=>message/catégorierouge. Catchpayloadsanshookglobal, lecturedupaniqueréelconfirmeaucunformatvaleurs.

### Deux passes et archive indépendante
PASSA : contrôlesPod3phases, arrêtfailclosed, égalitésfortesmessagesfermés, nouvelIOseulelecturePodargsdistincts. Aucunsecret/deps/unsafe/envglobale/cleanupnouveau ; forwardlocalhost etChildDropkillwaitinchangés.

PASSB : chaîneEndpoints-onlyIPB/PodA etremplacements/pertesaux2revalidations rejouésmentalement, toujoursrefussansH4. RéflexionS2 vérifiéeaux2callsites ; nepasinventerattaquantcontrôlantPodAPI/kubectl/poste(modèledemenacedifférent). Pasbypassnouveau.

Archive n3-r1 :13fichiers, **12entréesMANIFEST bytes+SHA256** concordantes, **6entréesSOURCES** identiquesracine. Parserlowercaseinitial0 corrigépourSHAuppercaseavantconclusion. Logsbruts vérifiés fixture10/0/0filtered, proof11/0/0filtered, rougeclippysemicolonconservé/run2Finished, m6list1/0benchmark ; pasrerun. ScanJWT/PEMaucunhit, passecretbrutrepublié, contrôletargetpasgarantieuniverselle. PasnouvellesortieORAS/Cosign/live ; transportinchangé nonréauditéglobalement.

### Remaining gates et staleness
IT/N5E2E, body404BusyBox/GNUréels, capturesH1–H4nonsecrètes, inventairespré/post etcleanup PENDING. PASSfixnecertifieaucunecapturefuture. Si6fichierschangentaprèsfd9c412, revaliderancres/fermeture. DetteClippy27proddifféréepasverte.

## Préliminaire historique (conservé)

- Reviewer indépendant pool-8, fresh RO, snapshot code `04172a4`, aucune mutation/runtime.
- Préliminaire persisté avant tout correctif ; qualifications finales ci-dessus font autorité. Ce rapport statique ne donne pas GO runtime.

## SECURITY-PRE-1 — MEDIUM — intégrité PH-7, Endpoints n'établit pas l'identité réseau du Pod

- Sources : fixtures/canary.rs:78-99/155-197 et fixtures/chain.rs:443-483.
- `ready_target` accepte name/namespace/UID/IP depuis Endpoints ; positif port-forward adresse le **Pod par nom**, négatif wget adresse l'**IP d'Endpoints** ; revalidate relit Endpoints seulement.
- Scénario : un writer limité à Endpoints conserve targetRef d'un canary vivant mais remplace addresses.ip par une IP PodCIDR inutilisée. Positif Pod vivant OK ; wget IP morte timeout ; Endpoints stable avant/après => marqueur H4 possible sans refus NetworkPolicy.
- Possibilité de réconciliation serveur/race limite le scénario ; aucun exploit live démontré, aucune attribution HIGH. Problème concret côté oracle que le lead a confirmé par lecture directe.
- Correction test-side attendue avant captures : lire **Pod API réelle** et vérifier namespace/name/UID, IP réelle correspondante et Ready, avant positif et pendant revalidations avant/après wget. Même corps réel/scripté ; oracle Endpoints stable mais IP/UID/PodReady réels discordants => erreur sans succès H4. Ne pas corriger en comparant à nouveau uniquement le même objet Endpoints.
- Statut : correction à préparer après rapport sécurité final, aucune modification encore ; pas de runtime/capture autorisé avec ce gap ouvert.

## SECURITY-PRE-2 — PH-6 — panic stdout API brute, qualification finale attendue

- Sources : apparatus_m6_e2e_chain.rs:246 et :283 : assert_eq(image_txt,cri_image), assert_eq(cr_digest,catalog).
- assert_eq publie automatiquement les opérandes en panic, donc la sortie API brute peut atteindre une future capture avant le schéma fermé. Préexistant à0ca9b05 ; toujours pertinent pour nouvelle mission de preuves sûres.
- Aucun secret ou fuite actuelle démontré. Correction envisagée : garder l'égalité mais assertion conditionnelle à message fixe sans opérandes ni debug ; ne pas affaiblir l'oracle ni réouvrir transportCLI/credentials hors scope.

## Constats préliminaires vérifiés par le reviewer

- Garde cible/argv/kubeconfig et sinks ORAS/Cosign examinés sans bypass concret à ce stade.
- Archive N3 :21entrées du MANIFEST vérifiées bytes/SHA256 ; logs sans hits des secrets fixtures exacts/base64/JWT/PEM recherchés. Patch contient une sentinelle PEM synthétique de test, pas une clé réelle démontrée.
- Ces recherches ne prouvent pas l'absence universelle de secrets ; captures runtime futures et audit final PENDING.
