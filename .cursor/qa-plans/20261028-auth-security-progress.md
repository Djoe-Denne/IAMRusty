# Registre QA de progression — sécurité authentification

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
