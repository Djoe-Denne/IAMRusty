# Registre QA de progression — sécurité authentification

## Preuve réseau minimale — plan avant code et exécution

Contrat architecte v2 local-full-envoy-iptables-evidence.md lu intégralement et
ratifié explicitement par lead avant code. Première tranche : préflight fermé
outils/backend/modules uniquement, pas encore conntrack-L ou règles worker.
Allowlist helper/test existants. Les cinq oracles du contrat sont préinscrits ;
les trois décodeurs/corrélations ultérieurs restent NON implémentés/non exécutés.
Préflight à couvrir avant son run :
test_envoy_tuple_preflight_rejects_missing_tools_modules_and_foreign_nodes_before_dump ;
test_envoy_tuple_bounds_and_failures_never_echo_private_output.
Invalides=>zéroexec ; outils/modules/versions/refus=>STOP sansnetlink/fallback ;
ordreargvfermé, identités stables, capture réellement bornée et cleanupchildhandle.
Un succès du préflight ne vaut pas preuve NAT/drop ou droit de patch réseau.

Candidat27 reçu figé, deux fichiers seulement, aucunrunworker. Suite complètefake
44PASS17.597s/0skip dans son worktree. Après intégration strictementidentique
normaliséeCRLF/LF, seuls deuxnouveauxscénarioscanoniques rejoués :2PASS9.870s,
pas rerun des42anciens pour le seuljalon. Logs.tmp/auth-envoy-tuple-preflight-
{suite,canonical}-20261029.log. AST etdiffcheckPASS. Aucun accèsréel/Cargo/module.
Hashescanon helper5505e051... / testsfd054c7c... gelés pourrevue25 ciblée,
sourcecandidat91c7552a.../efd961b4... mêmecontenu aprèsnormalisation. Pasliveavant
revue/livraisonsourcequalifiée ; présenceoutillage etbudget30s réels nonétablis.

Revue25 reçue et lue, finals18737875.../187398711... : PASSstatiqueborné sur
candidat et canon, aucunfinding, tousancienssymboles ASTinchangés, hashesconformes.
Fakes/oracles examinés,44+2PASS attribuéslead sansrerunreviewer. Briefingsecurity-
envoy-tuple-tooling-preflight persisté avantlivraison. Aucune preuveNAT/drop.

Préflight livréf0a93a1/hookfmtPASS/Deliverypropre/SDK02e5 ; hashimporté91c7552a...
attesté. Runréeluniqueexit0/TOOLING_PREFLIGHT_ONLY20.303s : conntrack1.4.7,
iptables1.8.9nf_tables, deuxmodulesdéjàrésidents, identitésstables. Preuve6b7/
envoy-tuple-preflight-080e13b312ee49b8a72aebcdd4298f3f. Zérolecturetuple/règles/
routeexec/HTTPauth, aucunmodule/install/règlemutée. Preuveoutillage seule.

## Décodeurs et observation corrélée — avant exécution réseau

Tranche poursuivie directement par lead dans allowlist ratifiée. Trois oracles
déjà nommés au contrat/plan implémentés : queries_and_nat_mapping,
unique_candidate_and_stable_identity, drop_chain_ambiguous_deltas. Premier run
purefake3PASS0.001s, aucuneAPI/commande/réseau. Décodage conservateur exige pour
l'instant zone0 EXPLICITE ; absence non transformée en0, précision de source
snprintf1.0.9 (ATTR_ZONE conditionnel) remontée àarchitecte avant toute adaptation.
ChaîneCalico3.29.7source relue : deny MarkClear(MarkPass) si aucune policy passe,
pas d'évaluation de graphe/ipsets. Le compteur partagé ne qualifie jamais à lui
seul le paquet ; verdict renvoyé PARENT_CORRELATION_REQUIRED, pasPASS.

Nom supplémentaire inscrit avant code/run du protocole de capture :
test_envoy_tuple_observation_is_single_use_and_never_accepts_http_callbacks.
Baseline privée au seul objetgarde, pas de tuples historiques remis au caller ;
finish consomme la baseline avant capture, impossible de recycler/adopter après
échec, fenêtre60s de corrélation indépendante du plafond30s par snapshot. Cette
borne relie deux snapshots séparés, pas un tunnel ou processus maintenu vivant.
Exiger identité/règles stables, tuple nouveau unique, deltasmonotones ; sourceSNAT
différente deCP doit être signalée sansautopatch. PasHTTPcallback/URLlibre.

Capture branchée en deux appels sémantiques begin/finish, état privé single-use,
septargvdiagnosticfixes maxparsnapshot (4préflight+listing/route/chaîne). Nouveau
runcompletfake48PASS15.729s/0skip aprèsextensioncompatibletransport (stderr
conntrack ne tolère que résumé1.4.7 exact et nombrelignes<=8 ; chaîne64KiBfixe,
autresflux16KiB, défautpréflightinchangé). Pas d'APIréelle/HTTP exécuté.
Avant run du complémentbornes :
test_envoy_tuple_reads_have_fixed_bounds_and_closed_stderr_summaries.
Vérifier plafonds exacts/+1 parmode etrésuméstderrconnu/vide/incohérent/sentinelle,
aucune fuite etcleanupchildhandle ; lemodetransportprivé n'est pasargumentpublic.

Clarification architecte v2.1 lue/ratifiée explicitement AVANT code : zone absente
reste observed_zoneMISSING, requested_zone0 etSOURCE_FILTERED seulement si builder
fixe1.4.7/-w0/exit0 attestés ; zéro observé seulement littéral. Zone non0/noncanonique/
directionnelle/doublon ou provenance nonattestée=>STOP. Aucun nouveau probe de lib/
kernel : références ABI, versions installées non prétendues. Deux oracles nommés
avant run : test_envoy_tuple_zone_missing_preserves_source_filtered_provenance ;
test_envoy_tuple_zone_conflicts_and_unattested_filter_fail_closed.

Avant correction, finding lead persisté : projection baseline exposait un alias
vers l'état privé. Étendre le test single_use déjà nommé : mutation identité/deny
de sortie ne doit pas changer la comparaison ; finish échoué/expiré doit consommer
l'état avant toute nouvelle capture et refuser une relance. Pas de runréseau.

Régressionalias reproduite : test single_use exit1/1ERRORopaque0.198s surl'ancien
retour partagé. Après copie profonde desprojections, suitecomplète51PASS15.758s,
zéroskip ; logs.tmp/auth-envoy-tuple-alias-{red,green-suite}-20261029.log.
Complément transport bornes exactes/+1 etrésumés stderr1PASS0.018s ; zonev2.1
deuxoracles compris dans51, versionslib/kernel nonprétendues. Diffcheck0.
Photosgelées pour revueindépendantefresh : helper67b504874a4af9a370f29592999e71ef590944e9a4fc07a3719f40f59e8e91b5,
tests3f4234b9b616333391bf14077d6f743ce03af936c495bfeeb9f58d4c69b95ba6.
Pasdelistingconntrack/règle/routeexec/HTTP sousnouvellecapacité encore exécuté.

Enveloppeparent préparée .tmp/auth-envoy-tuple-observation-20261029.py, HEAD
placeholder+hashnormalisé7385b551... bloquent la Deliveryancienne avantprobe.
Nom avant run : test_tuple_observation_refuses_unqualified_source_before_runtime.
Éprouver hashdifférent=>zéroguard/probe etAST, sans aucuneAPI. Séquencefuture
préinscrite : Adminfermé ; baselinepassivefermée puisactive/countersstables ; UNE
requêteexacte TCPfresh/noauth/nojar/noproxy/noredirect ; finishpuisAdminfermé,
policiespossédéesfingerprintstable, déltasIAMexplicites. Pas de verdictautomatique.

Sourcegateenveloppe1PASS/0FAIL/0ERROR/0skip, ASTPASS ; aucunappelguard/probe
vérifiépartripwires. Log.tmp/auth-envoy-tuple-wrapper-source-unit-20261029.log.
Aucun nouveau runréseaupendant préparation/revue de cet artefact.

Revuefresh28 lue intégralement, finals1875a3e... et1875f420... : PASSsécuritéstatiqueborné
surhelper/tests67b/3f etwrapperae2a988d..., aucunfinding. Oraclespositifs/négatifs,
bornes/anti-alias/protocole examinés ; tests51+sourcegate1 rapportéslead, pasrerun.
Notation secondaire ambiguë /login0412 remplacée par routeexacte déjàratifiée
/iam/api/auth/gitlab/login, sourcecontratAccepted/profilgitlab/scénarioexistant,
sansmodifiercode/hashpostimage. Briefings+INDEXpersistés avantlivraison/capture.

Sourcecapturelivréeff12007 auhooknormalfmtPASS ; Deliverypropre/SDK02e5,
helperimporté7385b551... conformeàphotorevue normalisée. WrapperseuleHEADchangée,
inversionbyteexactreviewae2a988d... vérifiée, ASTPASS, nouveauSHA0133dc5924a8...
avantUNrunpréinscrit. AucuneimageRust modifiée etaucunevalidationréseauchangée
enPASS parcespréconditions. Lireprochainrésultatréel avanttoutecorrection.

Observationréelleuniqueexit0, UNEGETtimeout, nouveauCTunique traduitsource
10.244.230.64≠CPInternal172.19.0.5. ZoneresteMISSING/filtre0attesté. Passifdeny
22/1320stable puis+9/+540, mêmesidentités/règlesobservées, IAMupstream/TLS0delta,
AdminLIVEepoch0/cleanupmarkerabsent. Preuve6b7/envoy-tuple-observation-94998d80...
rootrelue, SHAobservation0ad1c639... . PENDING_LEAD_CAUSAL_REVIEW/POD_CHAIN_SHARED,
pasPASSautomatique. AnciencontratingressCPInternal/32nonratifiéetnonapplicable ;
ROpropriéténouvellesource+réarbitragearchitecte demandés avanttoutpatch. Aucun
nouveauGET/trace/install/certificat modifié pour contourner ce résultat.

## Ingress du tunnel CP mesuré — plan avant code/run

Contrat architecte v2 original55lignes lu +faitsRO21, ratifié explicitement parlead
avantcode ; canon local-full-envoy-nodeport-ingress.md. IPIP10.244.230.64 unique
CPNodeUIDexact, pasVXLAN/physiqueInternalIP ; liaisonCPID↔NodeUID déjà attestée
danscapture, pas providerIDsuffix ni preuveCRD indépendante. Jugementlead motivé
QUALIFIED_EQUIVALENT_LAB_ONLY consigné au contrat et causal-review.json privé,
sans réécrire les artefacts PENDING/per-packetnotestablished du collecteur.

Tests nommés AVANT code/run, nouveau fichier unitaire ciblé :
test_envoy_ingress_requires_measured_tunnel_of_exact_owned_cp ;
test_envoy_ingress_tunnel_mismatch_ambiguity_and_stale_proof_block_patch ;
test_envoy_ingress_patch_tests_uid_version_and_old_value_and_preserves_peers ;
test_envoy_ingress_noop_and_conditional_rollback_do_not_adopt_replacement.
MauvaisUID/tunnel/roster/preuve/aviscausal=>zéropatch ; T/32TCP10000 seul, JSONtest
UID/RV/ancien spec, peers/egress/defaultdeny conservés ; no-op sanspropriété,
rollbackseulerègle/conditional/replacement/concurrence/alias refusés. Fakes sans
runtime/Cargo ; revue du delta intégré avant parentlive et infra-verifier après.

Complément nommé AVANT correction/run :
test_envoy_ingress_parent_witness_requires_exact_303_and_rolls_back_302.
Le témoin303 exact de la route login déjà retenue doit être accepté ;302 ou autre
statut non contractuel doit être refusé et retirer seulement l'entrée possédée.
Le candidat initial exigeait302 et son fake reproduisait cette erreur ; aucun
effet réseau n'a été tenté. Les quatre oracles précédents restent conservés.

Correction livrée et intégrée par patch contrôlé après échec OS206 du hook auteur,
deux fichiers byte-identiques aux empreintes finales : runner809bc118..., tests
e771bcd9... . Suite canonique réellement exécutée :5PASS/0skip,2.250s (process2.454s),
exit0 ; le303 est accepté et302/301/307/308/200/500 refusés avec rollback possédé.
Pas de Cargo/runtime/API/HTTP. Préflight local complémentaire AVANT mutation :
appeler seulement _ingress_evidence sur les sept projections privées existantes,
sans garde live ni patch ; vérifier compatibilité du schéma réel et helperimporté.
Ce contrôle de lecture n'est ni nouvelle capture réseau ni preuve causale.

Préflight sur originaux : refus fermé ligne220, private_file=False pourles7
fichiers ; helper impose aussi readonly. Aucun live/patch. Préparation locale
annoncée : copie exacte allowlist7 dans nouveau dossier privé scellé, contrôle
SHA/bytes et private_file, puis même lecteur ; aucun original modifié ni gate
assoupli. Son résultat sera distinct des tests fake et de la preuve réseau.

Résultat : copie sept fichiers scellés, bytes/SHA conservés et7private_file=True.
Lecteur canon refuse ensuite le helperCRLFraw67b504... ; copieLF livrée attendue
7385... . Après livraison95b912e et checkout propre qualifié, lecteur réel de
ces sept copies PASS avec helper effectivement importé depuis ce checkout,
observationSHA0ad1c639... inchangé. Aucun guard live/API ou patch exécuté.
Hook canon normal de95b912e : cargo fmt --all -- --check PASS (commitexit0),
unique Cargo et jobs24/E: conservés ; échec OS206 auteur demeure historique exact.

Revue indépendante finale : REVIEW REQUIRED avant live. Deux oracles ajoutés
AVANT correction/run :
- test_envoy_ingress_does_not_adopt_equal_spec_update_between_ack_and_first_read ;
- test_envoy_ingress_accepts_native_admin_owner_chain_tuple_and_json_roundtrip.
Le premier lie la propriété à l'UID/RV/spec de la réponse du patch, jamais au GET
ultérieur ; concurrence entre ACK et premier GET ⇒ STOP sans retrait étranger.
Le second accepte tuple natif et JSON roundtrip identiques, mais refuse tout
changement UID/incarnation. Les cinq oracles précédents sont conservés.
Extension éventuelle du garde -o pour patch : contrat architecte proposé puis
ratifié AVANT implémentation ; aucune permission générale de sortie de Secrets.

Défaut Admin reproduit AVANT fix : nouveau test sélectionné seul, exit1/1ERROR,
refus à _ingress_admin_pair:196 pour tuple natif vs listeJSON. Correction locale
de la seule comparaison via _ingress_json, références Serena vérifiées vers
_ingress_evidence et finish ; wrapper.tmp couvert par lecture ciblée.
Suite postimage6PASS/0skip, exit0,2.345s ; tuple/listes et roundtrip acceptés,
podUID/containerID/restartCount/imageID/ownerUID et booléen à la place du compteur
refusés ; finish/rollback fake vérifiés. S-1 reste NON corrigé, donc aucun live.

S-1 reproduit avant correction : oracle nommé exécuté seul, exit1/1FAIL/0.068s,
« ValueError not raised » après mise à jour égale du spec mais RV13 injectée
entre ACK et premier GET. Ce ROUGE est comportemental, pas environnemental.
Fake préparé pour renvoyer la vraie postimage JSON si la sortie -ojson fermée
est demandée ; l'oracle exige que le GET concurrent ait effectivement eu lieu,
pour éviter un faux vert par simple erreur de parse de l'ACK. Aucun helper
garde modifié avant amendement architecte ; S-1 reste bloquant pour ce live.

