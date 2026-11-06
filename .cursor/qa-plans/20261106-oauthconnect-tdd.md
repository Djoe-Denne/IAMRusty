# OAuthConnect — plan de lot TDD RED/GREEN

## Première tranche vérifiée — contrat serveur v2 (2026-11-06)

- Frontière `idp-connect-contract` seulement : types/méthodes, deux routes, HMAC par instance, nonce/PKCE, forwarding et erreurs génériques ; aucun provider ni parcours IAM nouveau qualifié.
- Preuve TDD : 16 cas exécutés sur le socle, 5 témoins verts / 11 échecs d'assertion404, puis 16/16 verts sur l'implémentation sans modifier les attentes. Les ajouts T-1/T-2/T-3 couvrent502générique, absencePKCE et GETsigné405 ; T-4 (allowlistredirect) reste à la tranche service.
- Régressions15/15 tests lib existants, check sans `server`, fmt et Clippy strict all-targets verts. Quatre lints de tests corrigés par l'auteur, sans allow ni changement d'oracle ; les16cas corrigés ont été réexécutés.
- Logs locaux et chronologie : `.tmp/oauthconnect-tdd/` et ledger parent. Zéro test ignoré/filtré dans les runs retenus, aucun Docker/Kind/E2E. Les autres tranches et M-1/M-2 restent ouvertes ; le cadrage historique ci-dessous n'atteste aucune exécution de ces autres tranches.

## État et périmètre

- Cadrage du 2026-11-06, **aucun test/Cargo/runtime exécuté**. Aucun PASS de la refonte revendiqué.
- HEADbase revérifié : `master@b39c6c4dcc6de9b7f123c00c9bd168aad08eca74`, sous-module rustycog `93b890ff7b821e9cb2e7d0aeb81957d098778a2f`. Arbre suivi propre lors de l'inspection.
- Entrées : `.cursor/handoffs/20261106-oauthconnect-tdd.md` et `.cursor/handoffs/20261106-lucid-auth-provider-evaluation.md`, corrections IAM comprises. AGENTS et skills `aiforall-mcp-workflow`, `serena`, `rustycog`, `creating-wiremock-fixtures` consultés.
- Travail direct de feuille, sans mission Ensemble, PM, score ou pipeline imposé. Seul ce fichier est écrit. Première version indépendante ; réconciliation après livraison architecture terminée, sans lecture pendant son écriture.
- Réconciliation du 2026-11-06 : ADR `docs/adr/0413-oauthconnect-multi-instances-profils.md` et contrat `docs/contracts/oauthconnect-multi-instances.md` intégrés dans le checkout canonique après livraison isolée. **AP-1 et AP-2 acquises par acceptation humaine explicite du 2026-11-06** (« Valider v2 et nonce », « Email primaire seul »), source `.cursor/handoffs/20261106-oauthconnect-tdd.md`, section Décisions tranchées. ADR désormais Accepted / Unimplemented et contrat actif ; publication parent avant auteurs. Implémentation et preuves de tests de la refonte restent non acquises.
- Cible explicite : un service RustyCog, catalogue au boot, profils OIDC standard/GitLab et OAuth configurable/GitHub. Chaque claim OAuth `sub`, `email`, `email_verified`, `preferred_username` est décrit par API GET JSON + JSONPath ; résultat typé, unique, réponse réutilisée dans une opération, pas de scripts/chaining.
- Préserver IAM UUID, slugs d'instances, règles LOGIN/LINK actuelles, transactions navigateur/PKCE, HMAC, transport vérifié, redirects exacts et acquisition stricte des secrets. Claims riches et actions déléguées **différés**, pas des critères de sortie de ce lot.
- E2E GitHub/GitLab : autre intervenant, voir handoff final ; aucune écriture/exécution E2E ni démarrage Kind/Docker ici.

## Canon, décisions acceptées et gates restantes

Faits : ADR 0200/0201 Accepted imposent IT réelles aux frontières internes et WireMock pour HTTP sortant seulement ; 0202 Accepted/Partial impose queues opt-in. ADR 0407–0409, 0411–0412 sont Accepted/Implemented dans leurs en-têtes. L'acceptation humaine AP-1 autorise désormais l'évolution partielle de 0408/0411 sur ce périmètre ; sa transcription documentaire par architecte et son intégration parent restent distinctes du code. ADR 0505 reste Proposed, malgré la réalité du chemin secrets GitHub ; ne pas la promouvoir par le plan QA.

Résolution de cadrage par le **contrat accepté sur ce périmètre via AP-1/AP-2 le 2026-11-06**, ratification documentaire intégrée, sans preuve d'implémentation :

