# Registre QA de progression — sécurité authentification

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
