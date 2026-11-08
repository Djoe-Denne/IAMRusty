# IAM → OAuthConnect v2 — plan QA de lot

Date : 2026-11-08. Statut : **plan avant implémentation, aucune exécution ni preuve RED/GREEN**.

Réconciliation parent après cadrage architecte : `docs/contracts/iam-oauthconnect-v2.md` §§4/5/6 prévaut pour les coutures. Q-2 est tranchée : bindings valides d'une ligne legacy/corrompue → writer committe consumed_at et purge nonce/verifier, puis domaine refuse sans capacité ni complete. Bindings invalides restent intacts, expirées au cleanup. Q-1 RELINK reste comportement existant sans garde métier ajouté ; résultat conflit réel à caractériser et, si rouge/ambigu, bilan groupé utilisateur. G-5 réel Connect conditionné à la couture existante, aucune extension manifeste/harness implicite. Baseline transport parent : 11 PASS,0 échec,0 ignoré,124 filtrés par module ; pas preuve v2.

**Choix d'exécution approuvé par l'utilisateur** : extension bornée de l'allowlist du lanceur existant aux sept binaires OAuth supplémentaires ci-dessous, sans changement de garde ni nouveau harness. Cette décision remplace l'alternative suite IAM entière décrite historiquement aux étapes4–5 : ne pas lancer cette suite, car sqs_event_routing_tests active SQS sans ignore. Test PowerShell/AST sélection étendu : RED avant extension, GREEN après ; régression threads GREEN, aucun runtime exécuté.

Commandes IT finales retenues : `& .\ops\tests\it-native.ps1 -Suite iam -TestBinary <binaire>` pour les dix binaires `auth_transactions_postgres`, `oauth_browser_transactions`, `oauth_cleanup_lifecycle`, `auth_oauth_start`, `auth_oauth_callback`, `auth_oauth_provider_failures`, `relink_provider`, `verified_idp_https`, `organization_account_guard`, `app_cleanup_supervision`. Réutiliser les preuves non affectées ; compiler tous consommateurs mais aucune campagne queues.

Ajout explicite approuvé ensuite par l'utilisateur : **`request_trace_redaction`**, onzième binaire du même lanceur gardé (directement adapté aux routes v2). TestselectionPS enrichi d'abordRED,puisallowlistuniqueajout→GREEN ; threadsregressionGREEN ;aucunruntimepourceschecks. Les autres gardes restent inchangées, pas de campagne IAM entière ni queues.

Complément parent reçu après rédaction : baseline `cargo test --locked -p iam-domain --lib` **53 PASS,0 fail,0 ignored,0 filtered**, log `.tmp/iam-oauthconnect-v2/baseline-domain.log`. Résultat rapporté par le parent, non réexécuté par le planner ; témoin de base, pas preuve de raccord v2. Aucun runtime Docker observé/démarré par le parent à ce préflight ; slot Cargo libéré après cette baseline.

## Base, contrat et périmètre

- HEADbase observé : `b028c2fdc213eee26f3e009392301955e920ce5f`, checkout canonique `C:/Users/djden/source/repos/AIForAll`, arbre propre au relevé initial. Serena voit cette racine ; lecture des sources courantes, pas un worktree divergent. Le ledger absolu `.cursor/handoffs/20261108-iam-oauthconnect-v2.md` cite encore `e8952d8` au §Fait : le relevé Git et la base fournie par le parent priment pour ce plan, sans réécriture du ledger.
- Mandat : TDD comportemental, couverture large dont cas biscornus, exécution directe du parent ; **ni Ensemble, ni PM, ni configuration de verification inventée**. Pas de quota de tests ou de chaîne automatique de reviewers. Allocation par modes de défaillance, sans score de routage manuel.
- Canon : ADR0413 **Accepted / Partial**, `docs/contracts/oauthconnect-multi-instances.md` §§3/5–7, ADR0200–0202. Le cadrage IAM que l'architecte prépare indépendamment est une dépendance documentaire : **le parent synchronise ses signatures, décisions et points de test avant de lancer les auteurs concernés**. Ne pas attendre/poller cet autre travail ; signaler une contradiction, ne pas choisir une règle métier en QA.
- Uniquement raccord IAM LOGIN/LINK/RELINK, nonce durable, migration additive, HTTP v2, orchestration/browser/fixtures/registry. UUID, slugs, règles email et stockage des tokens inchangés. Pas de retrait legacy, changement de BASE_URL/config déployée, SDK/pin, migration de comptes, nouveau harness, JWT validator ou E2E fournisseur.
- Connect livré : 295 PASS rapportés dans le contexte parent, non réexécutés ici. Ne pas refaire les 61 cas OIDC : éprouver leur **frontière de refus et absence d'effets IAM**, avec une preuve de provenance Connect réelle ciblée.

### Réconciliation sans doublonner l'ancien plan

Ce fichier est le plan opérationnel de la **tranche IAM**. `.cursor/qa-plans/20261106-oauthconnect-tdd.md` reste historique du lot Connect ; aucune modification de cet autre fichier n'est autorisée ici.

- Ancien G-5 IAM → G-1/2/3/5/6/7/8 ci-dessous ; ancien G-7 navigateur/mint → G-2/3/8/9 ; partie **client IAM** de G-6 HMAC/PKCE → G-4/9. Ces références désignent des obligations reprises, pas deux campagnes à produire.
- Anciens G-1…4 et partie serveur de G-6 : acquis Connect à ne pas réauditer. G-8 ancien, E2E GitHub/GitLab : reste externe, repris en G-10. Les identifiants `G-n` ci-dessous sont locaux à **ce fichier**.
- La suite transport/secrets et le plan OIDC du 2026-11-07 ne sont pas des preuves IAM exécutées. Un témoin v1 ne devient pas une preuve v2 par proximité de nom.