1. **A-1 — résolu-accepté (§§3.1,5)** : POST `/oauth-connect/{instance}/v2/authorize` et `/v2/complete`, ce dernier retourne uniquement `ProviderAuthentication { tokens, profile }`. Méthodes `authorize_authentication` / `complete_authorization`, défaut legacy refusant, aucun fallback v2→v1. ID token reste dans Connect. IAM génère un nonce indépendant de 32 octets base64url sans padding ; champ transactionnel nullable `authentication_nonce`, lu puis purgé atomiquement avec verifier par le consommateur writer. Présent uniformément pour OAuth/OIDC dans le S2S ; seul OIDC le transmet/valide auprès du vendor. Nouvelle migration additive, pas cache de corrélation.
2. **A-2 — résolu-accepté (§§2.3–2.4)** : `jsonpath-rust = "=1.0.12"`, AST du parser contrôlé, chemins localisés pour appairer les objets email/verified ; expressions ≤512 octets, profondeur ≤16, pas descente récursive/regex/fonctions. `sub` OAuth string non vide ou u64 exact, `sub` OIDC string inchangée ; cardinalité sub=1, autres=0/1, doublons même égaux refusés. Null optionnel=absence ; email_verified absent/null=false et sans email=false ; username absent=`""` sans fallback ; picture optionnel conserve avatar_url sans téléchargement. Mauvais type n'est pas absence. UserInfo borné selon T4, pas récupération générique de claims.
3. **A-3 — delta accepté explicitement via AP-2 (§2.3)** : GitHub primaire seul depuis `/user/emails`, zéro primaire=email absent, deux primaires=erreur ; plus de fallback `/user.email`, plus d'erreurs emails avalées. Les mappings email/verified utilisent même API et même objet localisé ; aucune jointure/script/branche vendor pour rétablir l'ancien fallback. LOGIN sans email continue de refuser, LINK peut réussir. Ne pas déclarer cette évolution équivalente à l'ancien profil.
4. **A-4 — résolu-accepté (§§1,3,6)** : prefix unique `/oauth-connect`, sous-route et clé HMAC par instance, PATH complet, inconnue/non canonique=404, mauvaise signature=401. Crates cibles `oauth-connect-{configuration,domain,infra,http,setup}`, binaire `oauth-connect-service`. Slugs conservés et issuer épinglé indépendant des replicas/clusters. Récepteur avant consommateur, migration SQL avant IAM, drainage des START v1 au moins TTL 600 s ou refus avec nouveau START ; retrait ancien code seulement après transposition et preuve E2E. Ports/SAN/configs de cutover restent à vérifier opérationnellement, pas à qualifier par un test unitaire.
5. **A-5 — cadrage accepté, preuves restantes (§§2.1,4–6)** : vrai loader `OAUTH_CONNECT`, liste non vide, direct/OpenBao explicites/exclusifs par instance, acquisition avant registry complet, aucune instance amputée ; aucun réseau dans le loader. HTTP redirect=none, timeout total10 s/connexion3 s, lecture décompressée≤1 MiB pour token/profil/discovery/JWKS ; erreurs de collaborateur 502 générique existant, sans nouvelle enveloppe. Équivalence `sub` GitLab réel/ancien ID demeure **gate de bascule externe**, pas résolue par le contrat. Overrides env de tableaux et APIs/AST sous Windows doivent être confirmés techniquement, pas supposés.

Gates d'approbation **acquises le 2026-11-06 par l'utilisateur** : **AP-1** « Valider v2 et nonce », **AP-2** « Email primaire seul », source ledger décisions §3, ligne5. Ratification documentaire architecte puis intégration/publication du contrat/plan par le parent avant premier auteur RED. Gates de bascule **non acquises** : **M-1** preuve E2E externe de compatibilité GitLab et des deux parcours, **M-2** cutover/rollback/TLS et inventaire des consommateurs validés avant retrait/bascule. M-1/M-2 **ne bloquent pas le démarrage unités/IT TDD sur le contrat intégré**, seulement la bascule/retrait. Aucun choix structurel supplémentaire par le QA planner.

Aucune contradiction substantielle empêchant la préparation des tests n'est identifiée entre ces deux documents. Les limites techniques JSONPath/loader encore à confirmer sont des préconditions de T0, pas une autorisation de changer la DSL ou le SDK. La contradiction Compose HTTP/HTTPS déjà signalée par l'architecte reste une gate de migration ; aucune preuve HTTPS de bout en bout n'est déduite des ports. AP-1/AP-2 acquises permettent de commencer RED puis GREEN après intégration des entrées ; aucune implémentation ni exécution n'est attestée par cette acceptation.

## Allocation guidée par les défaillances, sans scoring

Trois risques dominent :

- **Mauvaise association de compte** : sub réinterprété, slug confondu avec profil, email non vérifié utilisé pour fusionner. Portée : comptes existants, sessions, liens. Protection actuelle : clé provider+ID, UUID local, règles IAM. Détection actuelle : IT link/conflict ; **gap** : LOGIN verified/non-verified/absent et équivalence GitLab non démontrés. Concentrer les assertions DB/UUID ici.
- **Identité externe non validée ou mélange entre instances** : nouveau profil OIDC ou réponse/cache réutilisé hors opération. Portée : usurpation, credentials d'un utilisateur/issuer servis à un autre. Protection actuelle : HMAC/transport, pas de preuve de la nouvelle validation OIDC. **Gap** : signature/issuer/audience/expiry, nonce OIDC obligatoire dans le parcours accepté, isolation des réponses/keys et mapping typé. Tester les refus autant que le nominal.
- **Dégradation fail-open au regroupement** : loader permissif, secret d'une autre instance, PATH HMAC incomplet, PKCE perdu, redirects élargis. Portée : toutes les instances du nouveau service, fuite de secrets ou reprise d'un flux invalide. Protections/tests actuels ci-dessous. **Gap** : les mêmes assertions doivent traverser la nouvelle composition et ses chemins, pas seulement l'ancien adapter.

Passe heuristique : état boot invalide→aucun service utilisable ; pending→consumed avant échange ; callbacks concurrents→un seul gagnant ; ordre identité avant rapprochement email ; replay→refus ; cache réponse limité à une opération/instance/token ; redémarrage ou autre replica→même identité logique, pas état navigateur en mémoire.

## Inventaire borné des preuves existantes

`COVERED-source` ci-dessous signifie intent et assertions inspectés, **pas exécution ni preuve de compatibilité OAuthConnect**. Les autres symboles seulement repérés restent `unassessed` jusqu'à lecture/run focalisés par l'auteur.