Gates complémentaires à nommer AVANT le futur changement du garde (implémentation
toujours conditionnée à l'amendement ratifié) :
- test_envoy_ingress_patch_json_output_is_closed_before_probe ;
- test_envoy_ingress_patch_ack_identity_and_version_are_authoritative.
Sortie JSON de la seule NPallow-envoy/nsfull par argv fermé ; Secret/autreobjet/
namespace, autreformat, duplications/overrides ⇒ zéro commande avant ancrage.
ACK valide opposé aux relectures ; ACK absent/perdu ou UID/RV/spec divergents ⇒
STOP sans adoption ni retrait étranger. Provenance producteur/helpermutation
distincte après changement de grammar : contrat d'architecture doit fournir
liaison fermée vérifiable sans altérer les sept projections d'origine.

Complément de provenance proposé par architecte : allowlist versionnée de paires
exactes dans le runner, clé(captureHead, captureHelperRAWSHA, observationSHA) vers
helperMutationRAWSHA ; cas identique inchangé, inconnue/table vide ⇒ STOP.
Oracle nommé AVANT code/run :
test_envoy_ingress_helper_compatibility_requires_exact_qualified_code_pair.
Exiger les trois éléments source et l'autorité réellement importée ; mismatch de
chaque champ, paire absente ou autre hashcourant ⇒ refus, pas booléen/inputlibre.
Qualification préalable : archive RAW producteur vérifiée7385..., égale au blob
gitff12007 et au fichier delivery ; copie privée readonly faite avant édition,
aucun helper exécuté depuis cette archive. L'amendement écrit reste à ratifier.

Amendement final architecte82lignes lu et ratifié explicitement par lead AVANT
extension ; canon local-full-envoy-ingress-patch-ack.md, quatre fichiers fermés.
Noms normatifs supplémentaires inscrits AVANT implémentation/run (recouvrant
les alias de plan ci-dessus, pas une nouvelle campagne distincte) :
- test_envoy_ingress_patch_ack_is_frozen_and_lost_ack_never_grants_rollback ;
- test_envoy_ingress_rollback_uses_ack_rv_and_does_not_adopt_postrollback_get ;
- test_envoy_patch_json_ack_grammar_is_exact_before_any_probe ;
- test_envoy_ingress_qualified_capture_helper_pair_preserves_immutable_evidence ;
- test_envoy_ingress_unknown_helper_pair_or_rewritten_historical_summary_is_rejected ;
- test_envoy_ingress_mutation_authority_cannot_change_after_ack.
Gate source hors runtime : AST module complet sauf seul corpsparse identique au
RAW producteur et refus de cinq mutants collecteur/Admin/constante/import/lifecycle.
Paire de hash à approuver APRÈS codefigé/tests/revue ; tableinactive entre-temps.

Correctif ACK/provenance implémenté dans les quatre fichiers autorisés, table
versionnée encore VIDE donc aucune compatibilité historique active. Gate AST sur
RAW producteur conservé : PASS, seuls changements dans corpsparse ; cinq mutants
collecteur/Admin/constante/import/lifecycle refusés. Hashmutation provisoire du
checkout principal2672c43f... : NE PAS le confondre avec les octets LF qui seront
réellement livrés ; qualification finale doit viser le checkout de livraison.
Suites réellement exécutées : guard52PASS/0skip15.643s et ingress12PASS/0skip4.572s,
exit0 tous deux ; logs `.tmp/auth-envoy-ingress-correction-4dba24b0246c4757a4360fa58a7795d6/`.
S-1 oracle auparavant rouge maintenant vert ; perte/mauvaisACK, concurrence avant
GET et au rollback, autorité figée, no-retry incertain, manifesthistorique strict
et identité native couverts. Aucun résultat live ni activation de paire déduit.

Livraison98bdd75360e3f3d7478eb0586c5d0a5c6f9816d1 via hook normal fmtPASS,
checkoutdelivery propre et SDK02e5 inchangé. Helper effectivement livré RAWLF
9b0da96b7e0db6ace93628d4d55f059d997231384c44737ff2a39ea4f1a64605,
différent du RAWrootCRLF2672... mais ASTentiers4fichiers root/delivery identiques.
Gate répété sur VRAI RAWdelivery :PASS et cinq mutants horscorpsparse refusés.
Contrôles d'ABI/provenance du checkout réellement utilisé (pas rerun de tout) :
grammarACK1PASS.024s et helper_pair2PASS.368s ; logs delivered-*.log même dossier.
Table exacte toujours VIDE ; proposed-code-pair.json privé/scellé NONAPPROUVÉ,
relecture indépendante même reviewer demandée sur code figé et paire proposée,
avant ratification des valeurs/activation. Aucun live/API/Secret/runtime.

Préparation indépendante de la corrélation navigateur, NON exécutée : réutiliser
oauth_forward existant (ServiceUID possédé, loopback18081, handle du seul child,
CTRL_BREAK Windows/attente15s/marker conservé si cleanup bloqué). Admin refuse
le marqueur OAuth : snapshotavant AVANT forward, puis providerobservations,
scénario inchangé, providerobservationsaprès, fermeture forward, snapshotAdminaprès.
Pas de tunnel Admin pendant le navigateur ni nouveau port/transport/proxy.

Oracle de comptage conservateur prévu avant run, pas résultat observé : le
scénario existant effectue deux autorisations IdP (login+relink), ou trois si une
registration202 exige le nouveau login200. Les quatre compteurs IdP de succès
authorize/token/profile/pkce_validated doivent être cohérents pour ce même runId ;
ils ne comptent PAS toutes les tentatives invalides. Positifs IAM attendus4,
ou7 avec préparation de registration. Comparer les compteurs upstream_rq_total
IAM explicites avant/après, même Pod/incarnation/epoch, fenêtre exclusive sans
autre client de mission. Égalité exacte attendue + positifs navigateur complets
+ matchers effectivement chargés + trois refus sansBearer : candidat à la preuve
de non-dispatch N1–N3, pas les seuls compteurs vendor. Hausse inattendue, compteur
MISSING/reset ou dérive ⇒ INCONCLUSIVE, aucun zéro inventé ou PASS partiel.
Ce plan reste à confronter aux observations réelles et ne remplace aucune IT,
preuve Envoy/SAN ou acceptation de la matrice complète.

Complément architecte94lignes reçu après98bdd75 : format constant EXACT5tuples
et réservation du captureHead explicités, ratifiés AVANT alignement en code.
Nouveau test nommé AVANT implémentation/run :
test_envoy_ingress_reserved_capture_head_cannot_reclassify_altered_observation.
Capture sous head réservé, observation modifiée et summary réécrit au helper
courant doit refuser AVANT toute commande, même si le SHAobservation diffère.
L'oracle unknown_helper_pair sera étendu aux formes malformées/ordre/duplicats et
SHA de qualification exact, dans le même fichier existant. Helper/grammar9b0 et
preuves immuables inchangés ; recheck en cours de98bdd75 reste borné à sa source.

Recheck indépendant reçu/persisté : S-1, grammarACK et Admin PASS statique sur98,
aucun nouveau finding ; complément précise aucune activation sous nouveau format
avant alignement/recheck. Nouveau cas headréservé reproduit ROUGE/exit1/1FAIL.087s
(ValueError not raised) AVANT fix : observation altérée/review liée/summary au
helpercourant pouvait reprendre l'égalité. Puis constante exacte à5tuples,
manifest7ordonné, SHAdu reçu complet, observation OU head réservés prioritairement,
projection canonique figée. Table encore VIDE. Suite13PASS/0skip4.638s,exit0 ; log
provenance-shape-tests.log même campagne4dba... . Formes incomplètes/mutables,
ordre/duplicats/préfixes/bool/SHAmismatch et cas headréservé refusés avant commande.
Seuls runner/test ont changé ; helper9b0 et52tests du garde précédents réutilisés.

## Plafond server_info v2 — plan avant code/run

Amendement architecte v2 lu et explicitement ratifié : plafondFIXE131072B,
statistiques65536B, GET5s/tunnel30s inchangés. Aucun nouveau live avant test/revue.
Deux tests nommés avant code/run :
test_envoy_admin_server_info_v2_fixed_limit_and_opaque_projection ;
test_envoy_admin_server_info_v2_overflow_discards_and_closes_without_retry.
JSONvalide >16Kio/exact131072, champopaque nonrendu, transports connus/inconnus/
chunked ;131073 etplus, <=131073lusetjetés sansparse/retry, cleanup completfake.
Préserver stats exact65536/+1, délais et toutes les matrices déjà vertes.
Baseline surconstant16Kio puispostimage128Kio et suite complète. Pas de promesse
de PASS réel ; dépassement128Kio=>INCONCLUSIVE/STOP, aucune hausse itérative.

Baseline2tests avant changement de constante : exit1,6failures/9errors de sous-cas,
0.344s, pas incident d'environnement. Après UNIQUEconstante server_info131072,
suite42PASS5.567s, zéro skipped, toutes anciennes matrices conservées. Logs
.tmp/auth-envoy-admin-v2-{red,green}-20261029.log. Aucun accès réseau/runtime.
HelpercanonSHA7a56772b... ettests724aa67e... gelés pour revue25 ciblée ; hashhelper
normalisé99a62f950e0c6125a62eb8b8709606b0e52294fd86419948235d57f70ef9498e.
Enveloppes attendent désormais cette source v2 et refusent la sourceDeliveryv1 ;
aucun nouveau live avant revue, commit normal, avance propre et attestation.

Revue sécurité v2 récupérée et lue : final1870220f..., PASSstatiqueborné,
hashes7a/724conformes, aucunfinding ; oracles JSONtailles/opaque/overflow/cleanup
examinés. Briefing20261029-security-envoy-admin-v2-7f075a2.md. Aucun rerun pour
ce jalon ; livraison/attestation encore requises avant l'unique lecture réelle.

Adminv2 livré7c95d6e auhooknormalfmtPASS ; deliverypropre/SDK02e5,
helperimportéRAW99a62f95conforme. UNIQUElecturev2réelle exit0complete, EnvoyLIVE/
epoch0/Podcec77212stable, childfermé/marqueurabsent. ProjectionglobaleINCONCLUSIVE
car upstream_rq_5xxMISSING ; autrescompteursIAM/ext_authz int0 explicites. Preuve
session6b7/envoy-admin-snapshot-c95ceed1888041e1827d6cd8ab80647c. Pasdepreuve
NAT/drop/SAN/browser, pasdeGETauth/cert/policymuté. Aucunehausserequise ouprévue.

## Prérequis source du scénario SAN — plan avant dernier delta

Revue25 finale : sécurité statique bornée PASS sur trois hashes, sans autorisation
d'usage live de l'ancienne source importée. Le delivery actuel ne définit pas
encore la nouvelle API. Avant live : livrer la même postimage qualifiée dans ce
checkout et attester son hash, pas HEAD seul.
Unité nommée avant code/run :
test_san_refuses_unqualified_helper_before_cluster_probe. Hash normalisé CRLF/LF
et présence API doivent refuser une source différente AVANT construction/probes
d'ancrage et avant GET ou mutation. Pas de nouveau runtime ou harness.

Résultat unité source : 1PASS/0FAIL/0ERROR/0skip, zéroprobe vérifié, log.tmp/
auth-envoy-san-source-gate-unit-20261029.log. Attestation attendue du helper
normalisé CRLF/LF : eeee3b6aa0d0d2906d6b7662892eced5b897838f04b94a3bae61915ccc60eb11.
La livraison dans le checkout importé doit encore être constatée avant usage réel.

Prérequis source fermé avantpremierlive : commit7f075a2, hookfmt normalPASS,
deliverypropre/SDK02e5 ; helperRAW réellementimportéeeee3b6a... etAPIprésente.
Photorevue8af... normaliséestrictementégale àce fichier (eolGitLF), pasautocode.
PremierappelAdminnommé : closed_envoy_admin_snapshot_live, UNElecturefermée,
aucune requêteOAuth ni mutationcert/policy ; état réel, compteurs etcleanup àconstater.

## Envoy RE2 — correction compatible, plan avant code/run

Cause live confirmée et briefing persisté : regex publique ligne97 excède la
limite de compilation RE2 dans Envoy1.31.2. Ne pas relever cette limite.
Régression nommée AVANT code/run :
`test_public_post_auth_routes_use_six_exact_matches_without_widening_bypass`
dans test_oauth_browser_alignment_source.py. Vérifier les six paths POST exacts
équivalents à l'ancienne regex, cluster IAM et disabled ext-authz inchangés, position
avant le fallback /iam/, absence de cette regex, limites runtime non relevées.
Témoins négatifs : autres méthodes, suffixes, slash/casse/préfixes voisins ne
profitent pas de ce bypass ; les autres routes et transports restent identiques.
Baseline ancienne config rouge, postimage verte et suite d'alignement/rendus offline.
La preuve live ultérieure est le démarrage Envoy, pas un calcul RE2 simulé.
Oracles SAN/browser distincts toujours ouverts. Aucun nouveau framework/container
de validation, aucune modification IAM/Manifesto/SDK, pas de Secret lu.

Envoy candidat24 : baseline sur YAML exact ef4bc1f par substitution de lecture RO,
1FAIL/0ERROR attendu ; candidat1PASS puis suite12PASS2.564s zéro skip. Le test est
source-level, pas moteur Envoy. Comparaison structurelle indépendante du lead :
ancien matcher index4 remplacé par six copies à paths exacts ; tous autres champs
du bootstrap et autres documents YAML strictement identiques. Copies canon des
deux postimages exactes en octets, SHA YAML1631a45a.../testa5d6555f..., diffcheck0.
Recheck sécurité du seul delta Envoy demandé au même reviewer fresh Manifesto.

## Campagne d'intégration Manifesto — sélection finale avant exécution

Choix proportionné : réutiliser UNE campagne native du runner existant
`ops/tests/it-native.ps1 -Suite manifesto`, pas de nouveau sélecteur/harness.
Le composition root et les trois configurations changent pour toutes les fixtures
de ce service ; cette campagne couvre leurs consommateurs partagés et inclut
P3T4/P3T5 réels. Ni IAM/Hive déjà prouvés ni tout workspace ne sont rejoués.
Le runner exécute réellement cargo test -p manifesto-service --tests (sans flag
--locked) ; ne pas annoncer à tort la commande ciblée --locked de référence.
Cargo.lock, runner, config Cargo et neuf postimages intégrées gelés par SHA avant /
après ; check --locked consommateurs déjà vert, aucune dépendance changée.

Préflight : unCargo Windows, jobs24/E:, SDK02e5, Engine exact, contexte desktop-linux
sans override. Descriptors Manifesto relus : PG/FGA réels, migrations dans leur PG
jetable, SQS uniquement binaire dédié opt-in ; aucun Kind ou contexte protégé.
Lifecycle runner inchangé : inventaire préalable, témoin Engine contemporain,
registres complets, corrélation avec nouveaux ledgers d'incarnation, cleanup finally
seulement IDs possédés ; retrait normal des Exited(0) attestés, nonzero conservés,
aucun prune/force/volume. Les trois nœuds du labo et autres ressources préexistantes
ne deviennent pas fixtures de la campagne. Après run : compteurs exacts, failures /
ignored / filtered / retours anticipés à distinguer, inventaire et fuites à vérifier.
Pas de livraison Linux ni mutation du laboratoire pendant cette campagne.

Résultat campagne complète : arrêt compilation Cargo101, zéro test exécuté.
Premier défaut établi LNK1102 mémoire insuffisante (linkerHostx64), puis E0463 ;
pas de défaut fonctionnel prouvé. Aucun os206, zérofixturecréée, inventaire23/23
IDs+états inchangé, zéro owned-running/new-unowned/cleanupissue. Source gelée
inchangée, y compris lockfile. Preuves auth-manifesto-it-ce7d6d2112ce429aa42e84e509203079
et itfixtures-it-0c33ce196e984a8f964ffacdd70b39fb. Mesure mémoire après échec
10.85GiB disponibles/61.56, engagement96.69/125.56, pas preuve du pic.

Reprise décidée : conserver24jobs et uniqueCargo, sélectionner un binaire à la
fois (P3T4 puisP3T5) pour limiter les linkers concurrents, pasrelancer44binaires.
Extension compatible et fermée du runner existant, pas nouvellefixture/runtime :
option TestBinary autorise seulement apparatus_p3_t4_gateway ou apparatus_p3_t5_consent
avec Suite=manifesto unique ; absent conserve exactement le chemin historique.
Validation avant touteprobe, branche ciblée cargo test --locked -p manifesto-service
--test <binaire>, sans -j/--jobs ou modification des threads. Lifecycle inchangé.
Nouveau contrôle AVANT code/run : test-it-native-selection.ps1, pur PowerShell/AST,
positifs sélection/default, refus autres suites/noms/injections/ambiguïtés et
exercice de la branche réelle avec Cargo fake (aucunexe/API). Contrôles existants
threads/stop/failclosed réutilisés ; pas de nouvelle infrastructure.

Sélecteur implémenté dans le runner existant, défaut inchangé ; contrôle AST neuf
PASS (2 positifs, 3 défauts, 10 refus et trois branches Cargo fake), contrôles
threads/stop/stop-failclosed existants tousPASS. Aucune commande Cargo/Docker réelle
par ces contrôles ; lifecycle/registre/cleanup inchangés. Prochaine invocation
réelle sélectionnée : it-native.ps1 -Suite manifesto -TestBinary apparatus_p3_t4_gateway,
puis P3T5 seulement après résultat/diagnostic. Cargo24jobs conservé ; un binaire à
lier dans chaque commande, pas de promesse que cette mesure résout toute pression.

Sélection P3T4 réelle réussie : 8PASS/0FAIL/0ignored/0filtered, exécution9.04s
(compilation en plus), Cargo24jobs conservé ; source/lock/runner inchangés pendant
run. PG/FGA réels, fixture2créées/2retirées, pic2, inventaire23→23 mêmespréexistants,
zéroownedrunning/newunowned/cleanupissue. Preuves auth-manifesto-it-64ad164e...
et itfixtures-it-3dfb6550d23b4642ae5f3721fb31b8fe. Les32lignes "skipping" sont
toutes des notices PostgreSQL "already exists, skipping", pas des tests omis ;
aucun indicateur de test skipped trouvé. La suite complète initiale reste non
exécutée, ne pas la convertir en PASS. P3T5 est maintenant le prochain binaire.

P3T5 réelle réussie : 7PASS/0FAIL/0ignored/0filtered,7.69s exécution ; source gelée
inchangée. Deuxfixturescréées/2retirées, pic2, inventaire23→23 préexistantsconservés,
zérofuite/newunowned/cleanupissue. Preuves auth-manifesto-it-69e648d1... et
itfixtures-it-4dd192585843467fbdc1967229ca891b. Les24mentionsskipping sont toutes
noticesPGalreadyexists, pas omission de tests. Total ciblé15ITPASS, pasfullsuitePASS.
Les24jobs et threads existants sont restés inchangés. Revue indépendante25 Envoy
PASSstatique borné sur SHA exacts ; recheck14 sélecteurITPASSstatique, lifecycle
inchangé, aucune exécutionparreviewers. Briefings persistés avantlivraison.

Livraison Linux636b438/SDK02e5 : build artefact exit0/47.41s puis Manifesto
exit0/9.40s ; IDsfe567047... et19ccec47... inspectés Linux/amd64, labels exacts,
runtime appuser. Registre à création, Engine23/23 et latest préservés. Ce n'est
pas un résultat de test, ni une preuve de déploiement/readiness. Préflight du
wrapper Envoy : AST PASS, marqueur antérieur refusé AVANT patch, SDK gelé.

Reprises live ciblées : Envoy EXIT0/Ready Podcec77212..., Manifesto EXIT0/Ready
Pod8624d84e... avec nouvelleimage19ccec47..., UIDDeployment/CMprécédents conservés.
Résumés sous bail6b7.../envoy-routes-recovery-cb1350af... et manifesto-recovery-
45fb36fb... ; revueinfra14 en cours. Cela valide startup, pas trajetauth complet.

Oracle SAN : SEULE préconditionHEAD changée636b438, inversion versancienHEAD
restitueSHArevue87e6680f... exactement ; hashactuel2bfbff6c6ce3e99b534542a9a21c108739619f44dd43f120386027d5a72a753a.
Préflight routeexacteGET18080 sanssuivreredirect a échoué par TimeoutError15s ;
aucunpatchcertificat ni lecturecompteur après cetéchec. Verdict INCONCLUSIVE,
pas échec du contrôleSAN. Preuve.tmp/auth-envoy-san-preflight-20261029.json.
Diagnosticréseau21 même session réactivé pour localiser hop avant nouveauGET ;
pas répétitionaveugle ni changement de transport/config/timeouts.

## Lecture spécialisée Admin Envoy — plan avant code/run

Contrat architecte local-full-envoy-admin-readonly.md v1 lu et ratifié par lead,
avec risque explicite d'exposition Admin complète aux processus locaux pendant
le bref tunnel loopback sur le poste de confiance ; jamais pendant navigationOAuth.
Allowlist code : local_full_target.py et test_local_full_target.py. Rien d'autre.
Tests nommés avant code/run :
- test_envoy_admin_rejects_foreign_target_and_free_requests_before_probe
- test_envoy_admin_requires_owned_chain_and_stable_container_incarnation
- test_envoy_admin_exact_child_mapping_and_closed_gets
- test_envoy_admin_limits_and_missing_stats_are_not_zero
- test_envoy_admin_stale_marker_and_cleanup_use_only_owned_handle
Suite fake existante target à exécuter après gel du candidat ; pas de réseau,
fixture ou Cargo par ces tests. Lire ses scénarios négatifs réels, pas les seuls noms.
Revue de sécurité proportionnée du nouveau delta avant code intégré/runtime.
Compteurs manquants restent explicites ; pas de PASS réseau/SAN par mocks.

Régression avant usage réel localisée par source client-go0.36.1:443 : kubectl
imprime le port LOCAL dans « Handling connection for... », pas le distant9901.
Étendre le test déjà nommé exact_child_mapping_and_closed_gets AVANT run : positif
avec deux lignes locales49152, refus9901/autreport ; baselineancienparseur puisfix.
Ce refus fermé empêche l'usage réel, pas défaut fail-open démontré. Briefing
20261029-admin-kubectl-local-port-regression.md persisté avant correction.

Régression reproduite sans runtime : unittest ciblé exact_child_mapping_and_closed_gets
exit1/1ERRORAnchorError0.028s sur le parseur initial. Après comparaison au portLOCAL
attesté et deux refus de ports étrangers, suite40PASS5.544s, zéro skipped. Ce test
ajoute le format client réel absent des fakes initiaux, pas une répétition aveugle.
SHA finaux helper8af7e8b6... / tests0672ec44..., communiqués au reviewer25 avantlive.

Candidat26 reçu figé, deux fichiers seulement : suite fake40PASS/0skip,6.447s.
Intégration canon par nouveaux symboles et imports, aucun ancien corps modifié,
postimages strictement identiques après normalisationCRLF/LF. Tests canoniques
40PASS/0skip,6.967s (le second run valide aussi les frontières de chemin Windows
du checkout intégré, pas un rerun de campagne réseau). Diffcheck0. Logs
.tmp/auth-envoy-admin-{fake,canonical}-suite-20261029.log. Aucun Cargo, fixture,
Adminlive ou navigateur par ces tests. Revue indépendante25 en cours sur le même
contenu, hashes canoniques communiqués ; attente avant gel/livraison/live.

Adaptation oracleSAN attribuée explicitement au lead, fichier unique
.tmp/auth-envoy-same-ca-san-20261029.py : remplacer curl par snapshot sémantique
attesté sans changer le trafic, les certificats ou les oracles good/wrong/good.
Avant exécution, deux unités nommées sans réseau :
test_closed_admin_snapshot_rejects_missing_tls_counter ;
test_closed_admin_snapshot_requires_stable_epoch_and_monotone_counters.
Zéro réel accepté, MISSING/absence rejetés ; mêmes incarnation/restart_epoch et
compteurs monotones obligatoires. Une absence de compteur ailleurs n'est pas
silencieusement transformée en0 ; aucune preuve globale déduite d'un sous-ensemble.

AdaptateurSAN modifié uniquement pour API fermée + sélection3compteurs/mêmeepoch/
monotones : deuxunités2PASS/0FAIL/0ERROR/0skip sans réseau, log.tmp/auth-envoy-
closed-snapshot-units-20261029.log. Reste non exécuté live, préconditionHEAD devra
suivre la livraison finale du helper avant tout lancement. SHAadapterb0e62e7c... .

## Identifiant de clé du laboratoire — plan avant correction

## Manifesto : vérificateur service dédié — plan avant code/run

Contrat architecte v1 lu intégralement et ratifié explicitement par le lead,
docs/contracts/manifesto-grant-snapshot-service-policy.md, référencé depuis AuthZ.
Oracles avant exécution :
- grant_snapshot_requires_explicit_service_principal_policy : config absente,
  vide, paddée, mismatch, >256, issuer/audience vides refusées opaquement ; positif.
- grant_snapshot_service_jwt_preserves_uuid_actor_and_query_principal : vrai JWT
  service via vraie route ; subA≠queryB, résultat discriminant B et positif sans query.
- grant_snapshot_rejects_foreign_authority_before_snapshot_lookup : clé/sub identiques,
  autorités IAM/org/autre/historique refusées, mauvais aud/alg/clé/exp/absence et
  headers/UUID seuls ; compteur lookup/commande reste inchangé.
- grant_snapshot_policy_does_not_leak_to_human_account_routes : service refusé sur
  route utilisateur, IAM légitime positif ; localiser le refus plutôt qu'erreur unrelated.
Un test peut couvrir plusieurs matrices par sous-cas nommés, pas nécessité de
nouveau framework. Réutiliser units configuration/setup, HTTP réel existant et IT
P3T4/P3T5 si touchées. Ajouter les noms de tout autre test avant son premier run.
Sources/test sont écrits par l'auteur, unique Cargo au lead ; pas run Kind pour IT.
La preuve source n'est pas la preuve du redémarrage de l'image corrigée.

Premier batch Manifesto natif sur canon ef4bc1f+sept fichiers gelés :
`cargo fmt --all -- --check` PASS6.90s ;
`cargo test --locked -p manifesto-configuration --lib` : 1PASS/0FAIL/0ignored/0filtered,
42.66s avec compilation ;
`cargo test --locked -p manifesto-setup --lib grant_snapshot_service_policy_tests` :
4PASS/0FAIL/0ignored/0filtered, 43.34s avec compilation. Les quatre oracles nommés
sont réellement exécutés et verts, pas seulement compilés. Aucun réseau/runtime.
Logs.tmp auth-manifesto-{fmt,configuration,setup}-20261029.log ; décodage strict
avec helper validé (UTF16LE pour logs PS des tests). Première extraction avait
une faute de regex JS avant lecture, corrigée sans relancer Cargo ni les tests.
Revue sécurité fresh du diff intégré en cours. Prochain check sélectionné selon
consommateurs : `cargo check --locked -p manifesto-service -p oodhive-monolith`.
Pas de rerun des quatre autres services inchangés ni du SDK par principe.

Check sélectionné terminé exit0 :
`cargo check --locked -p manifesto-service -p oodhive-monolith`, natifWindows,
log.tmp/auth-manifesto-monolith-check-20261029.log. Ceci vérifie la compilation
des consommateurs, pas les IT/runtime. L'arbre Rust est resté gelé pendant le run.

### Régression de format de clé IAM

Régression nommée avant code/run :
`test_local_full_iam_key_ids_satisfy_registry_uuid_v4_contract` dans
test_local_full_acceptance_source.py. Les deux TOML IAM normal/OAuth doivent
présenter le même kid UUID v4 simple, exactement 32 caractères hexadécimaux
canoniques. Baseline sur les configurations actuelles non conformes, puis
postimages corrigées et suite source existante ; rendu Kustomize offline ciblé.
Aucun test réseau nécessaire pour le format. Le prochain bootstrap réel reste
autoritatif et doit conserver le registre, le PEM, l'émetteur et les epochs.
Ce test n'est pas une preuve de signature ou de readiness réelle. Pas de nouveau
harness, mutation de données, règle domaine assouplie ni rotation automatique.

Résultat kid IAM sur candidat22 figé : baseline RO des deux TOML exacts au
fad3f8d (substitution de lecture seulement, aucun edit worker) : deux UUID invalides
détectés, puis échec de cardinalité attendu ; aucun incident d'environnement.
Candidat même test1PASS, suite26PASS0.517s, zéro skip. Aucun Cargo/runtime.
La copie des autres champs TOML doit rester strictement identique avant intégration.

Copie canon des trois fichiers identique octet pour octet au candidat validé,
diffcheck0 ; comparaison TOML confirme seul kid changé. Deux rendus offline
112/122objets exit0 ; la configuration réellement montée OAuth (Secret généré
offline, pas ConfigMap de base) porte le kid conforme. Aucune lecture Secret live.
Hook fmt de livraison worker échoue os206, pas un défaut fonctionnel ou une
permission de désactiver le hook ; aucun commit worker. Worktree/staged préservés.

Livraisoncanon ef4bc1f : hook normal cargo fmt PASS. Reprise runtime ciblée IAM
exit0/ready, même binairecf02c4a, DeploymentUID inchangé, nouveau Poddf614d2c...
réellement prêt. Hypothèse causale config validée au runtime. Secretconfig neuf
UIDef3f1b84... enregistré à CREATE, aucun GET Secret ni suppression prédécesseur.
Preuve session6b7.../iam-kid-recovery-95b120ae2da749238c0ee2a507520be7/summary.json.
Cela ne prouve pas signature/login, Envoy ou navigateur. Recheck infra borné demandé.

## Fermeture des findings finaux (avant runs)

S-2 : `test_local_full_default_projection_policy_is_explicit_and_aligned` dans
test_oauth_browser_alignment_source.py : rendu défaut et OAuth, fichier réellement
monté puis env effectives, couple exact8080/false et absence d'ambiguïté. Baseline
défaut sans principal doit échouer, puis même test et suite ciblée.
`config::tests::local_full_default_projection_policy_is_explicit_and_aligned`
dans sentinel-sync : Config/File réel désérialise la TOML montée, PrincipalPolicy
validate ; sans table principal la désérialisation reste refusée. Pas env globale,
réseau, testcontainer ou champ optionnel. Commande future après préflight :
`cargo test --locked -p sentinel-sync --lib config::tests::` (jobs24 inchangés).

S-2 résultat root : baselinePython KeyError principal, puis suitealignment11PASS
(2.377s) ; unitétypedRust1PASS/0ignored/49filtrés (24.65scompile), log
`.tmp/auth-s2-sentinel-3f5b713fc4ae4615b8b7349fca0ef70e.log`, exit0. Parse réel
du fichier monté et refus sansprincipal prouvés sans runtime tiers. Nouveau
gate ciblé unique aprèsOOM réussi à24jobs ; le grosbatch précédent resteFAILenv.
Fmtglobalroot etdiffcheck aprèscorrectifPASS. La preuve live de révocation reste
ouverte. SDK74e8 publiéexact/FF etcheckoutcanon sélectionné, diff+2lignes revu
parlesdeuxreviewers et63PASSdéjàvalables ; aucunrerunSDKredondant.

T-1/T-2/T-3 et T-4 : noms/sélecteurs du briefing tests2219Z attribués à pool-18,
deux fichiers de tests seulement ; runs conditionnés au bail Cargo/IT du lead.
S-1 régression nommée au briefing2222Z attribuée à pool-17 ; plan refs/port existant
avant production et zéro save/outbox sur refus. Aucun gros batch --tests relancé.

Propreté du warning introduit : helper privé member.rs publish_member_removed_event
passe de huit à sept arguments en réutilisant &Organization déjà chargée, sans
nouveau DTO/validation/ordre d'effets. Serena confirme un seul appel dans
remove_member, sans autre fichier affecté ; UUID/nom identiques à ces deux champs.
Contrôle avant run : `cargo clippy --locked -p hive-application --lib --no-deps --
-W clippy::too_many_arguments -W clippy::future_not_send -W clippy::too_many_lines`.
Pas nouvelle infrastructure ni allowlint ; garde et événement inchangés, testsIT
membership prévus restent utiles. Sources root gelées pour cette commande.

Résultat Clippy ciblé11.96s/exit0 ; aucun too_many_arguments/future_not_send/
too_many_lines, warnings préexistants glob/unused conservés. Log
`.tmp/auth-member-clippy-5500ef3ab61044b7ba4eeab3163e1387.log`. Pas preuveIT déduite.

Plan18 examiné avant edits : nouveau test `owner_issuer_down_refuses_without_modifying_data`
ajouté aux deux `initial_schema_` existants et T1/T2/T4 déjà nommés. Tableau de
six organisations datées, toutes pages1+terminale ; snapshot exact cardinalité
etensemble+acteur+owner ; WIF/HTTP7+7 refus9–13s/positif1+1, unRawSigner/opération.
Compilation/test par binaire cible et bail séparé, aucune nouvelle dépendance.

Complément T-3 requis par la bascule ratifiée (gap explicite du reviewer, pas
une nouvelle campagne) : `owner_issuer_upgrade_preserves_legacy_rows_without_backfill`
dans le même fichierHive. Sur fixture PG dédiée et transaction rollback existante,
préparer effectivement l'ancien schéma initial et des lignes antérieures SANS
colonne owner_issuer ; appliquer la vraie migration additive puis vérifier rows/
UUID/owner/membership/index et owner_issuerNULL, pas de stamping automatique.
Ne pas confondre insertionNone aprèsmigration et preuve d'upgrade ; aucun nouvel
outil de migration/runtime ou changement de prod. Sélection `owner_issuer_` réunit
ce test et le refusdowngrade, préflightIT final obligatoire comme les autresPG.

Précompilation ciblée aprèsOOM, sourcecanon c53e8f6 gelée :
`cargo test --locked -p hive-service --test organization_signer_permissions --no-run`.
Le test collision principal est inchangé par auteurs17/18 et sera exécuté dans
la campagne réelle, pas ici. Un seul binaire à lier,24jobsconfig inchangés ; pas
startDocker/testfixture par --no-run. Ce contrôle ne qualifie aucun refusHTTP/FGA.

Résultat ciblé36.12s/exit0, binaire organization_signer_permissions produit,
zéro test exécuté ; log `.tmp/auth-hive-signer-compile-a42a9afe4a1b4797b5333de08d52a352.log`.
Deuxième progrès vérifié aprèsOOM (avec typedunit sentinel), sanscacheclean ou
override24jobs. Ne pas lancerlesIT avantintégrationcorrectifsfinaux.

S-1 livré455306d puis intégré4b2ceb5, quatre sources/tests buffers byte-exact.
Application2PASS0ignored/3filtrés, domaine4PASS0ignored/13filtrés ; mutation de
la seule garde organisation⇒1PASS1FAIL, restauration exacte SHA256⇒2PASS. C'est
une preuve de discrimination de l'oracle, PAS une baseline historique de
l'ancienne signature. Logs UTF8 recoupés et copiés par allowlist sept fichiers
vers `.tmp/auth-s1-pool17-proof-455306d`, empreintes identiques. Hookfmt canon
normalvert. Unités/probes vraidomaine, aucunePG/FGA/file qualifiée par ces runs.

Prochain bail18 exclusif accordé APRÈS restitution17 et intégrationroot :
une fois septoracles source figés, organization_api_tests--no-run puis seulement
remote_deadline_is_shared_between_identity_and_http -- --exact. PasDocker/ITPG,
pasprod, unbinaryparcommande24jobs ; rootn'exécuteaucunCargo/hook en parallèle.

## Compatibilité de l'isolation des fixtures Hive avec la migration irréversible

Constat statique nouveau au préflight transitive : HiveTestDescriptor::run_migrations_down
(tests/common.rs72–78) appelle encore Migrator::down(None). La nouvelle migration
refuse volontairement tout down. SDKTestDatabase partage un même container/URL
par processus, avale cette erreur puis appelle up (database.rs93–106/125–131).
Une fois les deux migrations appliquées, up ne nettoie donc plus les lignes.
Risque de contamination intertests, pas un défaut de provenance production ou
un échec runtime observé. Ne pas changer le down production ou le SDK générique.

Correction locale test-only attribuée : dans le seul HiveTestDescriptor, recréer
le schéma de la fixture jetable exclusivement possédée via Migrator::fresh, puis
up générique inchangé/no-op ; aucun usage sur DB vivante. SeaORM migration1.1.20
résolu : fresh drop tables/types puis exec_up, contrairement à refresh qui passe
par down (source locale migrator.rs193–215/275–357 vérifiée). Cette opération ne
sera exécutée que dans les PG testcontainers tracés au préflightIT final.
Nouveau test nommé avant run : `fixture_schema_reset_clears_rows_and_reapplies_owner_provenance`
dans organization_api_tests.rs : ligne avant reset, reset via vrai descriptor,
ligne absente/schemaowner_issuer et deux migrations présents, insertion légitime
ensuite possible ; refusdown production testé séparément et inchangé. Aucun
nouveau harness/deps/config runtime. Après sourcefigée, seul no-run affecté à
requalifier, pas rerun remoteT4 inchangé ; les huit oracles restent à exécuter
en campagnefinale pour les septPG, hors seulT4déjàvert.

## Préflight statique de la livraison Linux (sans Docker)

Dockerfile.build copie le contexte incluant .cargo/config.toml, dont target-dir
est WindowsE:, mais collecte les binaires sous /usr/src/target/release. Aucun
overrideLinuxdansleDockerfile oudockerfile-specificignore trouvé. Nom avant run :
`test_linux_delivery_build_keeps_24_jobs_and_its_own_target` dans la suite source
existante test_local_full_acceptance_source.py : conserver jobs24/targetWindows
config locale, ENV CARGO_TARGET_DIR Linux verscachecollecté, cargo release locked.
Baseline sourceguard puis petitpatchDockerfile uniquement, même sélection et
suiteexistante. Ce n'est pas une preuve de build Linux réel. Images aprèsIT,
depuis un worktree livraison propre HEAD+SDK exacts, jamais contextecanon dirty
contenant .tmp/.worktrees/inputs privés ; pas de nouveau script/harness ou testDocker.

Résultat sourceguardLinux : baseline1FAILENVabsent, puis1PASS et23PASSsuite
(0.341s). Recheckinfra statique demandé au reviewer14, buildLinux réel nonexécuté.

Livraison18 finale bbeb7da (parentff63cd4) : huitoracles ; troisfichiersdetests
dontcommonfreshfixture-only, SDKinchangé/pin74e8. LogsUTF16LE recoupés parhelper
strict : no-runHive initial10.60s et final17.62s, zérotest ; T4remote1PASS/0ignored/
19filtrés/13.59stest (38.34scompile), nonrejoué. SeptPGrestentnonexécutés.

Deux chaînes SQL complémentaires à réaligner AVANTIT : snapshots signer_writer_snapshot
et last_principal_event pointent encore sur tablelegacyoutbox_events. Writeractuel
HiveOutboxUnitOfWork→OutboxRecorder etentitéSDKécrivent rustycog_outbox_events
(transaction.rs11, setup/app.rs354, rustycog-outbox/entity.rs12). Remplacer seulement
table danscesdeuxhelpers, assertionsintactes. Sélecteurs existants
recognized_same_sub_distinct_issuers_do_not_share_organization_admin et
hive_membership_and_projection_preserve_target_issuer. Ce n'est pas unROUGEIT
exécuté ; uncountsurtablelegacyvide serait unoraclefaussement rassurant.

Troisfichiers18intégrésvia team_merge natif etcomparés bufferbrut au bbeb7da :
3/3identiques ; correctionrootcomplémentaire = seulesdeuxchaînes SQL ci-dessus,
aucuneassertionaffaiblie. Logs7fichiers archivés parallowlistetSHA256 sous
`.tmp/auth-oracles-pool18-proof-bbeb7da`, décodageUTF16LEstrict des troislogsCargo.
Retenir17.62scompilefinal(no-run), T4 1PASS/0ignored ; pas septPGPASS implicite.
QualificationT1/T2/T3/resetpg toujoursouverte, tâcheoraclesnonferméeavantpreuves.

Canonfinale03fddcfigéaprèsintégration : slotCargoLEADuniquereprisdepuis18.
Gatesannoncésavantlancement, tousséquentiels24jobs/E: : check--locked6composants
deréférence (nouveaugardeSync), puis no-runHive organisation_signer_permissions,
member_update_api_tests etorganization_api_tests (deuxSQLcorrigées/commonfresh et
nouvelleAPIintégrée), puisClippy4packagesall-targets2lintsderéférence au nouveau
testremote. AucunPG/fixturetestexécutépar--no-run. PasrerunSDK63/remote19/T4par
principe ; preuvesexactesdéjàvalidesréutilisées. RechecksRO16oracles/14Docker
pendantgel ; correctionsultérieuresattendentfinCargo avantmutationsources.

Gates e03 terminés : check6exit0/4.76s, Hivesigner--no-run19.70s, member12.11s,
org11.18s, Clippy4all-targetsexit0/3.99s ; zéroPGexécuté/aucunOOM. Logs
auth-integrated-*référencésauledger, warningspréexistantsconservés, pas2lintsvisés.

Warningexamined : update_and_delete_require_auth fonctionn'étaitpasenregistrée
en#[tokio::test], DÉJÀà8fa (comparaisonsourcevérifiée), mêmesassertions401.
Réactiver seulement#[tokio::test]+#[serial] avantcettefonctionexistante. Nouveau
sélecteur nommé iciAVANTrun : `update_and_delete_require_auth`. Contrôle
`cargo test --locked -p hive-service --test organization_api_tests -- --list`
prouveenregistrementdesoraclesetdel'anciendéfaut, sans exécuterfixtures. La
fonction préexistante non testée ne constituepas unPASS antérieur niunerégression
introduitepar18. Pasnouveautestharness/production/assertionsaffaiblies.

List-onlyréel : exit0/7.24scompile, **23tests enregistrés**, tousseptoraclesPG
initiaux +update_and_delete_require_auth présents. Zérofixture/testPGexécuté.
Log `.tmp/auth-org-test-registration-e7c0c8a4-fb3f-4301-b52c-945bab2056ad.log`.
Le casauthréactivé ajouteunhuitièmetémoinPG pertinent àla sélectionfinale ;
T4remote resteunitaire1PASSdistinct. AucunchangementproductiondepuisS1.

Préflight de la suitefermée it-nativeHive : sqs_event_routing_tests possède son
propre descriptor privé (59–64), seul autre appelMigrator::down(None) dans
Hivetests aprèsscanexact. Même correctionlocaleMigrator::fresh, sanschangement
des sixoracles ni du downprod. Le fichier activeHIVE_QUEUE__ENABLED dansson
seulprocess(27–30) et utilisePG/SQS/FGA réels : la suiteHivecomplète exécutera
donc ces sixcas, ce n'est pas unskip ni une activationglobale. Parentregistre/
inventairesdoiventtracerLocalStackcommePG/FGA. Pasnouvellefixture/harness.
Sélecteurs existants routes_organization_* / routes_member_* ; compilation
ciblée--no-run avantcampagne, évaluationréelleparfixtureexistanteaprèsgates.

Préparation mémoire de la campagne fermée existante : metadataCargo a identifié
les cibles IAM/Hive sansfeature requise. Pour nepasréintroduire le pic de linkage
du batchayantéchoué1455, précompiler CHAQUEcible test/bin de ces deuxpackages
séparément, toujours24jobs internes etE:, --locked --no-run. Inventaire/plan/logs
résultats séparés sous.tmp ; premier échec stoppe la séquence, aucunefixture
exécutée. Ce n'est pas un nouveauharness : lesIT seront ensuite exécutées par
ops/tests/it-native.ps1 -Suite hive,iam inchangé, qui réutilise cesartefacts.
Sourceba21a3cgelée, SDK74e8publié, aucun autreCargo/hook. Lesdeuxbinaryqueues
optent localement selonleurssources, pas d'overrideglobalpourforcerKafka/SQS.

### Résultat de précompilation et lancement IT réel

Notification reçue :54/54 cibles compilées, exit0 ;773 secondes cumulées dans
summary.json. Les54 logs recoupés : marqueur exit0 présent, aucun résultat de
test, aucune erreur1455/allocation. HEADba21a3c et SDK74e8 inchangés, jobs24 et
targetE: conservés. Cela résout le blocage de préparation mémoire, pas les oracles
fonctionnels. Preuves : .tmp/native-it-precompile-f2b93d6ed7f644e58f0a00b69dd7db32/.

Commande réelle suivante déjà approuvée :
`& ./ops/tests/it-native.ps1 -Suite @('hive','iam')`.
Runner existant inchangé, un Cargo à la fois, threads libtest par défaut2 ;
aucun --ignored. Auth negative réactivé et huit témoinsPG inclus, routageSQS
exécuté par ses binaires, signupKafka/SQSignored explicitement hors preuve.
Le runner enregistre les IDs à la création, inventaires et témoin Engine puis
arrête gracieusement les seules fixtures possédées et retire les Exited(0).
Aucune ressource préexistante adoptée ; leur mémoire volatile reste préservée.
Secrets et sorties brutes restent dans les preuves privées, restitution filtrée.

### Première campagne réelle : Hive vert, IAM arrêté sur configuration de fixture

Campagne it-00687e1bb02b42b78c45e0f712e442a6, sourceba21/SDK74e8 : Hive67PASS,
dont23 organisations et3 collision/signataire ; aucunignored Hive. IAM s'arrête
dans organization_account_guard : le témoin publié n'atteint aucune assertion
de route, car setup_test_server_with_signing_keys échoue au boot avec
PRINCIPAL_INVALID: policy does not match signing authority. L'autre cas de ce
binaire passe ; les ciblesIAM suivantes n'ont pas été exécutées. PasPASPIAMglobal.

Cause localisée : helper privé common.rs modifie jwt.public_base_url etjwt.issuer
pour la clé plateforme réelle, mais conserve auth.jwt.platform_issuer de la
configuration habituelle. Le validateur de production refuse correctement cette
incohérence (configuration/lib.rs993–1007), avant démarrage. Pas de défaut démontré
de la garde de compte. Serena références vide pour ce module de tests ; grep exact
recoupe4 consommateurs : organization_account_guard,public_refresh_expired_access,
registration_usage_boundary,signing_epoch_writer. Couverture de l'index limitée.

Correction test-only prévue : dans ce seul helper, aligner explicitement la policy
HTTP sur l'issuer exact de la clé plateforme et déclarer historical=false, comme
la fixture dédiée le demande. Aucunfallback, alias ou changementvalidateur/prod.
Conserver les assertions positives/négatives des4 consommateurs. Avant nouvelIT,
précompiler seulement les41 ciblesIAM du plan existant une par une à24jobs, puis
reprendre `it-native.ps1 -Suite iam` inchangé ; ne pas rejouer Hive dont aucune
source n'est touchée. Nouveaux sélecteurs non nécessaires : oracles existants nommés
ci-dessus, déjà dans le plan de campagne.

Cleanup vérifié sur échec :40 créations/40 retraits,0 possédéprésent/running,
0 inconnue/issuecleanup/nouvelle étrangère ; pic3 possédés (4total avec préexistant).
Les20 conteneurs préexistants ont mêmesIDs et états après. AucunarrêtDocker/WSL
de propriété inconnue ; registre .tmp/docker-resources-20261028.log.

Recoupement nominatif des10 lignesPASS exactes : huit témoins organisationsPG
(T1/T2/deuxinitialschema/upgrade/refusdown/reset/authnegative), collision entre
autorités et acteur/cible projection membership tous exécutés et verts. AvecT4
remote déjàvert etrevue indépendante des oracles, la tâche de fermeture des quatre
lacunes est maintenant complétée. La suiteIAM et les preuves Envoy/navigateur ne
sont pas englobées par cette clôture bornée.

Recompilation après correction de fixture0ae4222 :41/41 ciblesIAM exit0,
636s cumulées ; les41 logs confirment le targetWindowsE:, aucunOOM, aucun test.
La faute du champ descriptif target_dir du plan initial est documentée dans
metadata-correction.json, pas un override effectif. Recheck test-reviewer ciblé
PASS statique sansfinding, assertions4consommateurs intactes.
Reprise réelle autorisée suivante : `it-native.ps1 -Suite iam` uniquement,
sources0ae/SDK74e8 figées et cleanup/inventaires identiques àla premièrecampagne.

### Reprise IAM : premier défaut fermé, second arrangement de récepteur localisé

Campagne it-c2e84f197dbb4268b355f511b3591796 : organization_account_guard2PASS,
public_refresh_expired_access etregistration_usage_boundary1PASSchacun ; la cause
précédente est effectivement corrigée, sansmodifier leurs assertions. La suite
avance jusqu'à signing_admission :3PASS/2FAIL. Les2échecs arrivent dans le helper
assert_platform_receiver au constructeurSDK, errorPRINCIPAL_POLICY_UNAVAILABLE,
pas sur un résultat de quota ou de signature. Les cibles suivantes restent non
exécutées ; aucunPASSIAMglobal.

Cause source : l'unique JwtAuthConfig construit directement dans IAMtests ne
déclare pas les2champs obligatoires de principal, malgré issuer déjà connu.
Recherche exacte de toutes les constructions JwtAuthConfig/AuthConfig et
UserIdExtractor::new dans IAMtests : ce seul site ; autres receivers inlineJWKS
déjàexécutés avec succès. Serena retrouve les3appels des2tests en échec.
Correction test-only : platform_issuer=Some(issuer.into()), historical=false,
sansmodifier extraction, URLJWKS, assertions de publication/quotas ouproduction.
Sélecteurs existants nommés : les2tests d'admission du dernier budget et du17e
slot plateforme. Compiler uniquement signing_admission--no-run, pas41cibles.

La reprise reste via `it-native.ps1 -Suite iam` existant : interfaceferméeaux
suites, aucunfiltredebinaires. Choix borné de garder son inventaire/cleanup prouvés
plutôt qu'ajouter un nouveau runner oumodifierl'infrastructure de tests. Ce coût
de reprise des anciens cas est imposé par cette interface, pas un nouveau gate
global. Hive67PASS toujours réutilisé sans rerun. Compte d'essais : progrès réel
sur le précédent frontier (garde2PASS), nouveau défaut de fixture indépendant.

Cleanup reprise :30created/30removed,0ownedprésent/running,0issue/unknown/unowned,
picowned1 ;20préexistants mêmesIDs/états après. Registre contemporain du runner.

## Qualification indépendante pendant l'enquête SDK sur les ledgers

Sources canoniques e44/SDK74e8 figées, le nouvel auteurSDK reste en planRO sans
Cargo ni runtime. Baillead temporaire pour deux suites ext-authz existantes,
chacune séparément : `cargo test --locked -p ext-authz --test registry_publication`
puis `cargo test --locked -p ext-authz --test mesh_path_standin`. Sources/tests
recoupées : collaborateursHTTPWireMock, vraie sérialisationpublisherIAM, aucun
Postgres/testcontainer/Envoy. La première suite vérifie vidage autoritatif sans
résurrection et délai monotone réel59+2s autour de60s, puisrecoveryremplacement ;
la seconde couvre métadonnéestrust/signature etcontratstrip/recreate duprincipal.
Ce sont des preuvesHTTP/SDK, PAS une preuveEnvoyréel/Kind/navigation ni une IT
publisherwriter. Ne pas doubler les suitesni en annoncer le résultat avantrun.

Résultats canoniquese44/SDK74e8 : registry_publication2PASS0ignored/63.51stest
(compile42.96s), mesh_path_standin11PASS0ignored/1.60stest(compile5.24s), exits0.
Logs .tmp/auth-ext-authz-registry_publication-1724020a8c37462ab54c513de674ca80.log
et .tmp/auth-ext-authz-mesh_path_standin-c4689d0ba22c45c28674b7d80b92d088.log.
Lignesnominativesrecoupées : autoritatifvide/recovery/borne60s, spoof/expiration/
signature/type/pending/trustscope/retiring. AucunefixtureDockercréée parcesruns.
Conserver limite : standin ne prouvejamais le handshake d'Envoy ouunvrai navigateur.

## Registre SDK : plan borné ratifié avant implémentation et tests

PlanRO auteur19 msg_18524a2b8001UUTK7QtPVRyBIE lu intégralement ; acceptationlead
dans le mandat : helperprivé dir/run/PID/UUIDincarnation, filenameunique par
incarnation, create_new sansretry, seule catégorieIOopaque dansl'erreur. JSON,
run_id/process_id/createdID,flush/sync etownership restentinchangés. Deuxfichiers
SDK seulement : fixture_runtime.rs ettestconsommateur fixture_process_exit_it.rs.
Découvertechild depuis snapshot AVANTspawn, uniquement nouveauxfichiers réguliers
auformat exactrun/PID/incarnation, cardinalité1 etvalidationJSON existante,
aucuneadoptionancienledger. Ambiguïté=>échecfermé, pascontainerparnom/inventaire.

Sept nouveaux oracles nommés AVANTruns :
- fixture_ledger_new_incarnation_preserves_older_run_pid_ledgers
- fixture_ledger_same_incarnation_refuses_overwrite
- fixture_ledger_io_error_reports_only_error_kind
- child_ledger_discovery_excludes_pre_spawn_reused_pid_ledgers
- child_ledger_discovery_rejects_missing_or_multiple_new_ledgers
- child_ledger_discovery_matches_exact_run_and_pid
- child_ledger_records_reject_wrong_identity_and_invalid_created_id

ROUGE fonctionnel : extrairehelper avec anciennom, oracleincarnationéchoue ;
suffixeUUID ajouté et mêmeoracleVERT. L'erreurcompilation n'estpaspreuveROUGE.
Commandes sousbailworker19 exclusif natif24jobs/E:, SDKworktreeisolé74e8 :
`cargo test --manifest-path .worktrees/auth-fixture-ledger-20261029/Cargo.toml --locked --features testing --test fixture_runtime fixture_ledger_`
`cargo test --manifest-path .worktrees/auth-fixture-ledger-20261029/Cargo.toml --locked --features testing --test fixture_process_exit_it child_ledger_`
puisfmtSDK --all -- --check. Filtrespurs, aucun enfant/Docker ; témoinprocess_exit
réel NONautoriséàce stade etàpréflighter séparément si nécessaire.
Aucun changement publicAPI/env/schema/deps ; publicationSDK attendgates/diff/revues.

Livraison SDK19 reçue msg_185441b26001h2mOTR3lYdrBk7, sourcefigée2fichiers330+/12-,
pascommit/push. Preuves auparent.tmp/auth-fixture-ledger19-* ; globstandard les
masquait (gitignore), readdirfiltré les alocalisées sansautredécouverte.
ROUGEfonctionnel1FAIL0ignored19filtrés/22.55scompile ; VERTledger3PASS0ignored17filtrés
/11.44scompile ; childdiscovery4PASS0ignored2filtrés/14.45scompile. Fmtfinal0,
diffcheck0. Leshashesfichiersstablespendantgates ethashpatchfinal447a14bd96c6c497778608ba3467ac22e9c86408f03a61c4f6df673c4eefd079
ontétérecoupésau gitdiffcourant. Aucunchildréel/Docker/Envoy/Kinddéduitdecesunités.
Revuessécuritéownership ettestsoracles demandéesmêmessessions15/16, deuxquestions
distinctes utilesavantpublication ; pasnouvelle revueglobaleSDK/AuthZ. Brief factuel
persisté .cursor/review-briefings/20261029-sdk-fixture-ledger-candidate.md.

Revue15 sécurité et16 couverture reçues,persistéesetindexées : PASSbornés sans
findingbloquant surcepatch exact ; oraclesdiscriminants, ancienledgernonadopté,
correlationparentpréservée. PasqualificationenfantWindowsruntime/reparseeffectif.
Décisionlead : candidatprivéadmis àpublicationexacte autorisée surbranchemission
canonique aprèscontrôlehashes/diff/remote ; aucunAPI/schema/env/depsnouveau.
RepriseIAMintégrée obligatoire avantKind, noms/testspreuvesdéjàplanifiés. Le
témoinenfantSDKréel n'estpaspromu enPASS, ni une nouvelleplateformeDocker créée
pourcomblerunhorsscope ; limiteconservée. Causehistoriquerestehypothèse.

SDK02e5db23aac6016aab062c3078696d5f7fd464ef committéexact2fichiers, parent74e8,
hashessourcesidentiquesauxgates/revues ; publication FF surbranchemissioncanonique
vérifiée parls-remoteexact. Checkoutcanonique sélectionné, configurationSerena
étrangère mêmeSHA256, aucuneassertionprod niCargo.lockmodifié. Gitlinkappàfiger
avantnouvellecampagne ; pasdepushapplication.

Préparationnative suivante : réutiliserleplan41ciblesIAM connu, chacune --locked
--no-run séparément,24jobs/E:inchangés, nouveauSDK02e5 ; sourceappfigéeaucommit
depin. Cette recompilation est requise par le changement de cratefixturepartagée,
pas nouvellequalificationAuthZparent. Puisrunnerexistant `it-native.ps1 -Suite iam`
avec inventaires/IDs/registre contemporain etcleanup obligatoire. AucunKind ou
browser avant IAMfinale. Hive67PASS resteune preuve sourceinchangée ba21/SDK74e8 ;
nenommeraucuneréexécutionHivesous02e5. Validationruntimecommunledgerparent àfaire
parIAM, limitesSDKchildWindowsréel conservées sansPASSimplicite.

### IAM intégré au SDK02e5 — résultat réel acquis

Séquence sh_185530ff5001yUiiqM0E9Vysbj terminée exit0 :41précompilations exit0
(721s cumulées), puis campagne it-1de7a72631cf4f4493a908fbc49f09b3 : **290PASS,
0FAIL,1ignored**,41binaires terminés. Sourceapp00c6cf2/SDK02e5, aucunlockdrift.
Lignesnominatives recoupées : gardecompte, sessionspersistées/rotation, dernierbudget
cross-org et17eslotplateforme tousPASS ; signing_admission5PASS/signing_epoch_writer7PASS,
oauth_browser_transactions8PASS/token22PASS, autresrésultatscomplets danslelog.

Ignoredréel unique : test_signup_sqs_integration, opt-in explicite. SignupKafka
compilécommebinairevide (featurekafkaabsente), pasundeuxièmeignoredexécuté. Aucun
PASSKafka/SQSsignup revendiqué ; les2routagesSQS IAM existants sont bienPASS.
La prédictionpréflightde2ignored n'étaitpasunrésultat ; lecompteréelcorrige1ignored.

Cleanup :36created/36removed,0ownedrunning/present,0issues/unknown/newunowned,
picowned2 ;20préexistants mêmesIDs/états après.35ledgers tousnouveauformatincarnation,
aucunPIDdoublédanscefinalrun ; nepasaffirmercausehistoriquerétroactivementprouvée.
Compatibilité réelle SDKledger/parentEngine démontrée surcettecampagne réussie.
Preuves .tmp/auth-iam-incarnation-1d48036bc5614e888ff9d22e3edf5e00/ et
.tmp/itfixtures-it-1de7a72631cf4f4493a908fbc49f09b3/, registre contemporain.

TâchecorrectifledgerSDK désormaiscomplétée APRÈSpreuveintégrée, pasparseulpush.
GateITavantKind satisfait avecpreuvesréutiliséesétiquetées : Hive67PASSsurba21/SDK74e8,
sourcesHiveprod inchangées ; IAM290PASSsous02e5 ; ext-authz13PASSHTTPsoure44/74e8,
nouveauxoraclesSDK7PASS. PasdequalificationglobaleEnvoy/browser/production.

## Préflight livraison finale — deux builders secondaires

MétadonnéesDocker/Kind lues sansmutation : clusterfull absent, aucunconteneur
fullpréexistant ; port18080/18081libres. Seulconteneuractifétranger : Kindlegacy
aiforall-local-control-plane, nepasarrêter/adopter. DiximagessourcesduprofilOAuth
absentes ; seulbuild-artifactslatestpréexistant deprovenanceinconnue, nepasréutiliser
pourlespreuvescourantes. ConstruiredepuisworktreepropreSDK02e5 etnoterIDsréels.

LecturetransitiveREADMEbuilders : ext-authz etapparatus-controller recopientaussi
.cargo/config.tomlWindows maiscollectent /usr/src/target, sansENVoverride. Même
défautstatiquequeleprincipalDockerfile déjàcorrigé. Fixbornénécessaireàlalivraison
duprofilfullautorisé : ENVtargetLinuxet--locked dansces2builders, aucunbinary/
fonctionApparatus/plugin/SDK/changementarchitectural. GarderleursbasesRustexistantes
tantqu'aucunéchecMSRVétabli, jobs24inchangés. Sourceguardexistant nommé
test_linux_delivery_build_keeps_24_jobs_and_its_own_target étenduaux3builders
AVANTbaseline ; puispetitpatchconfigetmêmesélection/suite23. LesITnativesrestent
réutilisables : aucunRust/runtimeWindows/configauthmodifié ; buildLinuxréelreste
àexécuter, infrarecheck aprèsdelta.

Résultat sourceguardsecondaires : baseline1test/2subtestsFAIL (ENVmanquant), puis
mêmesélection1PASS etfullsuite23PASS/0.356s, sansDocker. Patch2builders limitéà
ENVtargetLinux+--locked, pasderéduction24jobs ouchangementRustbase. Infra14recheck
RO demandéavantbuild ; côtéapplicationRust/SDK/authaucundelta aprèsITnative.

Contexte Gitproprea7+SDK02e5 créé maisgatedetransfert arrêtéAVANTDocker : il
contient .opencode/missions historiquementSUIVI (225fichiers), pasune copie du
runprivécourant. .tmp/.worktrees absents, gitstatuspropre ; nomdedossierseulne
prouvepascontentsecret. Décisionhygienebornée : nepaslire/supprimercesarchives,
lesexclure ducontextbuildaveclesautresétatsagents/temp/worktrees via lesdeux
Dockerignore applicables (globaletcontroller-specific). Ce n'estpasuneditdes
configurationsMCPglobales ouunepermissiondebuilderdepuiscanon dirty.
Nouveau sourceguard nomméavantbaseline :
test_linux_delivery_context_excludes_agent_state_and_worktrees ; vérifierexclusions
explicites .tmp/.worktrees/.opencode/.cursor/.serena/obsidian/**/.git danslesdeux
filtres. MêmeconditionGitpropre+SHAexacte, puisavancerleworktreepossédépropreau
commitdecesfiltres,aucunreset/nettoyageadoption. NouveauDockerbuildtoujoursNONlancé.

Exclusionscontexte : baseline1test/2subtestsFAIL puis1PASS etsuite24PASS/0.267s.
Les filtres sontdesfichiersversionnés du dépôtuniquement ; aucun répertoirede
mission/sourcehistorique luoueffacé. La gardeGitpropreestmaintenue ; .tmp et
.worktrees doiventresterabsentsduworktreelivraison, .opencodehistoriquesuivi est
présentmaisexcluparDockerignoreapplicable. PasunepreuvebuildLinuxeffectif.

Précision de provenance après préparationducommit : .dockerignorecanonique est
unfichierLOCAL PRÉEXISTANT IGNORÉ, nonversionné ; gitaddl'arefusé (pasunrefusde
permissionoutil). Aucunforceadd/embarquementdesoncontenuétranger. Nosseuleslignes
ajoutées yontétéretiréesparpatchciblé,contenuantérieurconservé. Routecompatible :
nouveautemplateversionné ops/deploy/build/delivery-context-ignore.txt demission,
copieexclusive vers .dockerignore dansSEULworktreelivraisonpossédé, hashidentique
autemplate. Controller-specificresteversionnéetcomplété. Sourceguardvisecetemplate
etlefiltrespecific, paslaforeignconfig. Gitpropre etsourcesHEAD/SDKexacts restent
obligatoires ; seulinputgénéréignoréautorisé = copiedufiltreversionné, identité
documentée. Aucuninputprivé/clé/preuve copiée. Troisautresfichiersintentionnels
avaientétéstagéspargitaddpartiel ; nepasreset/unstageétrangers, contrôlerwhitelist.