## Risques et allocation utile

Impact et probabilité sont distingués qualitativement ; l'absence de couverture augmente l'incertitude de détection, pas la gravité. Pas de matrice numérique décorative.

1. **Prioritaire — capacité writer, non-rejeu et purge (G-1/2/3).** Impact : double authentification/retargeting ou secret durable récupérable. Probabilité : changement transversal de DTO, SQL et trois consommateurs ; concurrence réelle. Déclencheurs : callback concurrent, commit/UPDATE en échec, nonce legacy absent, mismatch de binding. Rayon : une transaction et les comptes auxquels elle pourrait donner accès. Détection actuelle : barrière Postgres, replay et bindings v1. Mitigation actuelle : SELECT FOR UPDATE, commit avant capacité, purge verifier. **Gap : aucun nonce durable ni oracle double-purge/complete v2 ; étendre ces témoins avant de multiplier les scénarios métier.**
2. **Prioritaire — validation externe avant mutation IAM (G-4/5/6/7/8).** Impact : confusion de compte, écrasement de lien/token ou session émise malgré refus Connect. Probabilité : remplacement de deux appels v1 par un résultat unique, branches LOGIN/LINK/RELINK et règles email différentes. Déclencheurs : complete refusé/partiel, mauvais slug, changement email/sub ou mauvais contexte. Rayon : utilisateur ciblé, utilisateur déjà lié et propriétaire d'un email. Détection actuelle : IT callbacks/relink, pas la provenance v2. Mitigation : capacité liée et règles métier actuelles. **Gap : zéro effet métier après refus v2, conservation des anciens tokens et identité instance+ID à démontrer.**
3. **Prioritaire — transport signé et borné (G-4/9).** Impact : fuite de credentials/nonce ou ressources non bornées ; HMAC sur mauvaise cible. Probabilité : nouvelles routes et corps ; `signed_post` courant finit par `response.json()` non borné. Déclencheurs : redirect, timeout, body volumineux/invalide, mauvaise signature. Rayon : requête sortante et diagnostics IAM. Détection : unités PKCE/HMAC et HTTPS v1. Mitigation : TLS vérifié, timeout10s et redirects désactivés déjà construits. **Gap : v2 complet, limite1MiB et erreurs sans secrets à tester sur le vrai adapter.**
4. **Prioritaire — upgrade non destructif (G-3).** Impact : perte d'états/comptes ou déploiement incompatible. Probabilité : le migrator courant ne contient que le schéma initial ; risque de faux vert sur DB neuve. Déclencheur : upgrade d'une base avec transactions en vol. Détection : schéma frais seulement pour cette nouvelle colonne. Mitigation : contrat additive/nullable et migration avant binaire. **Gap : fixture pré-upgrade avec lignes conservées et callback legacy refusé, sans reset.**
5. **Régression ciblée — autorité mint, cleanup et consommateurs compilés (G-8/9).** Impact : session sous mauvaise autorité, boucle cleanup cassée, appelant oublié. Probabilité : structs transactionnels partagés et fixtures largement importées. Mitigation : tests existants nommés ci-dessous. **Gap : réexécution sur diff intégré et transposition v2, pas extension à toute la crypto/FGA/queues.**

Passe heuristique : états BEGIN→persisté→authorize→consommé/purgé→complete→métier ; frontières TTL/nullable/1MiB ; ordre commit avant I/O ; idempotence callback et absence de retry ; deux listeners distincts sur le même writer, pas deux clones d'URL. L'échec complete ne remet **pas** la transaction consommée à disposition.

## Inventaire de preuves sources (pas de PASS d'exécution)

Tous les chemins suivants sont sous `services/IAMRusty/`, sauf mention contraire.