- **COVERED-source HMAC historique** : `services/GitHubConnect/tests/hmac_contract.rs:valid_hmac_exchanges_token_and_profile` vérifie tokens + ID `12345`, username/email/verified ; `hmac_path_must_include_service_prefix` refuse la signature du PATH nu et accepte le PATH préfixé. Équivalents GitLab repérés. `crates/idp-connect-contract/src/hmac.rs:tests` repère fenêtre 30 s, corps muté, mauvaise clé, Debug redacted ; `server.rs:tests` repère refus PKCE malformé/ancien adapter. Réutiliser, ne pas dupliquer partout.
- **COVERED-source link, pas LOGIN email** : `services/IAMRusty/tests/auth_oauth_callback.rs:test_oauth_callback_associates_new_provider_for_same_user` démarre un LINK explicite ; assertions un user, deux provider_tokens, objets antérieurs inchangés. `test_oauth_callback_prevents_linking_provider_already_bound_to_another_user` (409) attesté dans le ledger antérieur ; ne pas le vendre comme preuve du rapprochement LOGIN par email.
- **COVERED-source transactions/PKCE v1** : `services/IAMRusty/tests/oauth_browser_transactions.rs:pkce_s256_survives_connector_authorize_and_token_exchange` compare le challenge navigateur/S2S au hash du verifier transmis à l'échange. `replica_race_consumes_the_browser_transaction_once` crée deux listeners distincts, attend 202/400, une transaction consommée et verifier effacé. Son compteur `exchange_count == 2` compte token+profile v1, pas deux échanges de code : en v2 compter exactement un `complete` S2S et un POST token vendor, avec nonce/verifier purgés, pas conserver mécaniquement le compteur 2.
- **COVERED-source secrets stricts** : `services/GitHubConnect/configuration/tests/strict_secrets.rs:real_prefixed_loader_is_declarative_strict_and_never_falls_back` appelle le vrai loader/env, rejette références/mode invalides malgré des valeurs directes valides et vérifie l'absence des sentinelles dans erreurs/logs. `services/GitHubConnect/tests/openbao_startup.rs:resolved_secrets_reach_oauth_and_exact_iam_hmac_without_refresh` vérifie vraie composition, credentials OAuth résolus, clé HMAC exacte, redaction et aucune requête vault métier après boot. Acquisition failures/direct conflicts repérés dans ce même binaire.
- **COVERED-source non-forwarding TLS** : `services/IAMRusty/tests/verified_idp_https.rs:verified_https_redirect_does_not_forward_hmac_or_body_to_another_trusted_tls_endpoint` atteint une vraie origine TLS signée puis prouve zéro appel à une destination pourtant approuvée. Positif CA/SAN et refus mauvaise CA/SAN repérés dans ce fichier.
- **unassessed, à réutiliser selon diff** : `services/IAMRusty/tests/auth_oauth_provider_failures.rs` (400/401/403/429/5xx sans persistence métier), `auth_oauth_start.rs` (admission), autres cas navigateur (binding/intention/expiry/replay/writer down/relink), et `services/{GitHubConnect,GitLabConnect}/infra/src/{github,gitlab}.rs:transport_tests` (TLS explicite, scopes, PKCE, verifier).

Fixtures vérifiées : `GitHubConnect/tests/fixtures/github/service.rs` expose token/user/emails/reset ; GitLab équivalent expose token/user, **pas encore une preuve de discovery/JWKS OIDC**. IAM `tests/fixtures/idp_connect/service.rs` est le collaborateur S2S sortant ; `tests/common.rs` fournit serveur IAM réel, DB/migrations, replicas ; `has_sqs=false`, `has_openfga=false`. Le routeur Connect existant est exercé via `Application::new` + `axum_test::TestServer` : test de composition HTTP sans DB, pas E2E réseau/mesh. Garder cette distinction, ne pas ajouter Postgres/FGA/events au connecteur stateless.

Lecture inverse : les assertions métier de l'ancien GitLab `/api/v4/user` deviennent preuve périmée pour OIDC ; conserver leurs données utiles de migration, pas leur statut de couverture. Les anciens tests HMAC de deux services doivent être transposés à la composition unique avant retrait des anciens packages. Le test redirect actuel ne vérifie que `status != 200` : renforcer au code/erreur décidé et zéro requête vendor, plutôt que dupliquer cet oracle faible. Pas d'audit global des tests historiques.

## Tranches et registre de gaps TDD

Ordre recommandé après AP-1/AP-2 : **T0 → frontière v2/HMAC/PKCE/nonce de T6 → T1 → T2 → T3 → T4 → T5**, puis intégration/non-régression T6/G-7. La dépendance v2 précède les nouveaux profils et les deux chemins IAM, conformément au §6 ; le numéro T6 est conservé pour la continuité des gaps. Les caractérisations IAM de T5 peuvent précéder la refonte si elles ne changent aucun contrat. Pas de GREEN commun massif avant les RED de chaque tranche.

### T0 — rendre un RED exécutable, établir les témoins

- Vérifier l'approbation puis l'intégration du contrat, les noms Cargo/targets, la présence des APIs et données attendues. Construire les seuls targets sélectionnés sans les exécuter, puis témoins existants nominaux/négatifs. Préflight technique ciblé du pin JSONPath/AST/chemins localisés et des types `openidconnect`4 (réexports oauth2 v5) versus oauth2 v4 du workspace ; ne pas résoudre une erreur de types par bump global. Vérifier de vrais overrides env/tableaux du loader acquis, sans second loader/SDK modifié.
- **Plancher technique explicite** : nouveaux packages et DTO/méthodes v2 déclarés, fixtures/consommateurs legacy encore compilables, entrée de composition et déclarations/rows de migration compilables (sans anticiper leur comportement GREEN). Si package/API manque, compilation impossible = **blocked**, jamais RED métier. Après autorisation, un squelette compilable au résultat v2 explicitement refusant peut rendre l'assertion nominale rouge ; il ne compose pas token+profile v1. Aucun comportement GREEN du profil ne doit précéder son test. Interfaces de mapping approuvées testables sans réseau ; routeur/boot testables par la composition réelle.
- Un test réellement en RED doit : être découvert et exécuté (compteur non nul, pas ignoré), fixtures positives fonctionnelles, atteindre l'assertion du comportement attendu, échouer pour cette divergence identifiable. Un timeout de fixture, port occupé, dépendance absente, mauvais filtre ou erreur Rust n'est pas RED.
- Chaque tranche conserve base/diff test, commande, nom, assertion attendue/observée et statut RED ; après correction minimale, même oracle GREEN et non-régression voisine. Si une caractérisation passe déjà, consigner PASS de caractérisation, sans fabriquer un faux RED. Si le code GREEN existait avant le test, consigner une preuve rétrospective, pas TDD.

