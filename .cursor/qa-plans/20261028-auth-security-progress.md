# Registre QA de progression — sécurité authentification

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

Après restitution du micro-batch IAM, prochain créneau exclusif pool-3 : `cargo check --locked -p hive-service --tests` après attribution db.rs (compilation seule), puis `cargo test --locked -p hive-domain --test member_issuer` pour les règles d'autorité modifiées. Pas de PG/FGA ou queue exécutés par ces commandes.

Dans le même bail, sous réserve des préconditions déjà décrites : baseline de la suite SDK existante `--features http,testing --lib http::jwt_handler::jwt_rs256_tests::`, qualification séparée des fixtures invalides, correction test-only autorisée et même suite VERT, fmt SDK. Pas de changement de production/assertions pour obtenir VERT. SDK publié72845d9 immuable ; nouveau commit local test-only, sans push ou pin non publié. Cette extension ferme un gap connu d'appelants de l'API modifiée, pas une nouvelle infrastructure.

### Plan navigateur approuvé, configuration finale temporairement gelée

Noms inscrits avant campagnes statiques : `test_full_http_downstream_preserves_internal_mtls_and_shared_tls`, `test_proof_registry_drives_exact_relink_exemption`, `test_proof_origin_redirects_pkce_and_private_connector`, `test_forward_rejects_foreign_target_bind_service_and_mapping_before_api`, `test_forward_requires_owned_service_and_stops_only_owned_pid`, `test_simulator_consumes_code_once_and_verifies_s256_client_redirect`, `test_simulator_receives_real_authorize_token_profile_without_secret_output` ; négatifs TTL/corps/bindings et assertions du scénario JS selon plan auteur.

Commandes futures nommées : quatre suites Python du contrat plus `test_envoy_upstream_server_identity.py` après delta de renderer/certs. Tests déjà annoncés par auteur : deux rendererOK après un échec d'assertiontransport et un helperAPIabsent ; trois fixtureOK sanssocket. Helperabsent n'est pas un ROUGE fonctionnel. Les traces seront recoupées au livrable. Aucun moteur/réseaucluster exécuté. La valeur d'autorité du profil final est en réexamen architecte ; les tranches statiques indépendantes seules continuent.