Templateownedfinal etsourceguard24PASS/0.255s, canonical.localDockerignore non
suivi/ignoré confirmé ; template.txt nonignoré, aucunforceadd. Documentation du
build précise copieexclusive/hash etpréservationdufichierlocalétranger. Lecommit
de livraison contienttemplate/filtrespecific/test/QA/README seulement, pas.le
Dockerignorelocal. La copie dansleworktreepropreestuninputdemétadonnéesdéterministe
dufichierversionné, pasunecopie d'artefactsprivés etpasuneinstallation/runnertest.

### Exclusions récursives — plan avant renforcement

Avant tout transfert, le contexte matérialisé contient aussi les répertoires
suivis `rustycog/.cursor`, `rustycog/.serena` et `rustycog/obsidian` (existence
seulement inspectée). Les règles racine ne suffisent pas pour ces chemins.
Le dernier patch groupé a échoué sur une ancre QA et n'a changé aucun fichier,
fait revérifié par `git diff` au HEAD 4fcca1d.

Renforcer le test existant
`test_linux_delivery_context_excludes_agent_state_and_worktrees` avant baseline :
exiger les six exclusions racine et leurs variantes `**/`, dans le template
versionné et le filtre spécifique du controller. Constater la baseline, puis
ajouter ces seules règles et rejouer la même sélection et la suite de 24 tests.
Aucune modification du `.dockerignore` canonique local ni lecture des archives.
Mettre à jour la copie possédée seulement après vérification de son ancien hash
connu, puis vérifier les SHA application/SDK et les deux arbres Git propres.