### T1 / G-1 — catalogue et configuration boot [explicite, GAP ; effort indicatif 1 séance]

- Niveau : unités du vrai loader/validation + composition boot sous `services/OAuthConnect/{configuration,setup,tests}`, sans dépendances inventées ; packages cibles du contrat à confirmer dans les manifests créés.
- Nominal : deux profils, deux slugs distincts du même profil et troisième slug compatible sans branche vendor ; routes/secrets par entrée. `OAUTH_CONNECT` et tableaux/overrides réellement supportés modifient la bonne entrée ; loader sans I/O, puis setup secrets/discovery/JWKS avant registry complet.
- Négatif : catalogue vide, duplicate case-fold, slug hors lettres/longueur1–50, profil ou clés/fichier/env inconnus, ancien préfixe accidentel, mélange champs OAuth/OIDC, API inexistante/inutilisée, JSONPath invalide/non autorisé ; URL avec credentials/fragment ou paramètre OAuth réservé, transport/secret/référence incomplet, issuer Discovery différent, scopes vides/avec espace/contrôle et capacité S256 OIDC absente. Method auth token explicite et compatible, jamais détection/retry alternatif. Toute instance invalide fait échouer le boot entier, aucun fallback/sous-catalogue/hot-load ; down après boot ne désinscrit pas le slug.
- Oracle : résultat du vrai loader/boot et catalogue effectif, zéro I/O loader et aucune écoute métier avant succès complet ; erreurs RustyCog redacted. Pas une simple désérialisation isolée. RED : config invalide admise ou instance mal résolue ; GREEN : validation/composition minimale. Ni une convention d'index env inventée ni un loader permissif ne ferment G-1.

### T2 / G-2 — projection JSONPath typée [explicite, GAP ; ~1–2 séances]

- Niveau : unités table-driven de la projection réelle, sans HTTP, cible `oauth-connect-infra --lib` ; noms qualifiés finaux à reporter par l'auteur. Sources héritées : `github.rs:map_profile`, `gitlab.rs:map_profile`, `idp-connect-contract/src/dto.rs`.
- Nominal accepté §2.3 : `sub` string non vide préservée exactement ou entier0…u64::MAX rendu en décimal sans perte ; chemins enfant/index et filtres scalaires/logiques admis ; email/boolean/username depuis objets/tableaux, picture optionnel. Absence/null optionnels : email=None, verified=false, username=`""`, avatar=None ; email absent+bool isolé true reste false. Email présent+bool absent=false. Aucun fallback username vers email/sub.
- Négatif : sub0résultat/null/vide, tout claim>1résultat même égal, mauvais type (dont email chaîne vide), entier négatif/fractionnaire/horsu64 et bool string/number ; descente récursive/regex/fonction/code, expression513octets ou profondeur17 refusées (témoins512/16 acceptés si valides). Même API mais parents email/verified différents → erreur ; incohérence structurelle au boot, dépendante du JSON à l'exécution. Email sélectionné non vérifié avec autre adresse vérifiée ne devient pas vérifié.
- Delta GitHub sous AP-2 : primaire seul ; zéro primaire → email absent même si `/user.email` est rempli, deux primaires → erreur ; sous AP-1, IAM refuse ce LOGIN et accepte le LINK autorisé sans email. Pas de jointure/fallback caché. Fixtures valeurs/chemins attendus écrits indépendamment des résultats du code.
- Oracle : profil exact ou erreur existante, chemins parents localisés appariés lorsqu'ils existent, pas seulement même valeur/email ; aucune adresse synthétique, aucun premier élément implicite, aucun téléchargement picture. RED typique : mauvais typage/cardinalité accepté ou ancien fallback maintenu. Oracles du périmètre accepté AP-1/AP-2, à vérifier par les auteurs, pas résultats déjà obtenus.

### T3 / G-3 — GET JSON, réutilisation et confinement [explicite, GAP ; ~1 séance]

- Niveau : adapter réel + WireMock **sortant**, fixture existante étendue, pas moteur de scripts/harness neuf. Ajouter discovery/JWKS comme réponses HTTP de fixture seulement si requis par T4.
- Nominal : deux mappings même définition API → un seul GET authentifié par `complete` ; `/user` et `/user/emails` → un GET chacun ; auth bearer/token et headers déclarés attendus, hôte/config non littéraux. Deux complétions avec tokens/instances distincts obtiennent leurs propres réponses, aucun cache profil inter-opérations. Token POST form+méthode auth configurée, Accept JSON, access_token non vide/token_type Bearer compatible.
- Négatif : 401/403, 429/5xx, non2xx/HTML/JSON invalide et erreurs emails **non avalées** ; timeout total10s/connexion3s, redirect même vers origine autorisée, corps décompressé>1MiB refusé (témoin valide≤1MiB). Mêmes bornes sur token/profil/discovery/JWKS et callback HTTP de la bibliothèque OIDC. Zéro GET token/token en query ; pas de headers personnalisés Authorization/Cookie/Host/HMAC, templates/corps/pagination/chaining. Réponse d'une instance/token jamais servie à l'autre ; pas de retry code après timeout/erreur, pas de tokens partiels si profil échoue.
- Oracle : profil/erreur convenu + journal WireMock filtré par méthode/path/header et nombre exact par opération ; zéro token/secret transmis au Location. Ne pas vérifier seulement un compteur global susceptible de confondre appels métier et discovery/JWKS. Réutilisation positive ET isolation négative doivent échouer avant le GREEN correspondant.