- **COVERED-source, sous-invariant v1 seulement** : `tests/auth_transactions_postgres.rs:e3_consume_uses_writer_lock_one_committed_capability_and_erases_db_verifier`. Corps lu : verrou détenu via writer indépendant, deux consommateurs bloqués, un seul résultat, lecture SQL après retour montrant consumed+verifier NULL, replay refusé ; ligne expirée non consommée. Cela prouve l'intent de verrou/commit existant, **pas** nonce ni callback v2. `tests/support/writer_barrier.rs` est la couture à réutiliser, pas des sleeps probabilistes.
- **COVERED-source v1, à transposer** : `tests/oauth_browser_transactions.rs:browser_binding_provider_intention_expiry_and_replay_fail_before_connector`, `stored_target_and_redirect_mismatch_deny_but_query_cannot_retarget_consumed_capability` et `writer_consume_failure_is_unavailable_and_has_no_connector_call_or_consumption`. Corps lus : compteurs sortants, cible SQL, erreur503 sanitizée, trigger de panne réel puis reprise après rollback.
- **Preuve périmée pour v2, non à supprimer** : `tests/oauth_browser_transactions.rs:replica_race_consumes_the_browser_transaction_once` compte actuellement **2** appels token/profile et vérifie seulement verifier NULL ; v2 exige **1 complete**, zéro token/profile legacy et double-purge. Garder deux listeners distincts et la preuve DB. Ne pas conserver « 2 appels » comme oracle magique.
- **COVERED-source transport v1 uniquement** : `tests/verified_idp_https.rs:verified_root_and_ip_san_allow_real_hmac_authorize_s256_exchange_and_profile`, corps lu ; racine/SAN vérifiés, HMAC et3 appels v1. Les témoins `wrong_ca_denies_before_any_http_handler_despite_matching_server_ip_san`, `wrong_san_denies_before_http_even_with_the_exact_leaf_signing_ca_trusted` et `verified_https_redirect_does_not_forward_hmac_or_body_to_another_trusted_tls_endpoint` sont localisés, leur couverture v2 reste **unassessed**. Réutiliser `tests/support/verified_idp_tls.rs`.
- **Localisés, unassessed avant exécution/lecture des assertions utiles** : `tests/auth_oauth_callback.rs`, `auth_oauth_start.rs`, `auth_oauth_provider_failures.rs`, `relink_provider.rs`, `oauth_cleanup_lifecycle.rs`, `app_cleanup_supervision.rs`, `organization_account_guard.rs`. Ce dernier contient `published_organization_signer_with_victim_sub_cannot_me_link_or_relink` ; le test de mint à conserver est `oauth_browser_transactions.rs:oauth_registration_completion_then_second_login_uses_configured_authority`.
- Unités actuelles `domain/src/entity/oauth_transaction.rs:tests` couvrent bindings/expiry et `infra/src/repository/oauth_transaction_write.rs:tests` SQL/commit avec double de DB : utiles en complément, **ne remplacent pas** Postgres réel pour locking/rollback/upgrade.
- `tests/fixtures/idp_connect/service.rs:IdpConnectMockService` expose reset/received_requests et mocks v1 ; `hmac_ok` vérifie déjà le PATH reçu et les octets du corps. Une réponse canned complete prouve le comportement IAM, pas la validation OIDC. Préserver isolation/reset et `#[serial]` des fixtures partagées ; éviter qu'un mock permissif masque un refus.
- `tests/common.rs:IAMRustyTestDescriptor` : DB=true, SQS=false, OpenFGA=false, `Migrator::up` réel. Pas de nouvelle file ou fake OpenFGA. Si une variante exige `has_openfga()==true`, OpenFGA réel selon ADR0201.

Inventaire inverse : transposer les compteurs/DTO/chemins v1 des tests du parcours nouveau ; conserver les tests legacy qui ont encore un contrat distinct ; ne pas créer un doublon de toutes les règles OIDC ou crypto IAM. Tout nouveau cas rapporte le symbole réellement exécuté, pas seulement un nom de fichier.

## Registre borné des gaps et placements

Les obligations ci-dessous sont **explicites** (contrat et mandat), sauf les questions Q de fin de document. `GAP` signifie preuve v2 manquante dans ce relevé, **pas test déjà rouge**. Les noms de nouveaux tests restent à fournir par leur auteur après compilation. Efforts relatifs : court < moyen < conséquent, sans engagement calendaire.

### G-1 — nonce indépendant, durable avant START [GAP, prioritaire ; moyen]

- Placement/niveau : unités pures `iam-domain`, `domain/src/entity/oauth_transaction.rs`, puis extension browser Postgres G-2. Pas de DB mock comme preuve de durabilité.
- Scénarios : BEGIN LOGIN/LINK/RELINK avec et sans PKCE selon config ; nonce32octets base64url sans padding, exposé à authorize après create réussi ; panne create → aucun authorize ni contexte de succès. TTL invalide/redirect vide refusent ; bornes TTL≤600s avec temps contrôlé ou marges stables existantes.
- **Oracle** : décodage nonce=32octets, alphabet/padding et aller-retour exact stockage→authorize→expected_nonce ; verifier absent d'authorize, challenge S256 attendu si requis, nonce présent indépendamment du PKCE. Writer témoin en unité vérifie ordre ; DB réelle l'observe au moment de l'appel sortant.
- Indépendance nonce≠state/cookie/verifier/challenge : lecture du chemin de génération OsRng et sources distinctes ; vérifier non-dérivation et rôles séparés, sans statistique d'entropie, tirages massifs ni assertion fragile « tous les aléas doivent différer ». Fixtures à valeurs fixes distinctes pour les bindings. Aucun nonce recréé au callback.

### G-2 — consommation unique, commit/purge et bindings [GAP v2 ; conséquent]