Résultat : sélection existante rouge, deux sous-tests pour les six règles `**/`
absentes dans chacun des filtres ; après ajout, même sélection verte puis suite
24/24 PASS (0.266 s). Aucun Docker ou Cargo exécuté, aucune source Rust changée.
La revue infra reçue au 4fcca1d est persistée avec sa limite racine ; le delta
récursif sera transmis et la provenance de la copie générée actualisée avant build.

### Livraison Linux — résultat principal et images suivantes

Source figée cf02c4a / SDK02e5, filtre final vérifié indépendamment par infra14.
Premier build `Dockerfile.build` réussi : exit0/116.6s, release locked terminé
1m31s, image Linux/amd64 ID aa7c846a04953b5961f9d4a2419b75c8c52e64b5beb455b12f399265c3054c92
(préfixe sha256), labels app/SDK exacts. Ancienlatest et conteneurs inchangés.
Build ne vaut aucun test fonctionnel ; transfert14.52MB ne prouve pas à lui seul
chaque exclusion, celles-ci sont contrôlées statiquement avec revue et guard.

Plan livraison en cours : dix images source profil OAuth, séquentielles depuis
le contexte propre, sept consommateurs avec BUILD_IMAGE explicite ; ext-authz,
controller et Zot gardent leurs Dockerfiles existants. Préflight tags absents,
IDs --iidfile enregistrés immédiatement, aucune adoption d'ID préexistant.
Le lot s'arrête au premier échec ; pas de baisse de jobs, nettoyage de cache ou
build de développement Linux. Aucun cluster créé à ce jalon.