### T4 / G-4 — OIDC minimal valide et migration GitLab [AP-1 acquise ; preuve GitLab M-1 restante ; ~2 séances]

- Niveau : validation réelle + HTTP discovery/JWKS/token/UserInfo si le contrat les utilise, WireMock sortant et tokens signés de test ; jamais mocker la fonction de validation ni accepter un simple JWT décodé.
- Nominal accepté §2.4 : Discovery issuer exact et URLs admises par origines configurées ; `openid profile email` GitLab, Code+S256, ID token RS256/signature vérifiée, aud=client_id seul, azp absent ou identique, exp futur, iat requis≤maintenant+60s, nbf cohérent si présent, nonce attendu et at_hash correct si présent. `sub` chaîne non vide **octet pour octet**, même avec zéros initiaux ; jamais conversion numérique OIDC. Même autorité sur autre app/replica sans corrélation mémoire.
- Négatif : signature fausse/none/HS/autre algorithme, sélection kid ambiguë, issuer différent, audience absente/étrangère/supplémentaire ou azp différent, exp==maintenant/passé, iat absent/>+60s ou incohérence temporelle/nbf futur, ID token/sub/nonce manquant ou invalide, at_hash faux. Endpoints hors origines et jku/x5u/iss reçus ne choisissent pas une autre confiance. Deux instances avec même kid/sub et issuers distincts ne partagent pas leurs clés. Chaque refus empêche retour de tokens/profil et effets métier IAM, pas la consommation writer déjà commitée.
- **UserInfo borné** : un seul GET uniquement si email absent de l'ID token validé ET endpoint découvert autorisé ; email présent mais username/picture manquant → zéro GET ; pas d'endpoint → profil incomplet. sub égal obligatoire, claims non-null contradictoires refusés, seuls absents complétés ; mauvais type refusé plutôt que complété. Email et verified viennent de la même source, verified manquant=false ; pas de confirmed_at/fallback vendor. Erreur HTTP/JSON de ce GET→erreur, pas succès dégradé.
- **JWKS technique** : TTL≤300s, refresh au plus une fois/30s/instance, single-flight ; kid inconnu déclenche au plus ce refresh puis refus s'il reste absent ; expiration sans refresh réussi→fail closed ; rotation récupérable avec clé publiée valide. Horloge/ordonnancement contrôlés via moyens de tests existants, assertions aux bornes et reçu HTTP ; ne pas copier la politique access JWT IAM ni introduire un nouveau harness de cache.
- Oracle : `complete` retourne seulement tokens+profil validé exact ou 502 générique existant ; aucun ID token/raw_claims/handle exposé à IAM, aucune donnée sensible en diagnostic. Jeux de tokens **signés**/horloge bornée, pas contournement de vérification ni fixture « chiffrée » (JWE est exclu). Une erreur de clé/fixture est du setup invalide, pas le RED de signature.
- Migration : comparer un ancien provider_token GitLab (slug + ID exact) à un nouveau profil OIDC du **même compte externe** ; oracle UUID/lien préservés. Un JSON fabriqué avec `sub == ancien id` prouve l'adapter, **pas l'équivalence réelle chez GitLab**. L'intervenant E2E doit confirmer cette dernière sur l'issuer cible avant retrait de l'ancien chemin. Divergence → décision de migration, pas fallback email.

### T5 / G-5 — compatibilité LOGIN/LINK IAM [explicite, GAP borné ; ~1–2 séances]

- Niveau : unités domaine et IT HTTP IAM existantes, vrai serveur/DB/migrations ; fixture `idp_connect` adaptée authorize+complete comme HTTP sortant. Puis au moins un raccordement avec la composition Connect réelle et collaborateurs vendor WireMock : le mock S2S IAM seul ne prouve pas l'interopérabilité. Exercer routeur puis listener réseau avec les helpers existants, sans nouveau harness ni mock du usecase Connect.
- Transaction v2 : migration **additive** conserve lignes/états existants et ajoute `authentication_nonce` nullable ; BEGIN génère/persiste nonce32octets base64url sans padding avant authorize, distinct state/cookie/challenge, aucun verifier divulgué au START. Consommation writer récupère nonce/verifier puis purge les deux dans le même commit avant `complete`. Ancienne transaction sans nonce, callback invalid/provider/intention/cookie/target/expiry et panne writer → zéro complete, pas fabrication de nonce ni fallback v1. Contrôler LOGIN et LINK/RELINK, pas un seul consommateur. Migration seule ne réécrit pas users/provider_tokens/emails ; pas de reset de données pour verdir.
- LOGIN nominal : identité provider+ID déjà liée et email modifié → UUID inchangé, priorité du lien sur une adresse appartenant à un autre compte ; identité non liée + email existant + provider verified → même UUID, lien ajouté. Nouveau mail/identité → compte incomplet selon contrat actuel, pas substitution UUID par sub/email.
- LOGIN négatif : email absent **même pour identité liée** → refus actuel ; email existant + provider non vérifié → refus, pas fusion. Contrôler que l'email secondaire existant participe aussi au lookup (`user_emails`, pas seulement primaire), sans exiger sa vérification locale à la place de celle du provider.
- LINK nominal/négatif : email absent accepté sur transaction explicite autorisée ; identité liée au même/autre compte refusée selon codes existants ; email appartenant à un autre compte refusé. Ne pas recycler le test LINK existant comme preuve LOGIN.
- Frontière d'instance : même ID externe dans deux slugs/issuers distincts, adresses différentes pour isoler l'oracle → deux liens distincts sans collision automatique ; même instance/ID sur deux replicas → même UUID. Deux profils de type identique ne fusionnent pas leurs slugs.
- Oracle : réponse HTTP/code métier actuel, `users.id`, `provider_tokens(provider,provider_user_id,user_id)`, `user_emails`, absence de credentials/session supplémentaires sur refus ; JWT `sub` final = UUID IAM lorsque l'accès est délivré. Ne pas imposer un rollback global : la transaction navigateur est déjà consommée avant l'appel externe ; la consommation doit rester conforme à 0412 même en échec ultérieur.