- Placement : `iam-service --test auth_transactions_postgres` et `--test oauth_browser_transactions`, vrais writer/DB/HTTP, barrière writer existante.
- Nominal : après consume retourné, un autre writer voit consumed_at nonNULL et **authentication_nonce=NULL, pkce_verifier=NULL**, tandis que le consommateur détient les valeurs originales pour **un** complete. Vérifier aussi cette visibilité depuis le répondeur complete avant d'autoriser sa réponse, sans nouveau framework.
- Négatifs : state invalide/absent, cookie absent/autre, provider, intention LOGIN/LINK/RELINK, cible, redirect exact, expiry et replay ; zéro complete et aucune mutation métier. Les tentatives de binding invalides ne brûlent pas une transaction valide ; paramètres query ne retargetent pas une capacité.
- Concurrence : deux consommateurs sous lock puis deux callbacks sur listeners distincts, un gagnant ; une seule capacité, un complete, double-purge persistée. Aucun verrou de transaction maintenu pendant le réseau. Commencer par les scénarios existants plutôt que fabriquer une course non reproductible.
- Panne UPDATE/commit : couture de panne réelle existante, zéro complete,503 générique ; rollback laisse consommé=false et secrets d'origine, puis tentative valide possible après retrait de la panne. Une assertion SQL mocked « commit appelé » ne prouve pas cela. Si le commit ne peut être mis en défaut avec les coutures autorisées, noter la limite spécifique au lieu de dire couvert.
- **Oracle** : compteurs par endpoint v2, sélection SQL par id/state_hash précis (pas compteur global sensible à d'autres tests), réponses métier et journal des étapes ; zéro endpoint legacy. Vérifier absence d'I/O jusqu'au commit, pas seulement état final.

### G-3 — migration upgrade/legacy et cleanup [GAP ; moyen]

- Placement : extension `auth_transactions_postgres`/`oauth_cleanup_lifecycle`, migrations réelles `iam-migration` et DB fixture existante. Nouveau module de migration additive enregistré dans `migration/src/lib.rs` ; **ne pas éditer** `m20220101_000001_initial_schema.rs`.
- Nominal : fixture DB isolée migrée au schéma précédent, anciennes lignes login/link/relink actives/expirées/consommées + sentinelles users/provider_tokens/user_emails ; appliquer seulement l'upgrade. Comparer valeurs précédentes et comptes, vérifier nouvelle colonne nullable=NULL sur anciennes lignes, indexes/contraintes utiles conservés. Nouveau BEGIN persiste nonce et survit à une lecture indépendante.
- Négatif : ancienne ligne/state authentique sans nonce → callback refuse et demande un nouveau START, zéro complete ni nonce synthétique ; un START nouveau fonctionne. Expiry/cleanup purge encore les lignes éligibles, conserve les valides. Le traitement interne consumed/non-consumed d'une ligne legacy refusée doit suivre le cadrage IAM synchronisé ; ne pas inventer cet oracle SQL.
- **Oracle** : snapshots ciblés avant/après, historique migrator, information_schema nullable, valeurs legacy identiques ; aucune réinitialisation DB pour masquer une incompatibilité. Une DB fraîche avec nonce=NULL inséré ensuite prouve le refus legacy mais **pas** l'upgrade : garder les deux intents. Rollback opérationnel garde la colonne nullable ; pas de `down` destructif imposé.

### G-4 — adapter HTTP v2 signé et borné [GAP ; moyen]

- Placement : `iam-infra --lib auth::http_connector::tests`, WireMock HTTP sortant local ; puis transport TLS existant en G-9. Méthodes v2 du vrai `HttpIdpConnector`, pas fake du usecase.
- Nominal authorize : POST `/oauth-connect/<instance>/v2/authorize`, corps redirect/state/nonce/challenge+S256 exacts, sans verifier ni slug JSON ; complete : code/redirect/verifier/expected_nonce exacts, résultat tokens+profil intègre. Changer deux slugs pour détecter un PATH tronqué ou base mal concaténée ; HMAC vérifié sur méthode+**PATH complet reçu**+timestamp+octets bruts réels, jamais seulement sur DTO reconstruit.
- Négatifs : contexte nonce/PKCE invalide avant I/O, signature/body/PATH altérés dans le témoin de vérification ; absence de méthode v2 legacy-default → refus, zéro appel v1.401,403/5xx, JSON invalide, réponse tokens sans profil/profil mal typé → erreur typée sanitizée, pas succès partiel ni `user_profile` de rattrapage.
- Transport : redirection refusée sans forwarding HMAC/body, timeout réel borné et **zéro retry** complete, corps>1MiB refusé avant parsing non borné ; témoin valide à la borne si JSON valide. Couvrir authorize et complete via helper commun testé, dont réponse chunked sans Content-Length si couture existante ; ne pas conclure mémoire/RSS bornée du seul code d'erreur. Budget d'erreur générique sans inclusion de corps distant ou source reqwest sensible.
- **Oracle** : reçus du serveur et compteurs exacts, HMAC vérifié par le mécanisme contrat existant, DTO contractuel attendu écrit indépendamment de la réponse réelle. Timeout actuel10s : marge de scheduling bornée, ne pas dépendre d'une égalité milliseconde ni introduire un builder de test qui diverge de prod. Limite1MiB à synchroniser avec le cadrage IAM ; le `response.json()` courant n'est pas une preuve de plafond.

### G-5 — refus complete avant effets IAM [GAP ; moyen]

- Placement : IT callback/login/link/relink sur serveur IAM réel + Postgres, fixture Connect sortante adaptée. Au moins un nominal par intention avec résultat valide ; refus générique complete appliqué aux trois intentions.
- Négatifs : échec HTTP/timeout/DTO incomplet ; Connect refuse un résultat OIDC, puis le même callback est rejoué. Vérifier absence de user, lien, email, refresh/registration/session/JWT additionnels ; anciennes valeurs provider_tokens inchangées pour LINK/RELINK. Pas de fallback exchange/profile, pas second complete au replay.
- **Oracle** : snapshots ciblés des utilisateurs impliqués et des tables métier avant/après, réponse sans credential, compteurs v2/legacy. La seule mutation attendue en échec externe est la consommation/purge transactionnelle **déjà commitée**. N'exiger ni restauration du nonce ni rollback métier global pour une panne ultérieure hors validation.
- Provenance ciblée (après intégration) : vrai adapter IAM→router/listener **Connect réel**, vendor WireMock avec un jeton signé valide puis un nonce/claim invalide via les jeux OIDC déjà livrés ; refus Connect empêche tous effets IAM. Un succès réel OAuth peut compléter ce raccord si les fixtures existantes le permettent. Pas réécriture de61 scénarios ; mock S2S seul n'est pas une preuve cryptographique. Si la couture de composition manque, signaler le gap au parent, ne pas créer un second harness.

### G-6 — LOGIN : UUID et email inchangés [GAP v2 ; moyen]

- Placement : IT `auth_oauth_callback` + `oauth_browser_transactions`, fixtures DB réelles et complete sortant ; unités domaine en complément si déjà structurées.
- Nominaux : identité instance+ID déjà liée et email changé → même UUID, même si l'email appartient à un autre compte ; nouveau provider ID + email existant vérifié → rapprochement avec UUID existant (primaire et email secondaire) ; nouvelle identité/mail → compte **incomplet**, token de registration/refresh selon parcours actuel, puis completion et second login sous autorité configurée.
- Négatifs : email absent **même déjà lié** → refus sans stockage de nouveaux tokens ; email existant non vérifié côté provider → pas de rapprochement ni mutation. Ajouter témoin linked+email présent+verified=false : le lien existant reste prioritaire, ne pas appliquer une nouvelle politique de matching.
- **Oracle** : UUID `users.id`, couple provider/provider_user_id et user_id, emails inchangés ou ajout contractuel, contenu des tokens conservé au succès seulement ; JWT final sub=UUID IAM, issuer/audience configurés. Ne pas remplacer UUID par sub/email ; ne pas inventer une exigence de vérification locale du mail existant à la place du drapeau provider.

### G-7 — LINK/RELINK et namespace d'instance [GAP v2 ; moyen]

- Placement : `auth_oauth_callback`, `relink_provider`, browser transactions ; services métier réels derrière ports sortants mockés HTTP seulement.
- LINK nominal sans email autorisé ; autre email inexistant ajouté selon règle actuelle, pas promotion silencieuse en email vérifié. Identité déjà liée au même/autre compte et email d'un autre compte → refus selon règles existantes, anciens tokens inchangés.
- RELINK nominal : provider déjà lié, résultat complete validé, remplacement de ses tokens sans changement UUID ; email absent admis. Provider non lié, cible/intention erronée, complete refusé → pas de remplacement. **Ne pas copier le refus email LINK sur RELINK** : `ProviderLinkService::handle_provider_email_for_relink` ignore actuellement un email appartenant à un autre user, sans le réaffecter. Geler ce comportement tant que le contrat ne le change pas.
- Instance : même ID externe sur deux slugs distincts + emails distincts pour isoler l'oracle → liens distincts ; même slug/ID depuis autre replica → UUID stable. Utiliser id opaque (dont zéros initiaux) ; aucune normalisation numérique dans IAM.
- **Oracle** : liens/tokens complets avant/après, user_id exact et absence de mutation chez l'autre propriétaire. RELINK changeant d'identité externe vers une identité liée ailleurs est la question Q-1 de fin : **ne pas inventer une règle de conflit ou une réparation ici**.

### G-8 — orchestration/browser/registry et consommateurs [GAP v2 ; court à moyen]

- Placement : `application/src/usecase/{oauth,link_provider}.rs`, `http/src/oauth_browser.rs`, `setup/src/app.rs` et IT start/callback/relink. Tous les chemins utilisent authorize_authentication/complete_authorization et le même résultat validé ; aucune branche provider-specific ajoutée.
- Nominaux : LOGIN/LINK/RELINK, PKCE requis ou non selon config existante, slugs inchangés ; session plateforme requise pour START link/relink, callback sans bearer autorisé par cookie/state/capacité. Deux onglets même cookie → transactions distinctes consommables et nonce attendu propre.
- Négatifs : aucun client enregistré/slug inconnu, bearer d'organisation avec sub victime, state d'une intention vers une autre, contexte query injecté ; erreurs et absence d'effets préservées. Pas de nouvelle négociation v1/v2 ou URL de déploiement changée pour faire passer une fixture.
- **Oracle** : méthodes sortantes et endpoint reçus, bound target/redirect, scope/URL contractuels, réponse HTTP actuelle. Réexécuter `organization_account_guard` et `app_cleanup_supervision` après intégration des consommateurs ; modifier leurs données de construction pour le nouveau champ si nécessaire sans affaiblir leurs assertions.

### G-9 — secrets, TLS et régressions transversales ciblées [GAP v2 ; moyen]

- Placement : unités Debug/modèle dans domain/infra, tests HTTP sortants, `verified_idp_https`, `oauth_cleanup_lifecycle`, browser mint.
- Nominal : même chaîne de CA/SAN vérifiés et HMAC pour v2 authorize+complete ; nonce/verifier ne survivent pas au stockage consommé ; cleanup rejoint proprement ses tâches et conserve les transactions valides.
- Négatifs : CA fausse/SAN faux avant handler, redirect vers second endpoint sans transfert ; sentinelles fixes code/token/verifier/nonce/secret HMAC et message DB ne figurent pas dans Debug/erreurs HTTP/logs capturables. Types sans Debug/Serialize : revue de surface, pas test compilant volontairement un appel interdit. `BegunOAuthTransaction` portant désormais nonce ne doit pas hériter d'un Debug divulguant celui-ci.
- **Oracle** : reçus TLS réels et zéro handler en refus, capture existante cleanup/logs, recherche des sentinelles dans sortie effectivement capturée, corps d'erreur générique. Distinguer absence dans une capture de garantie absolue sur tous les logs. Garder mint configuré et supervision ; pas nouvelle campagne globale de certificats/crypto/FGA.

### G-10 — preuves externes et hors lot [GAP-defer externe]

- E2E GitHub/GitLab réels : autre intervenant, avant bascule/retrait legacy ; pas gate cachée bloquant les unités/IT locales. Parent remet instance/slug/issuer, ancien lien canonique, UUID avant/après, BEGIN→complete sans replay et limites de campagne.
- GitLab : comparaison sur **le même compte réel**, `sub == ancien GitLabUser.id.to_string()` ; fixture fabriquée ne prouve pas cette équivalence. Divergence : stop cutover et décision humaine/architecte, pas migration improvisée ni rapprochement email.
- BASE_URL/registry de déploiement, Compose/Kind, SDK et retrait legacy : hors mandat (**GAP-accept pour ce lot local**, pas déclaration de readiness déployée). Aucun runtime démarré par le planner.

## Ordre TDD et commandes candidates réellement localisées

Commandes **non exécutées**, à lancer par le parent sur Windows, une à la fois ; jobs24 et target `E:\cargo-target\AIForAll` fournis par `.cargo/config.toml`, sans override. Vérifier nombre de tests sélectionnés : filtre sans test ne vaut pas vert.

1. Après synchronisation du cadrage : auteurs unités domaine et HTTP WireMock peuvent travailler indépendamment ; signatures v2 déjà présentes ou socle refusant **compilable** d'abord. RED = assertion comportementale qui échoue pour la raison attendue, témoin nominal/setup valide ; API/colonne absente, erreur compilateur, fixture cassée ou environnement indisponible = **blocked**, pas RED. Exemple RED possible sur base : refus v2 du client IAM malgré requête valide, si les méthodes du trait sont disponibles ; le test nouveau nonce attend le socle si sa surface manque.
2. Après API transactionnelle intégrée : adapter doubles/writer/constructeurs, puis auteurs upgrade/DB. Après consumers intégrés : browser et l'ensemble IAM tests. Pas de compilation contre deux branches non intégrées ; pas d'auteur lancé sur un fichier non disponible depuis sa base.
3. Recettes ciblées :

```powershell
cargo test --locked -p iam-domain --lib
cargo test --locked -p iam-infra --lib auth::http_connector::tests
cargo test --locked -p iam-application --lib
cargo check --locked -p iam-domain -p iam-application -p iam-infra -p iam-http_server -p iam-setup -p iam-migration -p iam-service --all-targets
```

Le `--lib` transport vise les tests locaux déjà structurés, pas un lancement Postgres ; vérifier qu'un futur test ajouté n'y démarre pas transitivement une ressource non préflightée.

4. **Phase IT finale après intégration des sources**, seulement après préflight parent lifecycle/test-evidence et inspection des créations/remplacements transitifs. Information de préflight parent, sans nouvelle enquête planner : `ops/tests/it-native.ps1` lit registry + Engine witness, trace les fixtures, stoppe gracieusement les IDs possédés et retire leurs Exited(0). Sa sélection `-Suite iam -TestBinary` accepte **uniquement** les trois binaires ci-dessous (lignes29–38) :

```powershell
& .\ops\tests\it-native.ps1 -Suite iam -TestBinary auth_transactions_postgres
& .\ops\tests\it-native.ps1 -Suite iam -TestBinary oauth_browser_transactions
& .\ops\tests\it-native.ps1 -Suite iam -TestBinary oauth_cleanup_lifecycle
```

5. Pour les autres cibles vérifiées (`auth_oauth_start`, `auth_oauth_callback`, `auth_oauth_provider_failures`, `relink_provider`, `verified_idp_https`, `app_cleanup_supervision`, `organization_account_guard`), **ne pas inventer des valeurs TestBinary supportées** ni lancer des IT brutes en contournant la garde. Option recommandée si portée utile et capacité confirmée : suite IAM existante finale, après adaptation de tous consommateurs :

```powershell
& .\ops\tests\it-native.ps1 -Suite iam
```

Sans TestBinary, le wrapper lance `cargo test -p iam-service --tests` (préflight parent lignes281–292). C'est potentiellement large, avec dépendances/fixtures transitives : ne pas le qualifier de check statique ni activer kafka/SQS par commodité. Alternative **soumise à arbitrage préalable du parent** : méthode fixture-guard existante bornée, prouvant le même ownership par IDs + registry + trace/stop/cleanup ; ses commandes exactes ne sont pas établies par ce plan. Pas d'élargissement du harness, nouveau publisher ou garde maison pour obtenir une sélection. Un test TLS purement loopback peut rester sans Docker si son chemin réel le garantit ; son placement dans iam-service ne dispense pas de cette vérification parent. Les RED natifs domaine/HTTP loopback restent possibles avant tout Docker.

**Gap d'exécution associé G-5…9** : tant que ni suite IAM existante ni alternative guard existante ne sont retenues/exécutées, les preuves de ces autres binaires restent GAP/blocked, pas couvertes par les trois runs autorisés. Reporter sélection/cfg, tests ignorés/non exécutés et coûts du choix au bilan ; éviter une réexécution redondante si la suite complète couvre déjà les trois binaires.
6. Gates statiques sur diff intégré, parent responsable des éventuels warnings préexistants :

```powershell
cargo fmt --all -- --check
cargo clippy --locked -p iam-domain -p iam-application -p iam-infra -p iam-http_server -p iam-setup -p iam-migration -p iam-service --all-targets -- -D warnings
cargo check --locked -p oodhive-monolith --all-targets
```

`oodhive-monolith` est vérifié dans `runtime/monolith/Cargo.toml` et dépend des crates IAM : check final approprié, sans exécuter/déployer le monolithe. Adapter la commande de fmt au périmètre si des fichiers étrangers préexistants échouent, sans les modifier silencieusement. Recettes candidates, pas promesse qu'elles passent.

## Portes d'entrée/sortie, budget et revue

**Entrée par tranche** : cadrage IAM synchronisé par parent, contrat des API/hook existants décidé ; build ciblé compilable, fixtures et sentinelles disponibles ; bail Cargo exclusif ; pour IT, préflight et bail infra/IDs tenus par parent. Fixtures DB doivent préserver upgrade et legacy ; queues off par défaut. Auteur tests séparé optionnel, pas de pipeline automatique.

**Sortie du lot local** : G-1…9 prouvés au niveau approprié ou limites/échecs précisément arbitrés ; RED puis GREEN attachés aux symboles/commandes/base réellement exécutés quand RED comportemental possible ; régressions intégrées focalisées + compilation des consommateurs + fmt/clippy pertinents ; aucun PASS global si cas contractuel bloque ou suite non exécutée. Constats nouveaux/ambigus regroupés ci-dessous avant décision, pas suppression pour quota. G-10 reste externe explicitement, donc pas de déclaration de bascule/E2E live.

Chaque preuve consigne sélection/nombre de tests, code de sortie, oracle observé et limites. Une erreur initiale de build n'est pas requalifiée en RED après correction. Les tests retenus doivent échouer sous une mutation plausible (suppression de purge nonce, appel legacy, acceptation complete sans profil, mauvais provider/cible) ; réflexion de revue, pas nouveau chantier d'outillage mutation.

**Budget indicatif à confirmer par le parent** : setup/fixtures/données20%, nouvelles preuves focalisées40%, régressions/intégration/revue15%, **buffer25% nommé** pour courses, upgrade et fixtures/TLS. Total planifié75%. Si capacité insuffisante, réduire doublons de scénarios/protocoles déjà acquis Connect et recommander découpe du lot ; ne pas sacrifier buffer ni preuve non-replay/refus sans effet pour garder des cas moyens. Pas de délai garanti.

**Revue indépendante proposée, non chaîne imposée** : correctness ciblée sur ordre/UUID/email/RELINK et security ciblée sur capacité non-rejouable/HMAC/secrets, diff intégré + contrat, bornées G-1…9. Un profil peut suffire si le parent couvre l'autre angle ; aucun quatuor obligatoire. Findings bloquants : violation contractuelle/authz/non-replay ; suggestions hors lot différables. Nouvelle revue/correction ne découle pas automatiquement de tout finding.

**Ressources** : planner zéro Cargo/IT/Docker/Kind/WSL, zéro création donc aucune ligne de ressource nouvelle à enregistrer. Parent : registre contemporain des IDs réels, cleanup fixture en succès/échec, arrêt gracieux possédé et vérification avant/après ; zéro conteneur possédé running en fin de campagne, aucun prune/suppression hors autorisation. Pas de démarrage Kind pour ces IT.

## Registre final groupé — rares/ambigus rouges et décisions utilisateur

Revue TEST indépendante9d10 : PASSstatiqueborné,**T-1MEDIUMretenu** (2témoinsG7email à ajouteravantIT:RELINKignoreemailétranger/LINKajouteemailnonvérifié),pasbugproddémontré ;**T-2LOWdifféréGAP-accept** (streaming sansCL/timeoutpendantcorpsnonqualifiés,Wiremockcorpsfixe,preuvecompensatricesourcechunkreader+reqwest). Aucun nouveauharness imposé,aucuneITPASSparlecture. Report `.cursor/review-briefings/20261108T1237Z-tests-iam-v2-oracles-9d10e3b.md`,parentpersisteavanttestcorrections.

GREEN intermédiaire orchestration complète : **87domain+30application PASS**, sans échec/ignoré/filtré ; les14cas LINK/RELINK passent après6RED/8PASS. Clippy strict domaine+application+migration passe, fmt passe. Onze binaires IT (les10retenus et request_trace_redaction directement adapté) compilent, **aucun encore exécuté**. Browser/fixtures v2 intégrés ; correction de syntaxe par l'auteur avant compilation, aucun oracle métier changé. Revue sécurité producteurs+LOGIN sans finding ; delta final LINK/helper doit encore être revu.

RED intermédiaire LINK/RELINK : **8PASS/6FAIL/0ignoré/16filtrés** après compilation. Quatre START±PKCE et deux refus complete constatent v2absent ; huit gardes opération/cible/provider/redirect passent avec les deux clients effectivement enregistrés. Ces échecs contractuels ne sont pas des rouges rares à supprimer.

GREEN intermédiaire LOGIN : **87tests domaine PASS/0échec/0ignoré/0filtré**, dont les4oracles LOGIN précédemment rouges, fmt et Clippy strict all-targets passent. LINK/RELINK et navigateur ne sont pas encore qualifiés ; la façade de types v2 est intégrée et compile pour permettre leurs témoins sans dépendance nouvelle.

RED intermédiaire LOGIN orchestration : **0PASS/4FAIL/0 ignoré/83 filtrés**, compilation réussie. Deux START (±PKCE) et deux callbacks (complete refusé/email absent) constatent zéro appel v2 au lieu d'un, sans erreur de montage stricte repository/JWT. LINK/RELINK restent à préparer après accès aux types du port en application ; cette absence de préparation n'est pas un test RED.

GREEN intermédiaire domaine/helper/writer : **83 tests domaine PASS** (8 nouveaux de forwarding/refus), **4 unités writer PASS** ; aucun ignoré, filtres writer142. Migration additive et IT Postgres compilent ; Clippy strict domaine+migration passe après import explicite du réexport `async_trait`. Les IT réelles n'ont pas encore tourné et l'orchestration utilise encore v1 : **raccord global non terminé**.

RED intermédiaire writer : unités SQL compilent, **3 PASS/1 FAIL/0 ignoré/142 filtrés** ; le consume ne retourne pas encore le nonce original. Les nouveaux tests Postgres/upgrade compilent (`auth_transactions_postgres --no-run`) mais **aucune IT n'a tourné**. Le schéma de préimage est isolé dans une transaction fixture avec search_path sans public, pas un reset ; preuve DDL transactionnelle à distinguer d'un rollout opérationnel.

GREEN intermédiaire domaine : **75 PASS/0 FAIL/0 ignoré/0 filtré**, Clippy domaine all-targets strict et fmt passent. Les22casnonce passent sur les mêmes oracles ; tirageOsRng distinct vérifié dans le delta. Nouvelle API complete du contexte privé intégrée mais forwarding et suppression finale des usages v1 restent à prouver ; SQL réel toujours non exécuté.

État TDD intermédiaire nonce : **22 nouveaux cas compilent ;9 PASS/13 FAIL/0 ignoré/53 filtrés**. Six BEGIN échouent sur nonce vide, sept legacy/corrompus échouent sur capacité accordée malgré writer témoin déjà consommé/purgé. Ce sont des écarts contractuels à corriger, pas des cas métier nouveaux ; SQL réel, migration et ordre réseau restent à prouver par leurs tests dédiés.

GREEN intermédiaire transport : **22 PASS/0 FAIL/0 ignoré/124 filtrés** (11v2+11v1), timeout réel10s inclus, mêmes oracles. Clippy strict infra **rouge sur 9 dead_code préexistants/non modifiés**, dont huit comparés au log baseline ; détail `.cursor/review-briefings/20261108-iam-http-green-quality.md`. Ne pas confondre cette dette avec un test rare rouge ou annoncer un PASS global. Aucun allow/affaiblissement appliqué.

État TDD intermédiaire transport (avant implémentation, pas bilan final rare) : 11 tests v2 compilent ; **3 PASS/8 FAIL/0 ignoré/135 filtrés**. Les nominaux échouent par refus v2 default, les négatifs réseau par absence de reçu. Cela qualifie l'absence de raccord v2, **pas encore les erreurs de réponse, le timeout ou la borne 1MiB**. Les trois pré-I/O peuvent passer par refus default et seront réévalués après GREEN. Les oracles ne sont pas modifiés pour les faire passer.

**Règle prioritaire de restitution** : ne supprimer aucun test rare/ambigu, ne changer aucun oracle et ne coder aucune règle métier spontanément pour verdir. À la fin, compléter **dans ce fichier** un bilan groupé par intérêt fonctionnel, pas une avalanche de failures. Séparer : (A) oracle contractuel obligatoire en échec, à corriger ou explicitement laisser bloquant ; (B) question nouvelle sans règle ratifiée, décision utilisateur avant réparation/suppression. Aucun rouge observé par ce planner.

Pour chaque groupe restant : G/Q concernés et symboles/commande/base ; intérêt utilisateur et scénario ; attendu + source du contrat **ou hypothèse** ; observé exact ; impact de réparation (règle changée, données/tokens touchés, compatibilité) ; impact de suppression (risque qui perd sa preuve) ; preuves et limites/setup ; décision demandée, propriétaire et échéance. Ne jamais écrire « exotic PASS » ou clôturer pour atteindre un quota.

### Groupe A — contractuels non négociables sans arbitrage explicite

- Consommation unique/commit/double-purge et zéro I/O sur mauvais binding ou panne writer (G-1/2/3) ; refus complete sans effets IAM ni fallback (G-4/5) ; UUID/email/namespace et autorité existants (G-6/7/8) ; transport et non-fuite (G-4/9). État initial : **GAP, aucun run**, pas des RED déjà connus. Si rouge, consigner l'écart ; supprimer le témoin supprimerait une preuve de sécurité/compatibilité, pas la contrainte.

### Groupe B — questions ouvertes, pas spécifications ajoutées par QA

- **Q-1 — RELINK avec changement de provider_user_id / identité déjà liée ailleurs.** Fait source : `domain/src/service/provider_link_service.rs:relink_provider_for_user` vérifie qu'un provider est déjà lié puis sauvegarde le nouvel ID ; pas de contrôle de conflit analogue à LINK dans ce corps. Conséquences exactes des contraintes DB/du repository non évaluées ici. Intérêt : empêcher confusion/vol de lien sans changer implicitement une capacité de relinking existante. Attendu : décision contrat IAM à confirmer par parent/architecte puis utilisateur si règle nouvelle ; **pas « tout conflit doit refuser » inventé**. Observé : lecture du corps seulement, aucun test exécuté. Réparer peut restreindre le relinking et ses erreurs ; supprimer un test de caractérisation perdrait la détection de cette différence. À présenter groupé si le cas reste rouge/ambigu.
- **Q-2 — legacy refusé : tranché avant auteurs par contrat IAM §4.** Bindings valides → consommation/purge commitée puis refus sans capacité/complete ; aucune fabrication de nonce ni rollback de capacité après erreur réseau. Une garantie supplémentaire d'annulation réseau n'est pas ajoutée. Les assertions précédentes demandant synchronisation sont désormais résolues par cette règle, pas des ambiguïtés reportées à l'utilisateur.
- **Q-3 — preuve live GitLab.** Inconnue externe, pas défaut du code prouvé : équivalence sub/ancien ID. Effet d'une réparation locale improvisée : identité/UUID potentiellement changés, hors lot. Effet d'ignorer la preuve : cutover non qualifié. Responsable : intervenant E2E/parent avant bascule ; aucun faux vert de fixture.

## Handoff / limites du relevé

Localisation : fichiers/symboles cités vérifiés par Serena et matching exact ; manifests réels confirmés. Impact : transaction/migration, adapter, trois parcours, fixtures et consommateurs IAM/monolithe. Limites : pas d'audit exhaustif, pas de test exécuté, pas de preuve de commit-failure injecté ni d'équivalence GitLab live ; signatures finales et cas RELINK à synchroniser avec le cadrage architecte. Le parent intègre/commite ce fichier avant auteurs isolés si besoin ; ce planner ne committe ni ledger ni code.