### Oracle Envoy live — scénario nommé avant campagne

`envoy_same_ca_expected_san_roundtrip` : après déploiement ancré et contrôle des
UID propres, utiliser le vrai Envoy local-full et son IAM upstream. Positif sur
GET public exact `/iam/api/auth/gitlab/login` sans JWT, capturer seulement statut,
chemin et effet TLS (pas code, cookie ou state). Avec les seuls certificats
générés de ce run, comparer la validation du leaf IAM puis du leaf Hive : même
CA transport, mais ce dernier ne contient pas le SAN IAM attendu.

Négatif borné proposé : mettre uniquement le leaf/key Hive du run dans le Secret
transport IAM possédé, sans lire un Secret live ni changer la CA, la config Envoy,
les routes ou l'identité cliente. Patch d'annotation de Deployment possédé pour
renouveler les connexions ; UID avant/après et effets du rollout archivés. IAM
doit rester démarré avec ce leaf valide. Le vrai Envoy doit refuser le flux avec
échec de vérification SAN discriminant (compteur `fail_verify_san` si disponible
dans l'image existante, sinon diagnostic TLS précisément borné), non un simple
503 ou timeout. Restaurer le leaf IAM connu, nouveau rollout et positif équivalent.

Pas d'exécution de ce scénario à cette ligne : noms/UID réels, accès d'observation
Admin loopback, outils présents et effets du patch restent à préflighter. Aucun
pod de preuve, port exposé ou installation nouvelle implicitement autorisé.
Si l'observation discriminante manque, résultat INCONCLUSIVE, jamais PASS depuis
le rendu ou reqwest. Le rollout ne donne aucun droit de remplacer un objet
étranger ou de modifier la cible. P1–P3/N1–N3 navigateur restent un run distinct.

### Bootstrap Windows — paramètres ACL, plan avant correctif

Premier essai : zéro nœud/volume créé, 20 préexistants inchangés. Arrêt dans
secure(directory) avant écriture creation.phase. Répertoire d'ancrage vide retenu,
bail pending non réutilisé. Invocation PowerShell actuelle reproduite sans effet
avec marqueur : `$args[0]` non lié, commande exit1 ; témoin en bloc de script lie
le marqueur, exit0. Ce n'est pas un refus d'accès établi ni un manque d'autorisation.

Correctif local : un helper privé transmet le chemin en Base64 UTF-8 dans une
commande fixe encodée UTF-16LE (option standard PowerShell EncodedCommand).
Aucune interpolation de chemin en syntaxe PowerShell, aucun argument résiduel,
aucune variable d'environnement nouvelle. Règles ACL opérateur/SYSTEM, lecture
seule, allowlist et refus de fallback inchangés ; ErrorActionPreference=Stop.

Nouveaux tests dans la suite existante, nommés avant exécution :
`test_windows_acl_directory_and_private_probe_bind_literal_paths` (vrai processus
PowerShell, uniquement répertoire temporaire propre, pas Docker/API/secret),
`test_windows_path_command_uses_encoded_data_and_propagates_failure` (frontière
de commande et échec opaque). Reproduire d'abord le premier avec l'ancien code,
puis correction et deux sélections, suite target complète. Aucun changement Rust,
SDK, images ou politique de confiance. Revue bornée utile avant nouveau bootstrap.

Baseline native avec l'ancien code : échec dans secure, avant le probe, même
AnchorError que le bootstrap. Premier correctif valide secure(directory) sur le
vrai OS ; le test avance au probe du fichier mutable et échoue. Diagnostic borné
montre une ACL par défaut du fichier contenant une identité non traduisible :
le refus strict du probe n'est pas une preuve de défaut des règles de privacy.
L'arrangement sera donc explicitement protégé avec secure(file), témoin positif
private_file=True, puis rendu mutable dans le seul cleanup de ce fichier propre
et témoin négatif False. Aucun assouplissement de Translate ou de la policy.

Autre fait observé : les messages de progression PowerShell OEM850 provoquent
des erreurs de décodage dans le lecteur Python UTF8. Le helper fixe explicitement
sa sortie UTF8 et supprime seulement les messages de progression, jamais les
erreurs. Le test couvre aussi le chemin Unicode, la commande encodée et la
propagation d'échec. Ce progrès localisé ne justifie aucune relance de Kind à ce stade.

Le témoin positif a ensuite localisé un second défaut préexistant : secure(file)
retire WriteAttributes au propriétaire avant de positionner IsReadOnly. Diagnostic
PowerShell UTF8 précis : accès refusé à la définition d'IsReadOnly, ACL déjà
appliquée, flag encore mutable. Ce n'est ni un refus d'outil ni une permission
à élargir : repositionner le flag sur le fichier neuf possédé avant d'appliquer
la même ACL finale. Pas d'autre utilisateur, escalade ou droit final ajouté.
Le même oracle positif/négatif contrôle le résultat réel ; pas de bootstrap retry.

Après remise en ordre, secure(file) et le probe positif réussissent réellement ;
l'échec se déplace au teardown du test qui tentait de rouvrir un fichier scellé.
Cette réouverture n'appartient pas au contrat et n'est pas ajoutée au produit.
Les artefacts non secrets de ces essais sont conservés, aucun forçage de droits.
Arrangement final du test : DACL privée connue avec droit WriteAttributes gardé
uniquement par le fichier de probe du test, flag read-only puis mutable, cleanup
ordinaire. Test supplémentaire nommé avant run :
`test_windows_seal_marks_readonly_before_withdrawing_write_attributes` vérifie
l'ordre et les droits finaux inchangés de secure(file), sans nouvelle ressource.
Le succès natif du seal réel dans l'essai précédent reste une preuve bornée,
pas un PASS de test dont le teardown a échoué.

Résultat final : sélection Windows 3 PASS, suite cible 24 PASS (2.044 s), zéro
skip et aucune erreur de décodage. Le contrôle natif porte sur protection d'un
répertoire propre, chemin Unicode/littéral, probe read-only accepté puis mutable
refusé. Le seal de fichier garde les droits finaux initiaux ; son ordre est
couvert par oracle source/commande et son positif natif précédent reste distinct.
Logs .tmp/auth-windows-acl-binding-accepted-{green,suite}.log ; anciennes preuves
rouges/teardown échoué conservées, aucune réécriture de résultat.

Avant nouveau bootstrap : revue sécurité bornée du diff guard/tests, commit du
delta de mission, nouveau bail pending (jamais réutiliser le répertoire d'ancrage
vide du premier essai). Les images cf02c4a restent valides : aucun Rust, Dockerfile,
configuration de service ou SDK n'a changé, seulement helpers de conduite Windows.
Conserver séparément SHA des binaires/images et SHA des opérations de bootstrap.

Revue sécurité indépendante reçue et persistée : PASS borné, aucun finding sur
les deux postimages exactes recoupées avec git hash-object. Pas de nouvelle
garantie d'immutabilité ni de permission de descellement. Reprise conforme :
committer ce delta Python/tests/QA, avancer le contexte propre et créer une
nouvelle lease pending ; les assets privés de cette même mission restent identifiés.

### Deuxième bootstrap — DACL protégée Windows, plan avant correction

Le nouvel essai c9cbf80 s'arrête encore avant Kind : zéro ID nœud/volume,
creation.phase absent et répertoire d'ancrage vide. Diagnostic natif limité à
ce répertoire possédé : Set-Acl demande SeSecurityPrivilege, absent. Comparaison
read-only des tokens natif et Context Mode : tous deux non élevés, privilège
absent. Ne pas relancer via un autre outil, activer un privilège ou changer de compte.

Différence d'arrangement localisée : le bootstrap crée son répertoire avec
Path.mkdir(mode=0o700), le test natif utilisait mkdir par défaut. Renforcer ce
même oracle avec 0o700 avant nouvelle baseline. Hypothèse documentée : Set-Acl
réapplique le descripteur complet sur cette DACL protégée et demande une écriture
d'audit non nécessaire ; le contrat ne modifie que propriétaire/DACL.

Alternative compatible à examiner/tester : API .NET Framework SetAccessControl
de l'objet fichier/répertoire, qui persiste ses sections modifiées, sans demander
de nouvelle identité ou privilège. Même propriétaire, mêmes ACE et protections,
aucun accès SACL/audit ajouté ou supprimé. Documentation Microsoft .NET4.8.1
confirme l'API disponible dans Windows PowerShell5.1 utilisé. Pas de mutation
runtime avant validation native de la condition réelle et recheck sécurité ciblé.

Baseline renforcée reproduite : oracle natif0o700 ERROR AnchorError dans secure,
pas d'accès Kind. Remplacement par FileSystemInfo.SetAccessControl sur les sections
Owner/Access modifiées (aucune règle Audit ni modification SACL). Sélection3 PASS,
puis suite24 PASS. Ajout d'assertions natives des droits exacts sur ce répertoire
protégé, suite finale24 PASS2.243s, aucun skip. Pas d'activation de privilège,
élévation ou compte différent ; ErrorActionStop et refus natifs restent propagés.
Log .tmp/auth-windows-protected-dacl-exact-suite.log, anciens rouges conservés.
Diff restreint guard/tests/QA soumis à recheck avant commit et prochain bail neuf.

Recheck sécurité reçu : PASS borné sans finding sur les postimages exactes.
Revue indépendante de Reference Source confirme masque Owner/Access modifié,
pas Audit/SACL, sans dispenser des droits Windows de ces sections. Les contrôles
de refus restent actifs. Commit puis nouvelle lease pending uniquement ; aucun
ancien ancrage adopté, aucune élévation pour la reprise.

### Cluster neuf créé — frontières pipeline et sortie native

Troisième bootstrap b9c5eaa : trois nœuds créés/ancrés, IDs Engine exactement
égaux à ceux du bail, trois volumes corrélés, UID kube-system vérifié. Scellement
Windows réellement exercé ; le bootstrap s'arrête ensuite à l'entrée pipeline
de Invoke-LocalKubectl avant application Calico (InputObjectNotBound).
Le log montre aussi un readerthread UnicodeDecodeError du subprocess Kind :
Go émet UTF8, capture text implicite CP1252. Pas d'erreur à masquer ou ignorer.

Plan avant fix/run : ajouter l'acceptation ValueFromPipeline explicite au helper
PowerShell avancé, en conservant l'ordre/complet des objets et tous arguments de
cible ; pas de nouvelle commande permise. Dans run Python, capturer bytes puis
décoder strictement la sortie textuelle UTF8 au retour, pour ne pas perdre une
erreur de décodage dans un thread ; garder les appels binaires byte-exact et les
échecs opaques. Encodage explicite du helper Windows conservé.

Tests nommés avant exécution dans la suite existante :
`test_windows_kubectl_pipeline_preserves_payload_and_target_arguments` (PowerShell
natif avec Invoke-LocalTarget remplacé localement par un spy, aucun appel API),
`test_native_runner_decodes_utf8_without_reader_thread_loss`,
`test_native_runner_rejects_invalid_text_without_reflecting_output`,
`test_native_runner_preserves_binary_io` (enfants Python locaux seulement).
Baselines ciblées, correctifs et suite target. Aucun Cargo/IT/Kind dans ces tests.

La reprise de bootstrap s'appuiera sur la lease ANCHORED existante, dont le chemin
retourne après vérifications sans appeler Kind create. Pas de nouveau pending,
pas reset/restart de nœud. Sources binaires/images restent cf02c4a, SDK02e5.

Baselines : pipeline natif ERROR InputObjectNotBound ; runnerUTF8 deux FAIL
(mauvais texte/perte de thread et texte invalide non bloqué), témoin binaire déjà
vert. Après un paramètre ValueFromPipeline dans le helper avancé et capture
bytes/décodage strict dans run : pipeline1 PASS, runner3 PASS, suite28 PASS.
Oracles renforcés sans nouveaux noms : payload multiple/multiligne/absent,
arguments cible inchangés, modes text/universal combinés, mauvais UTF8 stdout
ou stderr (exit0/3), entrée non encodable opaque, binaire byte-exact. Suite finale
28 PASS2.954s, aucun skip/readerthreaderror ; logauth-cli-frontiers-reviewed-suite.

Décision de séquencement proportionnée : intégrer ce delta de conduite revu par
le lead et lancer la reprise réseau de l'ancrage existant ; revue sécurité RO
du commit exact en parallèle, avant toute étape applicative/credentials OAuth.
Le bootstrap ne déploie ici que le CNI public épinglé. Aucun nouveau verdict
fonctionnel ni permission cible n'est créé ; finding bloquant éventuel arrête
les admissions suivantes. Pas de pipeline de rôles ou attente de GO utilisateur.

### LocalStack — boucle de disponibilité, plan avant correction

Déploiement OAuth arrêté exit1 sur second rollout de dépendance (301.42s),
applications/bootstrap pas encore appliqués. Sept dépendancesReady, LocalStack
Running/nonReady. API health localhost : SQS running, hookREADY init-aws.sh ERROR.
Script monté0555,160octets, shebangLF correct, sansBOM/CRLF/backslash-n littéral.
awslocal implicite tente http://localstack:4566 et échoue à connecter (10.88s) ;
même GET explicite127.0.0.1:4566 obtient NonExistentQueue : serveur vivant, files
absentes. Cause compatible avec service sans endpointReady pendant son bootstrap.

Nouveau test source nommé AVANT baseline/run :
`test_localstack_internal_bootstrap_uses_loopback_but_advertises_service_host`
dans test_local_full_acceptance_source.py. Vérifier init ET deux lectures de la
sonde sur endpointloopback explicite, tout en gardant LOCALSTACK_HOST=localstack:4566.
Baseline rouge, correction des deux seules configurations puis sélection/suite.
Pas de changement Rust/SDK/binaire, nouveau service ou probe simulant un succès.
La preuve réelle sera hookREADY réussi et les deux files présentes sur même API.

Reprise runtime bornée après integration : patchUID du ConfigMap init et du seul
DeploymentLocalStack possédés, rollout attendu et UID Pods avant/après tracés.
Conserver files/données/Secrets, pas delete/reset ; poursuivre bootstrap/apps à
partir des helpers existants, pas rerun intégral recréant les secrets exclusifs.

Baseline : deux sous-tests rouges (bootstrap/readiness). Après correction des
deux configurations, sélection1PASS puis suite25PASS/0.294s, zéro skip. Rendu
Kustomize réel offline des profils local-full et oauth-proof : exit0,112/122objets,
ConfigMap générée localstack-init-b56fgd6t65 et deux appels de sonde loopback.
Première analyse supposait un nom sans hash et a levé StopIteration ; commande
de rendu réussie mais analyse corrigée, aucun accès API ou source modifiée pour
contourner cet échec. Le nom généré change légitimement avec le script : la reprise
créera la nouvelle ConfigMap depuis le rendu et patchera le Deployment possédé
vers cette référence exacte, sans écraser/supprimer l'ancienne ConfigMap.

Première application runtime a créé cette seule ConfigMap puis échoué avant
modificationDeployment : generation1, ancien volume/probe etPodUIDinchangés.
Frontière transport confirmée : kubectl1.36.1 --patch-file '-' lit un fichier
nommé '-' (source v0.36.1 os.ReadFile + diagnosticnatif--local sanssecret/API).
Correction de l'enveloppe, pas produit : patchpublic via-p JSON ; validation
UID+contenu de laCMcréée avantreprise, testsUID+champsvisés au lieu d'un état
resourceVersion global variable. Aucun fichierprivé ou secretpassé enargv.
Deuxième application autorisée aprèscettecauseconfirmée, pasdererundéploiement.

Oracle SAN préparé mais toujoursnonexécuté : transportSecret par vrai fichier
privétemporaire, privacyvérifiée sur fichiervideAVANT contenu puisreadonly,
cheminseulenargv, cleanupordinaire sansACLélargie. Revueoracles16 actualisée.

Vérification réelle de la correction : repriseexit0, mêmesUIDDeployment,
rolloutPodUIDd836...→fd2fa4a6-f81d-4cbd-846c-7aa6da7318cd, hookREADY SUCCESSFUL,
deuxGETqueuesréels exit0. Preuves session6b7.../localstack-recovery-d1c0402eac4f4bc2939b9db838208270.
AncienCMconservé, nouvelleCMcrééeUID57a9aa32-8f95-4c76-9858-f62ea1624219.
Reprise aprèsdépendances seulement ; pas de globalPASSauth ni Secret/job recréé.

### Verdict de l'oracle SAN — corrections avant usage

Revue16 indépendante reçue/persistée : deux MEDIUM dans script non exécuté.
Compteur absent ne vaut pas zéro ; exception d'observation/restauration ne vaut
pas défaut TLS. Correction locale des seules fonctions de lecture/verdict : trois
compteurs requis sinon INCONCLUSIVE ; FAIL seulement mauvaise feuille acceptée,
conservé même si une erreur ultérieure survient. État restauration/cleanup séparé.
Cas synthétiques nommés AVANT validation, unittest temporaire sans runtime :
`test_san_counter_omission_is_inconclusive`,
`test_san_complete_triplet_can_pass`,
`test_san_restoration_observation_error_is_inconclusive`,
`test_san_accepted_wrong_name_remains_fail`.
Pas de nouvel oracle de production ou framework ; script de preuve sous.tmp,
recheck16 ciblé après correction, aucune campagne avant ces corrections.

Correctionoraclescript : parse_tls_counters exige3compteurs, aucunevaleurpar défaut ;
discriminationcomparative etverdictpurs testables, absence/exception→INCONCLUSIVE,
mauvaisnomobservéaccepté→FAILconservémêmesiAdmin/restaurationéchoue ensuite.
Étatpatch/rollout/HTTP/restauration/cleanup séparé ; statutnégatif capturéavant
requêteAdmin pournepasperdreunrougeétabli. QuatresynthétiquesPASS0.000s, log
.tmp/auth-envoy-san-verdict-unit-fixed.log. Premieressai n'aexécutéaucuntest
(crochetmanquantSyntaxErrordansscript), corrigésansexécuterleruntime ; traceinitiale
conservée. SHAoracle87e6680f0900ff4dd7c78431863721ace4125fa0f06654da7a38ecd5a02e7f2e.
Ce sontpreuvesdevérificationduverdict, pasdehandshake ou privacyWindowslive.

### Diagnostic RO borné — contrat ratifié avant code

Note architecte4 lue intégralement ; delta compatible etratifié explicitementpar
lead dans .cursor/handoffs/20261029-local-full-bounded-pod-logs.md. AucuneADR ou
permissionnative/global/RBAC ajoutée ; target+UID+ownership+noSecret conservés.
Caps200lignes/32768B/30s, namespacefullonly, Pod+containerexactrégulier,
attestationparentavantlecture etpostcheckPodUIDavantrestitution ; race=>discard.
LogsseulementtraitésContextMode/motifsfermés, jamaisraw/secret/indexation.

Testsfake nommés AVANT code/run, suite test_local_full_target.py existante :
`test_logs_accepts_exact_owned_pod_container_and_bounded_previous`,
`test_logs_rejects_invalid_grammar_before_any_probe`,
`test_logs_rejects_wrong_endpoint_and_stale_cluster_before_log_request`,
`test_logs_require_owned_namespace_pod_uid_and_named_regular_container`,
`test_logs_discard_capture_when_pod_identity_changes`,
`test_logs_failures_do_not_echo_sensitive_output`.
Pas de boucleCLIlogs générique sansownership ; pasde logs brutstandalone.

Premiercandidat revu parlead, sixnouveauxfakePASS0.399s puis suite34PASS5.632s
dansworktreefigé ; pasCargoniAPI. Retourlimitéàcatégoriesfermées satisfaitprivacy.
Complémentborné nécessaire audémarrageCrashLoop : catalogue statiquedesymptômes
configuration/fichier/panic, pasextraitougroupedynamique. Nouveau nom avantedit/run :
`test_logs_return_only_closed_startup_categories` ; conserver tousrefus etsentinelles.

## Correction du statut du forwarding — plan avant run

Finding infra indépendant local_full_target.py480–493 : exit enfant ignoré et
CLI0. Nouveau test nommé
test_forward_cli_propagates_child_failure_and_preserves_owned_interrupt dans la
suite cible existante, faux enfant/target seulement (aucun socket/API/processus).
Témoins exit0⇒0, exit1⇒1 opaque et KeyboardInterrupt volontaire⇒0 ; baseline puis
petite correction main et même sélection, enfin suite target complète. La garde
UID et le cleanup par seul handle restent inchangés et couverts par tests existants.

Exécution lead : baseline du nouveau test FAIL0!=1 pour enfant exit1, puis même
test PASS après contrôle du retour process.wait dans main. Témoins exit0 et
interruption volontaire passent sans altérer le cleanup. Suite target complète
**21PASS**,0.256s. Aucun subprocessus kubectl réel, socket ou runtime utilisé.

Le plan de mission approuvé reste la référence de portée. Ce registre nomme les nouveaux tests avant leurs runs et conserve les preuves reçues, sans déclencher toute la liste de référence ni créer une infrastructure.

## SDK — batch auteur Windows natif

Snapshot livré : `72845d9ddfc590852602791eb50cfad6262d80bc`, comparé au parentd60a88b. Slot exclusif pool-3 restitué explicitement.

Noms inscrits par l'auteur avant run : `qualified_principal_encoding_preserves_utf8_and_size_bound`, `legacy_platform_grants_require_provenance`, `principal_cache_keys_include_issuer`, `uuid_extension_alone_is_not_a_permission_principal`, `optional_principal_denial_is_not_wildcard` ; sélection des régressions HTTP/JWT existantes selon le diff.

Commandes et résultats transmis :
- `cargo fmt --manifest-path rustycog/Cargo.toml --all -- --check` : premier FAIL format ; rustfmt ciblé sur dix fichiers auteur, même commande PASS ensuite.
- `cargo check --manifest-path rustycog/Cargo.toml --locked --features http --lib` : PASS.
- `cargo test --manifest-path rustycog/Cargo.toml --locked --features http --lib permission::` :14PASS dont5 middleware ; `--lib http::middleware_auth::` :1PASS.
- `cargo test --manifest-path rustycog/Cargo.toml --locked --features http --lib http::` :28PASS.
- `cargo test --manifest-path rustycog/Cargo.toml --locked --features http --test permission_middleware --test mesh_gateway_auth` :12+4PASS.

Total distinct annoncé53, aucun ignored ; ne pas additionner les sous-filtres HTTP déjà inclus. Tests lib/features non sélectionnés restent absents. Une revue indépendante bornée précède publication exacte sur branche de mission ; test collision Hive réel encore NON EXÉCUTÉ. Traces natives à recouper au besoin, aucune compilation seule n'atteste la sécurité globale.

## SDK RS256 — compatibilité sous feature testing

### Périmètre borné du gap de fixtures

Constat statique sur72845d9 : plusieurs JwtAuthConfig des tests RS256 gardent les deux nouveaux champs à None ; factoryrs256_extractor utilise un helper pur alors qu'un test appelle aussi extract_user_id. Les positifs de cette suite n'ont pas été exécutés par les53 tests featureshttp ; ne pas les prétendre verts.

Fermer ce gap sans changer production/assertions : adapter fixtures/factories dans le seul `rustycog-http/src/jwt_rs256_tests.rs`, policies explicites de l'autorité attendue du scénario ; boolhistoriquefalse sauf le cas nommé iamrusty. Commande prévue, à confirmer par source avant run : `cargo test --manifest-path rustycog/Cargo.toml --locked --features http,testing --lib http::jwt_handler::jwt_rs256_tests::`, puis fmt SDK. Ce module sélectionné utilise WireMock et les clés crypto de fixture, pas un testcontainer ; inspecter les préconditions/transitifs avant exécution. Une fixture désormais invalide n'est pas un ROUGE de production. Toute défaillance crypto réelle reste ROUGE à remonter, pas une assertion à adapter.

Pas de nouveau harness ni test d'acceptation inventé : c'est la compatibilité d'une suite existante avec le contrat d'API changé. Même auteur/contexte SDK, à déclarer ; revue finale indépendante conservée. Nouveau commit test-only et publication explicite avant mise à jour du pin, aucune réécriture de72845d9.

## Envoy — batch statique auteur

Commande nommée au plan avant run : `python -B -m unittest discover -s ops/tests -p test_envoy_upstream_server_identity.py -v` ; quatre fonctions : `test_compose_clusters_require_their_server_dns_identity`, `test_kind_clusters_require_their_server_dns_identity`, `test_expected_identities_are_issued_by_certificate_generators`, `test_local_full_render_requires_its_rewritten_server_dns_identities`.

Traces natives recoupées : quatre tests/quinze échecs d'assertion SAN avant correction, quatreOK après. Rendu Kustomize réel hors ligne en mémoire, pas d'erreur d'environnement/skip. Vrai handshake Envoy mauvaisSAN/bonSAN mêmeCA NON EXÉCUTÉ ; infra-verifier reste exigé après modifications infra intégrées au laboratoire.

## Signataire et navigateur

Nouveaux noms/commandes spécifiés dans les contrats ratifiés ou en cours de livraison ; aucun run de ces nouveaux tests à ce jalon. Le futur auteur doit consigner ses sélecteurs effectifs avant run. Les commandes Cargo globales de référence non exécutées ne sont pas implicitement PASS. Tests IT via fixtures existantes après préflight ; Kind après corrections et IT finales seulement.

### Plan remote approuvé avant premier run

Le plan local transmis par pool-5 nomme : `remote_endpoint_security_modes_are_explicit`, `remote_verified_requires_rs256_only`, `remote_constructor_is_secure_by_default`, `remote_rejects_unsupported_algorithm_before_io`, `remote_rejects_invalid_digest_before_credentials_or_io`, `remote_authorization_rejects_invalid_credentials_without_leak`, `remote_redirects_never_reach_target`, `remote_success_bodies_are_bounded_without_content_length`, `remote_rejects_encoded_responses`, `remote_http_errors_are_not_read_or_reflected`, `remote_operation_deadline_covers_identity_and_body`, `remote_tls_authenticates_server_name`, `remote_errors_redact_hostile_inputs`, avec publicRSA8192 et six scénarios historiques conservés.

Commandes autorisées sous son créneau exclusif après restitutionSDK : `cargo test --locked -p iam-configuration --lib`, `cargo test --locked -p iam-infra --test remote_signer`, `cargo test --locked -p iam-infra --lib signing::remote::tests::`. HTTPsortant WireMock existant ; petit auxiliaireraw pour flux réellement inconnus/segmentés et terminaisonTLS privée selon motif existant, pas nouveauharness. Dépendancesdev bornées au contrat ; lock uniquement référencesiam-infra si nécessaire. Les échecs de compilation d'API nouvelle ne constituent pas un ROUGE fonctionnel. Résultats pas encore reçus.

Actualisation auteur remote reçue : configuration29/29PASS, remote_signer16/16PASS, TLSprivé2/2PASS (133autresunits filtrés). MêmeCA bon/mauvaisSAN etCAinconnue réellement exercés parclientprovider ; redirectionTLS→HTTP zérorequêtecible surSign/GET. PasdeROUGEbaseline : production avait précédé accordlead explicite et premiercréneau, écart déclaré, pas TDDreconstitué. Tracesàrecouper avantacceptation finale. Noms ajoutés par auteur AVANT prochainrun : `remote_accepts_rsa8192_public_material`, `remote_key_id_stays_one_encoded_segment`, `remote_rejects_premature_eof`. Ce témoin8192 est publicstructure, pas qualification signatureRSA8192.

### Livraison remote finale recoupée

Commit auteur17c9cfd intégré au6b6d364, sept fichiers identiques vérifiés. Le delta []=>RS256 a été qualifié ensuite par29 tests config (trace msg_1840f5d81001icyscFw5q1qxjU). Réutilisation19 remote et2 TLS réellement observés dans msg_1840185a9001RsYKxlGlqv6bvE ;133 autres units volontairement filtrées dans le seul module TLS, pas ignorées. fmt trois packages IAM et Clippy configuration/infra ciblés PASS ; rootfmt FAIL environnement OS206. Les doublons de présentation de la même commande ne sont pas additionnés. Aucun PASS du nouveau SDK combiné ou de la mission globale déduit.

### Premier gate applicatif

Commandes annoncées avant run : `cargo check --locked -p sentinel-sync --all-targets` PASS ; `cargo check --locked -p hive-domain -p hive-application -p hive-infra -p hive-http --lib` PASS après correction namespace via réexport existant ; `cargo test --locked -p sentinel-sync --lib translator::hive::` sixPASS/0ignored.

Noms annoncés avant run : organization_created_writes_owner_tuple, member_joined_emits_role_plus_base_member_tuple, organization_deleted_deletes_owner_and_member_tuples, member_roles_updated_replaces_role_tuples, hive_membership_and_projection_preserve_target_issuer, organization_deletion_keeps_same_sub_authorities_distinct_and_rejects_legacy. Aucun oracle collision Hive réel ou migration DB qualifié par ces unités ; source applicative reste incomplète. Slot app restitué avant micro-batch remote ; celui-ci est maintenant aussi restitué.

### Second gate applicatif, inscrit avant exécution

Sous nouveau créneau exclusif pool-3 : refresh offline limité aux deux références dev-deps hive-service déjà résolues, puis fmt fichiers auteur et checks locked des quatre bibliothèques Hive, manifesto-http/telegraph-http, hive-service --tests (compilation seule). Pas de fixture/queue/IT. SDK publié ne peut pas être modifié sur un échec full/testing sans nouvelle attribution.

Test pur nommé : `cargo test --locked -p manifesto-http --lib principal_tests::`, oracle `public_read_keeps_foreign_authority_out_of_uuid_only_branches`. `monolith_principal_policy_is_explicit_and_consistent` reste différé au broker IAM intégré. L'oracle IT `recognized_same_sub_distinct_issuers_do_not_share_organization_admin` est écrit mais NON EXÉCUTÉ : trois clés distinctes, trois autorités reconnues, vrai PG/FGA, positifs propres, révocation indépendante, refus avant RPC/mutation. Sa compilation n'est pas un PASS fonctionnel.

### Raccordement IAM : plan approuvé avant production

Source explicite des deux champs `[auth.jwt]`, validation de PrincipalPolicy et correspondance exacte avec l'autorité réellement signée ; helper crypto pur conservé, composition/setup contrôlés avant callback signant. False explicite aux profils ordinaires, true seulement dans le cas de fenêtre historique testé.

Nouveaux noms déclarés avant run : `iam_http_verifier_requires_explicit_principal_policy_before_signing_io`, `iam_http_verifier_preserves_configured_authority_and_legacy_window`, `iam_http_verifier_rejects_foreign_hmac_issuer_at_uuid_only_boundary`.

Séquence prévue : oracle Cell contre le helper setup existant pour un ROUGE fonctionnel avant correction lorsque la compilation le permet, puis configuration/setup et témoins. Commandes sous futur créneau dédié : `cargo test --locked -p iam-configuration --lib` ; `cargo test --locked -p iam-setup --lib <sélecteur nommé>` ; `cargo test --locked -p iam-setup --test seeded_verifier_configuration` ; checks IAM/monolithe selon état des branches, fmt ciblé et Clippy pertinents. Le monolithe global ne peut pas être déclaré vert avec les anciens consommateurs ; compilation seule/erreur fixture ne sont pas un oracle fonctionnel.

### Troisième gate applicatif et fixtures SDK

Résultats Gate3 : Hive-service --tests compile PASS, member_issuer7PASS ; SDK
baseline bloquée E0425 par import de test avant tout run, puis baseline19 dont11
fixtures refusées faute de policy, puis19PASS/0ignored après adaptation test-only.
Nouveau commit local77672b8 parent72845d9, deux fichiers ; fmtSDK PASS. Revue
indépendante bornée demandée sur snapshot propre avant publication séparée.

Créneau lead suivant nommé avant run : `cargo test --locked -p iam-infra --lib
signing::remote::tests::`. Justification : couvrir les imports explicites du
module TLS intégrés après le run auteur et leur combinaison avec IAM/SDK pincés.
Le module privé ne lance que des sockets loopback/TLS, aucun testcontainer.
Arbre canon gelé ; slot unique restitué par auteur et réservé au lead.

Résultat réellement exécuté au canon (IAMc98b6ea/SDK72845d9) : **2PASS/0FAIL/
0ignored**,133 autres units filtrées,46.61s compilation et0.18s tests. Bon nom de
certificat et refus redirection downgrade couverts par ces deux tests. Warnings
de fixtures non utilisées rapportés, aucun nouveau testcontainer/runtime tiers.
Slot lead restitué avant nouveau bail applicatif pool-3.

### Décision navigateur finale, avant ses runs de profil

Contrat architecte v2 ratifié au21ea615 : origine navigateur18080 mais
public_base_url/autorité8080 préservés, policies explicites8080/false et deux
callbacks18080 uniques côté IAM et GitLabConnect. Oracles nommés dans
`docs/contracts/0412-oauth-proof-authority-alignment.md`, notamment rendu effectif,
registry indépendant du port de signature et scénario P1–P3/N1–N3. Sélectionner
ceux utiles au delta, pas exécuter toute une nouvelle matrice par principe.
Inventaire Kustomize vérifié9historiques+proof=10, aucune exclusion Knative.

### Qualification finale à libérer après intégration des auteurs

Publication SDK776 réalisée après revue fresh des oracles, sans changement de
production/assertions : branche mission avancée FF depuis728, ls-remote exact.
Pin canonique2b0ea47, config Serena étrangère préservée par SHA256. Les19PASS
auteur restent distincts de la revue statique et du module TLS exécuté au canon.

Nouveaux sélecteurs annoncés par l'auteur avant le bailAPP final :
`cargo test --locked -p hive-domain --lib sync_service::principal_tests::` :
auto_invite_owner_requires_exact_platform_provenance,
historical_auto_invite_job_rechecks_owner_before_provider,
sync_without_auto_invite_accepts_foreign_or_unresolved_owner,
non_inviting_job_creation_does_not_require_platform_owner ; ainsi que
Manifestoapplication public_only_read_never_looks_up_members_or_org_admin et
HiveHTTP member_principal_ambiguity_is_conflict_without_identity_echo. Pas encore
de résultat reçu à cette ligne. Handler sentinel de projection est aussi attribué,
ses noms effectifs à annoncer avant run, ROUGE avant correction si faisable.

Attribution handlertests précisée avant run dans le fichier Telegraph existant
notification_created_routing_test.rs, aucune dépendance ajoutée. Sélecteur
`cargo test --locked -p telegraph-service --test notification_created_routing_test sentinel_principal_` :
sentinel_principal_legacy_owner_is_failed_without_effects,
sentinel_principal_legacy_deletion_is_failed_without_effects,
sentinel_principal_qualified_event_writes_and_replay_is_idempotent,
sentinel_principal_unknown_event_remains_completed_noop,
sentinel_principal_translator_error_is_opaque. Baseline handler puis correction
et même sélection ; unités WireMock/ledger, pas un OpenFGA réel ou les IT du
reste du fichier. Les quatre oracles Sync ont été renforcés depuis74c4349 et
doivent être rerun au même sélecteur ; les anciens4PASS ne leur sont pas attribués.

Browser auteur a nommé avant ses runs :
test_oauth_proof_keeps_8080_authority_and_exact_18080_redirects,
test_oauth_registry_redirect_is_independent_of_platform_public_base_url,
test_proof_origin_redirects_pkce_and_private_connector,
test_proof_private_inputs_drive_hash_without_secret_output. Livraison rapportée
9c114db : alignment10, target20, simulator5, SAN4 PASS ; acceptance21/22 avec
oracle source périmé utilisant ancien scalaire Suite au lieu du selected réel
de la boucle. Micro-correction attribuée dans ce test existant (sans toucher le
runner), même suite seulement à requalifier ; pas de faux ROUGE produit.

Gap supplémentaire vérifié par lecture jwks_seed_tests.rs22–32/130/325 : sa
factory de configuration n'a pas encore de policy explicite, donc des positifs
du constructeur et des négatifs de seed peuvent être invalides/vacuement verts.
Attribution de deux champs à la factory seulement, nouveau commit test-only.
Sélection à exécuter après son bail APP (slot exclusif explicite) :
`cargo test --manifest-path rustycog/Cargo.toml --locked --features http,testing --lib http::`,
baseline puis même sélection après adaptation et fmt SDK. Oracles JWKS existants
incluant seed vide/snapshots/fraîcheur/binding, pas de nouvelle infrastructure.
Les19PASS précédents ne leur sont pas attribués. Aucun testcontainer/ignore trouvé
dans le périmètre rustycog-http/src ; modules gated jwt_handler/jwks seulement.

Le profil livré est une préparation : secrets privés par run en Secret/create
only, trustbundle via init image IAM identique et SSL_CERT_FILE process-local.
Backend TLS réellement livré, confiance effective et refus nom/CA, navigateur
réel et corrélation des négatifs avant IAM restent NON PROUVÉS.

Après intégration laboratoire36ffab6 et IAMc98b6ea, run lead ciblé annoncé :
`python -B -m unittest discover -s ops/tests -p test_oauth_browser_alignment_source.py`.
Justification distincte du run auteur : le rendu réel utilise maintenant aussi
les champs policy du configIAM de base intégré après sa branche435376f ;
contrôler l'assemblage final et absence de conflit/duplication. Aucun runtime,
aucun Cargo ou secret réel, Kustomize hors ligne en mémoire seulement.

Résultat canon réellement exécuté :10 tests, exit0,1.975s ; aucune panne ou
preuve réseau déduite. Les autres suites auteur identiques ne sont pas rejouées.

Ne pas démarrer runtime ou IT pendant les éditions de production. Après livraison
app/lab intégrée, geler l'arbre compilé et effectuer check des six packages de
référence, fmt ciblé/global avec résultat réel et Clippy utile. Revue sécurité
indépendante du diff intégré et revue des oracles adversariaux, contextes fresh,
briefings avec vrais HEAD/diff avant correction ; pas de revue parent redondante.

Préparation statique du runtime seulement à ce stade : le lanceur existant
`ops/tests/it-native.ps1` prend une liste fermée de suites, pas une sélection de
targets. Il assure les témoins Engine, registre à création et cleanup possédé.
Préférer sa campagne Hive/IAM couvrant les composition roots touchés plutôt
qu'un nouveau wrapper. Vérifier le choix final des suites après diff intégré,
les opt-ins queues laissés désactivés et les tests réellement exécutés/ignorés.
Les ressources existantes ne deviennent pas possédées par leur nom. Sources
transitives consultées : Hive common (PG+OpenFGA, pas SQS) et fixture_runtime
ledger exclusif/fullIDs, collision fail-closed sans adoption. Inspection finale
des effets et préconditions reste à faire sur le pin exact retenu avant lancement.
Kind/full et navigateur réel seulement après IT finales ; aucun lancement ici.

Après restitution du micro-batch IAM, prochain créneau exclusif pool-3 : `cargo check --locked -p hive-service --tests` après attribution db.rs (compilation seule), puis `cargo test --locked -p hive-domain --test member_issuer` pour les règles d'autorité modifiées. Pas de PG/FGA ou queue exécutés par ces commandes.

Dans le même bail, sous réserve des préconditions déjà décrites : baseline de la suite SDK existante `--features http,testing --lib http::jwt_handler::jwt_rs256_tests::`, qualification séparée des fixtures invalides, correction test-only autorisée et même suite VERT, fmt SDK. Pas de changement de production/assertions pour obtenir VERT. SDK publié72845d9 immuable ; nouveau commit local test-only, sans push ou pin non publié. Cette extension ferme un gap connu d'appelants de l'API modifiée, pas une nouvelle infrastructure.

### Plan navigateur approuvé, configuration finale temporairement gelée

Noms inscrits avant campagnes statiques : `test_full_http_downstream_preserves_internal_mtls_and_shared_tls`, `test_proof_registry_drives_exact_relink_exemption`, `test_proof_origin_redirects_pkce_and_private_connector`, `test_forward_rejects_foreign_target_bind_service_and_mapping_before_api`, `test_forward_requires_owned_service_and_stops_only_owned_pid`, `test_simulator_consumes_code_once_and_verifies_s256_client_redirect`, `test_simulator_receives_real_authorize_token_profile_without_secret_output` ; négatifs TTL/corps/bindings et assertions du scénario JS selon plan auteur.

Commandes futures nommées : quatre suites Python du contrat plus `test_envoy_upstream_server_identity.py` après delta de renderer/certs. Tests déjà annoncés par auteur : deux rendererOK après un échec d'assertiontransport et un helperAPIabsent ; trois fixtureOK sanssocket. Helperabsent n'est pas un ROUGE fonctionnel. Les traces seront recoupées au livrable. Aucun moteur/réseaucluster exécuté. La valeur d'autorité du profil final est en réexamen architecte ; les tranches statiques indépendantes seules continuent.