### T6 / G-6 — nouvelle route, HMAC, PKCE et secrets [explicite, GAP de transposition ; ~1–2 séances]

- Niveau : unités HMAC/DTO et boot/composition HTTP Connect ; réutiliser `hmac_contract`, `strict_secrets`, `openbao_startup`, `transport_tests`, `verified_idp_https`. IAM navigateur reste IT DB réelle.
- **Première dépendance après T0** : DTO/méthodes v2/récepteur `server` et témoins HMAC/PKCE/nonce avant implémentation des profils. Nominal : POST `/oauth-connect/{slug}/v2/authorize` puis `/v2/complete`, bon secret et PATH complet ; redirects callback ET relink-callback exacts ; nonce S2S32octets base64url sans padding couvert par HMAC, challenge S256 sans verifier au START, verifier seulement complete après consommation. OAuth reçoit le contexte S2S mais n'envoie pas nonce OIDC au vendor ; OIDC transmet exactement ce nonce.
- Négatif : HMAC absent/mauvaise clé/timestamp hors30s/corps nonce-PKCE altéré ; signature autre instance ou PATH nu ; bearer JWT seul ; nonce malformé, champs PKCE désappariés/invalides et capacité absente → refus avant I/O, jamais v2→v1. Inconnue/noncanonique=404, mauvaise signature=401 ; redirect proche mais non exact et erreurs collaborateur=502 générique selon handlers. Pas `/v2/profile`, alias v1 multi-instance ni autre méthode métier exposée. Aucun essai de toutes les clés ; zéro appel vendor sur refus pré-adapter. Ancien adapter défaut v2 refuse explicitement plutôt que composer v1.
- Secrets : vrai loader nouveau prefix/env, OpenBao déclaré valablement acquis une fois au boot et transmis à la bonne instance ; clés HMAC A/B séparées. Échecs acquisition, références/mode inconnus, conflits direct+vault et secret vide/court → pas de boot utilisable/fallback ; sentinelles absentes de Debug/Serialize/erreurs/logs ; pas de refresh vault métier introduit. Les tests OpenBao existants sont HTTP WireMock, **pas preuve d'un serveur OpenBao réel**.
- Transport : HTTPS/CA/SAN valides passent ; mauvaise CA/SAN, URLs avec credentials, HTTP hors exception locale explicite et redirection refusés sans fuite. Le test TLS IAM→Connect existant ne prouve pas à lui seul le nouveau client Connect→vendor : conserver/porter aussi les assertions de `transport_tests` sur cette frontière.
- Oracle : status/code exact du contrat et reçus HTTP (incluant clé/PATH et absence d'I/O au refus), capture redacted ; ne pas recopier l'algorithme du nouveau router dans le calcul d'attendu du test.

### G-7 — invariants navigateur et non-régression de mint [explicite, COVERED-source partiel / unassessed ; effort : réutilisation ciblée]

- Réexécuter `oauth_browser_transactions`, `auth_transactions_postgres`, `oauth_cleanup_lifecycle` après raccordement : nominal inscription puis seconde connexion, link/relink ; négatifs binding cookie, provider/intention/target/redirect/expiry, replay, panne writer ; course deux replicas→un seul complete S2S et token POST vendor, nonce/verifier purgés. Conserver expiry/cleanup des anciennes lignes après migration, pas effacer les fixtures legacy pour réussir. Adapter le décompte v1 token/profile en oracle v2, jamais affaiblir l'unicité métier.
- Historique borné : HMAC PATH nu (tests dédiés), PKCE ancien adapter silencieux (tests contrat/transport), fallback loader/secret (strict_secrets), issuer du mint d'utilisateur complet (contrat `docs/contracts/oauth-domain-token-authority.md`). Dernier cas : garder l'IT `oauth_registration_completion_then_second_login_uses_configured_authority` ; positif issuer/audience configurés, négatif retour à l'autorité legacy détecté, sans rouvrir le moteur de signature IAM.
- Aucun doublon de toute la suite cryptographie/refresh/FGA hors diff ; élargir seulement si le contrat change effectivement ces frontières et le signaler au parent.

### G-8 — preuve E2E GitHub/GitLab [explicite, GAP-defer externe]

- Autre intervenant propriétaire ; aucun scénario E2E écrit/exécuté par ce lot. Le parent suit réception de cette preuve avant de déclarer la compatibilité bout-en-bout/migration achevée.
- Voir entrées/oracles ci-dessous. Une IT verte ne ferme pas ce gap ; une indisponibilité provider/consentement n'est ni RED métier ni PASS E2E.

## Commandes natives Windows — recettes vérifiées, NON exécutées

Depuis `C:\Users\djden\source\repos\AIForAll`, **une commande Cargo à la fois**, 24 jobs via `.cargo/config.toml` sans `-j`/override ; target Windows `E:\cargo-target\AIForAll`. Pas de build/test dans Docker. Les anciens packages ci-dessous proviennent des manifests inspectés ; les noms OAuthConnect sont les cibles du contrat accepté, pas des packages existants à HEADbase. Cette réconciliation n'a exécuté aucune de ces commandes.

Préparer un target précis puis un cas découvert ; `--no-run` n'est qu'un prérequis technique, jamais RED :

```powershell
cargo test --locked -p idp-connect-contract --features server --lib --no-run
cargo test --locked -p idp-connect-contract --features server --lib
cargo test --locked -p github-connect-configuration --test strict_secrets
cargo test --locked -p github-connect-infra --lib transport_tests
cargo test --locked -p gitlab-connect-infra --lib transport_tests
cargo test --locked -p iam-infra --lib auth::http_connector::tests
```

Composition HTTP existante, WireMock sortant, sans lancer les applications dans Docker :

```powershell
cargo test --locked -p github-connect-service --test hmac_contract
cargo test --locked -p gitlab-connect-service --test hmac_contract
cargo test --locked -p github-connect-service --test openbao_startup
cargo test --locked -p iam-service --test verified_idp_https
```

IT IAM focalisées, **seulement après bail et préflight lifecycle/evidence**. Commandes Cargo exactes des targets actuels :

```powershell
cargo test --locked -p iam-service --test auth_oauth_start
cargo test --locked -p iam-service --test auth_oauth_callback
cargo test --locked -p iam-service --test auth_oauth_provider_failures
```

Ces commandes directes requièrent la capture/cleanup de campagne existante coordonnée par le parent ; ne pas les lancer nues en prétendant que les IDs sont automatiquement possédés. `ops/tests/it-native.ps1` n'autorise actuellement en sélection IAM nommée que `oauth_browser_transactions`, `auth_transactions_postgres`, `oauth_cleanup_lifecycle` : ne pas inventer `-TestBinary auth_oauth_callback` ni une suite `oauthconnect`. Recette réellement disponible pour la non-régression navigateur :

```powershell
powershell.exe -NoProfile -File .\ops\tests\it-native.ps1 -Suite iam -TestBinary oauth_browser_transactions
powershell.exe -NoProfile -File .\ops\tests\it-native.ps1 -Suite iam -TestBinary auth_transactions_postgres
powershell.exe -NoProfile -File .\ops\tests\it-native.ps1 -Suite iam -TestBinary oauth_cleanup_lifecycle
```

Si le parent ne peut satisfaire le protocole de preuve/teardown pour les autres targets focalisés, les marquer **blocked** et arbitrer l'exécution avec les moyens existants ; aucune extension du runner ni campagne IAM intégrale imposée par ce plan. Garder les nouvelles caractérisations navigateur dans ce binaire lorsque cohérent, pas uniquement pour contourner une restriction de runner.

**Commandes futures, AP-1/AP-2 acquises ; intégration du contrat et existence des manifests/targets encore requises** : unités G-1 dans `configuration`, invariants domaine dans `domain`, projection/GET/OIDC dans `infra` ; frontière/composition G-6 dans `services/OAuthConnect/tests/v2_contract.rs` (nom de target proposé par ce plan, pas fichier déjà présent), portage du test secrets sous `tests/openbao_startup.rs`. Réutiliser les fixtures et assertions des anciens services, pas un nouveau harness. Compléter dans ce même fichier les noms qualifiés réellement écrits par les auteurs.

```powershell
cargo test --locked -p oauth-connect-configuration --lib --no-run
cargo test --locked -p oauth-connect-domain --lib --no-run
cargo test --locked -p oauth-connect-infra --lib --no-run
cargo test --locked -p oauth-connect-service --test v2_contract --no-run
# Puis, un target/cas RED a la fois, sans execution concurrente :
cargo test --locked -p oauth-connect-configuration --lib
cargo test --locked -p oauth-connect-domain --lib
cargo test --locked -p oauth-connect-infra --lib
cargo test --locked -p oauth-connect-service --test v2_contract
cargo test --locked -p oauth-connect-service --test openbao_startup
cargo test --locked -p iam-domain --lib entity::oauth_transaction
cargo check --locked -p oauth-connect-service -p iam-service -p oodhive-monolith
```

Le nom `oodhive-monolith` est confirmé dans `runtime/monolith/Cargo.toml` ; son check vérifie le consommateur IAM modifié, pas l'ajout d'un nest Connect (interdit). Préparer le lockfile versionné incluant `jsonpath-rust=1.0.12` et contrôler les dépendances/types avant d'invoquer `--locked` sur les nouveaux targets ; absence de résolution Windows=blocked, jamais RED métier ni raison de bump SDK. Pour un cas unique : `cargo test --locked -p <package confirmé> --test <target confirmé> <nom découvert> -- --exact` ; variante `--lib` avec nom qualifié pour les unités. Vérifier la découverte et le compteur non nul. Les exemples sans filtre servent aux régressions de tranche, pas à une promesse de RED précis.

Avant intégration, fmt/clippy sur les packages réellement conservés/modifiés ; par exemple pour le contrat partagé actuel :

```powershell
cargo fmt -p idp-connect-contract -- --check
cargo clippy --locked -p idp-connect-contract --features server --all-targets
```

Compléter la sélection avec les vrais nouveaux packages et consommateurs IAM touchés. Les anciens packages ne disparaissent des commandes qu'après transposition de leurs obligations. Pas de `--all-features` activant queues/Kafka inutilement.

## Portes d'entrée/sortie et intégration

Entrée avant auteur isolé : AP-1/AP-2 explicitement approuvées, parent a intégré le contrat/plan et rendu leurs versions approuvées accessibles. **Correction vérifiée : ce plan n'est pas gitignored ; `git status` le montre untracked**, sans sélection par `git check-ignore`. Il est destiné à être un artefact durable conservé par le parent. Un untracked n'est pas transmis aux worktrees depuis HEAD ; parent le persiste avant ces auteurs, aucun git add/commit par cette feuille. APIs/fixtures/jeu de données décidés ; compilation ciblée verte, témoin nominal exécutable ; slot Cargo libre, env restaurable et ports des fixtures disponibles.

Avant IT : charger `local-runtime-lifecycle` et `aiforall-test-evidence`, inventorier ressources/baux, tracer les IDs à création et contrôler les remplacements transitifs du harness. DB/migrations IAM réelles ; WireMock sortant seulement ; `#[serial]` pour singleton/env commun ; pas de queue opt-in pour ce lot, aucun OpenFGA ajouté. Aucun runtime pour le cadrage présent.

Sortie de tranche : RED métier conservé quand applicable, GREEN du même cas + nominal/négatif voisins ; aucun assert supprimé/skip ajouté pour réussir ; changements d'oracle seulement après correction de spec explicitement tracée. Chaque gap fermé indique fichier:symbole final, commande, compte exécuté et preuve, pas seulement un fichier existant.

Ordre d'intégration : acceptation/persistance → plancher compilable → RED/GREEN frontière v2 puis profils/boot → migration additive transaction avant IAM v2, adaptation LOGIN **et** LINK/RELINK → raccordement IAM/Connect et suites focalisées/fmt/clippy sur contenu intégré → handoff E2E externe. Avant cutover : récepteur/config/TLS prêts puis consommateur, drainage v1≥600s ou nouveau START explicite, image/config legacy récupérable, aucun retry de transaction consommée. Après M-1/M-2 seulement : retirer anciens services/members/path dependencies et leurs tests déjà transposés ; vérifier inventaire exact des références packages/binaires/configs/CI/docs et ports/SAN sans wildcard d'allowlist navigateur. Aucun apply/build livraison/runtime par ce cadrage. Pas d'exécution IT/E2E concurrente avec Cargo parent. Aucune revue indépendante automatique ; si demandée, la borner à G-1…G-7 et au diff/contrat intégrés.

Sortie de lot auteur : obligations G-1…G-7 vérifiées ou limites/échecs préexistants arbitrés explicitement ; AP-1/AP-2 acquises pour le contenu livré, preuves techniques A-2/A-5 réelles ; zéro ressource possédée encore running hors bail parent actif, inventaire après et cleanup documentés même sur échec. G-8/M-1 et M-2 restent séparés jusqu'à leurs preuves : annoncer « unités/IT validées, E2E/bascule restantes », jamais « refonte compatible E2E » sans elles. Publication/branches/closeout appartiennent au parent, aucune mutation Git par ce planner. À cet état, **seules les approbations AP-1/AP-2 sont acquises ; sorties techniques/tests/E2E restent non vérifiées**.

Budget proposé, sans capacité promise : **20 % setup** (contrat, compilation, données/fixtures, préflight), **55 % tests/RED-GREEN/régression ciblée**, **25 % buffer nommé** (OIDC/ID GitLab, conflits de fixture/loader et investigation d'échecs réels). Efforts de tranches indicatifs en séances de travail, à recalibrer après approbation AP-1/AP-2 et préflight technique T0. Si capacité insuffisante, réduire le lot/migration avec le parent, ne pas supprimer les refus authn ou le buffer pour ajouter des tests secondaires.

## Handoff à l'intervenant E2E externe

Entrées parent : commit intégré et contrats approuvés ; versions IAM/Connect v2 et migration additive, slugs/issuer logique/public base URL exacts ; chemins navigateur inchangés, sous-routes S2S authorize/complete et redirects callback/relink-callback ; configuration GitHub API+JSONPath/delta primaire-seul approuvé et GitLab OIDC `openid profile email`, S256 et nonce durable ; mode secrets/CA expurgé, valeurs uniquement par canaux approuvés ; comptes externes de test, consentements et fixture DB d'un lien GitLab préexistant. Aucun code/token/cookie/secret dans l'artefact de preuve. L'autre intervenant choisit le runtime autorisé et en porte préflight/bail/cleanup.

Oracles attendus, séparément GitHub et GitLab :

- Parcours navigateur réel via routes exactes/Envoy, pas appel direct bypass : nouveau login puis compte complété et nouvelle connexion ; issuer/audience plateforme exacts, JWT final `sub` = UUID IAM, session/refresh persistés selon contrat existant. Distinguer email requis LOGIN d'email optionnel LINK.
- Pour GitLab : ancien lien reconnu par `sub` OIDC réel sur l'issuer cible, même UUID/ID lié sans création de second compte ; comparer hors logs sensibles, sinon stop bascule/escalade, jamais mapping spécial/email forcé. Pour GitHub : primaire seul et verified du même objet ; email non public lorsque le compte de test le permet. Primaire absent doit documenter le delta approuvé : pas `/user.email`, LOGIN refusé, LINK autorisé sans email ; panne emails reste erreur, pas profil silencieusement appauvri. Distinguer fournisseur réel/synthétique et cas indisponibles.
- Link/relink autorisé vers compte existant conserve UUID ; identité liée ailleurs refusée ; replay/absence de cookie ou state et changement d'instance refusés sans échange supplémentaire. START authentifié et callback transactionnel sans Bearer conformément à 0412, pas relaxation générale des routes.
- Si deux replicas/clusters sont disponibles dans le périmètre autorisé : même autorité/identité logique et consommation unique ; sinon limite explicitement déclarée, pas nouvelle topologie exigée pour fermer les unités/IT.

Retour demandé : commit/config cible expurgée, fournisseur/issuer/compte fixture, routes traversées, scénarios exécutés et statuts, UUID/lien avant/après, erreurs/limites, IDs ressources/cleanup. Aucune implémentation/exécution E2E ni ressource Docker créée par cette feuille.
