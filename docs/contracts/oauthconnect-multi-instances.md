# OAuthConnect multi-instances — contrat d'implémentation v1

Date : 2026-11-06. Canon secondaire de [ADR-0413](../adr/0413-oauthconnect-multi-instances-profils.md), **Accepted / Partial**. Base initiale inspectée : `b39c6c4dcc6de9b7f123c00c9bd168aad08eca74`, SDK `93b890ff7b821e9cb2e7d0aeb81957d098778a2f` ; proposition documentaire intégrée à `0a51206b1f9f4a999de7fc4812e947aee0445531`.

**Contrat actif**, ratifié explicitement par l'utilisateur le 2026-11-06 : « Valider v2 et nonce (Recommandé) » et « Email primaire seul ». API authorize+complete, nonce IAM durable indépendant, profil minimal et delta GitHub sans fallback public sont acceptés ; aucune nouvelle capacité de claims riches/actions. La frontière `idp-connect-contract` v2, la configuration déclarative et la projection OAuth pure sont désormais implémentées et testées. Transport, secrets résolus, profils OIDC exécutés, setup/service et parcours IAM restent des cibles, pas des composants livrés.

**Preuve de la première tranche seulement** : 16 tests v2 passés de RED (5 PASS / 11 échecs de routage) à GREEN (16 PASS), 15 tests lib historiques PASS, check sans `server`, fmt et Clippy strict all-targets PASS sous Windows. Les fakes outbound ne qualifient ni les providers ni IAM ; allowlists redirect du service, validation OIDC et nonce durable IAM restent à implémenter/tester dans leurs tranches. E2E et cutover restent externes et ouverts.

**Deuxième tranche vérifiée le 2026-11-07** : configuration38tests (RED23/15→38PASS), projection41oracles (RED18/23→41PASS),3caractérisations lexicales post-GREEN, budget12tests (RED6/6→12PASS) et20unités privées post-implémentation, soit114tests vérifiés sans ignoré/filtré dans les runs complets. Fmt/check/Clippy strict verts sur les trois crates. Le [budget S-1](oauthconnect-jsonpath-budget.md) a été implémenté et recoupé par sécurité, puis ses unités/qualité validées. Aucun réseau fournisseur, secret réellement résolu, DB/IAM ou E2E dans ces preuves ; l'ensemble du boot §2.1 reste partiel.

Après intégration de cette ratification documentaire par le parent, commencer les unités/IT TDD autorisées selon le plan QA réconcilié ne requiert pas d'attendre les E2E externes. Ces preuves et les gates cutover restent requises pour la bascule et le retrait des anciens services ; acceptation du contrat ne signifie ni GREEN, ni livraison, ni autorisation d'activation runtime dans ce lot documentaire.

## 1. Périmètre et invariants

- Un service `services/OAuthConnect/`, crates `oauth-connect-{configuration,domain,infra,http,setup}`, binaire `oauth-connect-service`. Hexagone mince RustyCog ; `setup` construit une fois clients HTTP, profils validés et registry immuable. Default de la crate contrat toujours sans Axum ; feature `server` seulement côté récepteur.
- Pas de DB Connect, événements, queues, OpenFGA, bearer utilisateur, serveur callback navigateur ou nest monolithe. Pas d'adoption lucid-auth, plugin loader ou SDK auth nouveau. Les exceptions 0408 priment sur le gabarit métier et les anciennes mentions HS256 des skills.
- Slugs de 0411 : lettres, longueur 1–50, canonicalisation minuscule au parse config/IAM, doublons après case-fold interdits. Le routing interne n'accepte que la forme canonique. Instance = autorité logique, pas incarnation/Pod/cluster ; le client_id peut changer sous contrôle, l'issuer et les sujets ne se réinterprètent jamais silencieusement sous le même slug.
- IAM conserve comptes UUID, services métier, provider+ID externe et stockage de tokens. Ni email, ni username, ni profil `oauth/oidc` ne remplace cette clé.
- LOGIN : email exigé avant lookup ; lookup provider+ID prioritaire ; rapprochement vers compte email seulement si email_verified provider vrai ; sinon règles actuelles refus/compte incomplet. LINK : intention/cible authentifiées au START et transaction consommée ; absence d'email admise ; identité déjà liée ailleurs refusée. RELINK garde ses règles actuelles, pas celles du LOGIN.
- Aucun changement des routes publiques `/iam/api/auth/{provider}/...`, des conditions cookie/state/transaction/PKCE de 0412, ni des codes 400 invalid_provider / 422 connector_not_configured de 0411.
- Claims riches et actions au nom de l'utilisateur restent différés. Ne pas ajouter `raw_claims`, `extra`, `id_token` au résultat IAM, un paramètre `claims` configurable, refresh/revoke/API déléguée ou nouvel endpoint profil générique.

## 2. Configuration typée, chargée au boot

### 2.1 Forme et validation

`AppConfig` conserve les contrats RustyCog `server`, `logging`, `auth` si nécessaire au constructeur HTTP existant, et `openbao` optionnel. Préfixe environnement du nouveau service : `OAUTH_CONNECT`. `providers: Vec<ProviderConfig>` non vide remplace les sections GitHub/GitLab. `ProviderConfig` contient :

- `id`, `client_id`, `redirect_uris` exacts (Callback **et** Relink), `scopes: Vec<String>` non vides/dédupliqués (chaque entrée = un token sans espace/contrôle, casse préservée, sérialisation séparée par espaces, aucun scope vendor ajouté implicitement), `pkce_supported: bool` explicite ;
- `secrets_mode: direct | openbao` explicitement obligatoire, `client_secret`/`hmac_secret` **ou** `client_secret_ref`/`hmac_secret_ref` exclusifs ; pas de mode par défaut utilisable ;
- `transport_security: verified | local_insecure | isolated_test`, défaut `verified`, et racines CA explicites selon les points d'injection existants ;
- `token_auth_method: client_secret_post | client_secret_basic` explicite, sans détection ni retry sous une autre méthode ;
- `profile`, enum typé à discriminateur `kind = "oauth" | "oidc"`, sans troisième mode ni champs vendor ; ne pas combiner `serde(flatten)` avec une validation d'inconnus qui serait contournée.

Employer le loader strict SDK acquis côté GitHub ; clés/fichiers/env inconnus, ancien préfixe chargé par accident, doublon, mode inconnu, champ requis absent ou configuration de l'autre profil => échec de boot. Le loader ne fait **aucun réseau**. Après validation déclarative, `setup` résout les secrets async puis les métadonnées OIDC et publie le registry complet, ou échoue ; jamais une liste amputée silencieusement. Un fournisseur momentanément down après boot ne disparaît pas du catalogue.

Pas de hot-load : changement de la liste/profil/secret vendor => restart Connect ; catalogue/base_url IAM => restart IAM ; changement HMAC/redirect_uris partagés => coordination et restart des deux. Les tableaux de config et overrides env doivent être prouvés par tests du loader réel ; ne pas supposer une syntaxe d'index env que le SDK n'expose pas, ni ajouter un deuxième loader pour la simuler.

URLs serveur/vendor : absolues, sans credentials ni fragments. HTTPS vérifié par défaut ; un endpoint HTTP requiert l'exception locale/test explicite appropriée, jamais `debug_assertions` ou déduction depuis le hostname. Pas d'URL du vendor fournie par la requête IAM. Les paramètres OAuth réservés dans les endpoints authorize sont refusés plutôt qu'écrasés (`state`, `redirect_uri`, `client_id`, `scope`, `response_type`, PKCE, nonce). Pas de headers/URL secrets dans les erreurs.

### 2.2 Profil OAuth adaptable

Champs : `authorization_url`, `token_url`, `apis: Map<ApiId, JsonGetApi>`, `claims`. `JsonGetApi` fixe une URL GET JSON et `access_token_auth: bearer | token` ; headers non secrets bornés à `Accept`, `User-Agent`, `X-GitHub-Api-Version` (si utilisés). Interdire Authorization/Cookie/Host/HMAC personnalisés, variables, templates, corps et pagination/chaînage.

Chaque claim `sub`, `email`, `email_verified`, `preferred_username` est un **objet** `ClaimSource { api: ApiId, json_path: String }`. `api` référence une définition GET JSON ci-dessus, pas un appel calculé. Ces quatre mappings sont requis dans la config ; leurs valeurs runtime peuvent être absentes sauf `sub`. Mapping `picture` optionnel uniquement pour conserver l'avatar **déjà** prévu par `ProviderUserProfile`, pas pour ouvrir les claims riches. API inexistante ou jamais utilisée => erreur de config.

Extrait cible GitHub (compléter les champs communs et les allowlists, ne pas copier comme fichier prêt à booter) :

```toml
[[providers]]
id = "github"
client_id = "<public-client-id>"
scopes = ["user:email"]
pkce_supported = true
token_auth_method = "client_secret_post"
secrets_mode = "openbao"
# client_secret_ref / hmac_secret_ref : formes SDK actuelles, non reproduites ici.
# redirect_uris : callback et relink-callback publics IAM exacts.

[providers.profile]
kind = "oauth"
authorization_url = "https://github.com/login/oauth/authorize"
token_url = "https://github.com/login/oauth/access_token"
[providers.profile.apis.user]
url = "https://api.github.com/user"
access_token_auth = "bearer"
[providers.profile.apis.emails]
url = "https://api.github.com/user/emails"
access_token_auth = "bearer"
[providers.profile.claims.sub]
api = "user"
json_path = "$.id"
[providers.profile.claims.preferred_username]
api = "user"
json_path = "$.login"
[providers.profile.claims.email]
api = "emails"
json_path = "$[?(@.primary == true)].email"
[providers.profile.claims.email_verified]
api = "emails"
json_path = "$[?(@.primary == true)].verified"
[providers.profile.claims.picture]
api = "user"
json_path = "$.avatar_url"
```

Les headers Accept/User-Agent utilisés par GitHub sont fixés dans chaque API/config de déploiement ; ne pas les déduire du slug. Les URLs user/emails sont indépendantes : pas de concaténation `user_url + "/emails"` cachée. GitHub Enterprise/autre hôte compatible n'exige pas une branche de code.

### 2.3 JSONPath et normalisation

- Dépendance retenue **`jsonpath-rust = "=1.0.12"`**, lockfile versionné. Sources : publication registry 2026-10-13, dépôt non archivé consulté le 2026-11-06. `serde_json_path` 0.7.2 est une alternative crédible mais dernière release consultée de 2025-02 ; ce constat ne prouve pas son abandon. Ne pas introduire `jsonpath_lib`, jq, JS/Rhai/Lua ou un dialecte d'opérations maison.
- API vérifiée : `parser::parse_json_path(&str) -> Parsed<JpQuery>`, `JsonPath::query(&str)` et `query_with_path` sur une valeur JSON ; `query::js_path` retourne valeurs et chemins. Valider au boot, conserver expression validée/AST si adapté. **Ne pas supposer** un ancien `JsonPathFinder` ni un evaluator précompilé absent de cette version. Une réanalyse runtime bornée avec l'API publique est acceptable.
- V1 accepte chemins enfant/index et filtres de comparaison scalaires/logiques RFC 9535 ; exclut descente récursive, regex, fonctions/extensions et évaluation de code. Longueur expression ≤ 512 octets, profondeur de segments/filtres ≤ 16 ; contrôle de l'AST du parser maintenu, pas un second parser JSONPath. Les caractères `$`, `@` et filtres sont JSONPath, pas des templates.
- Clarification technique S-1 du 2026-11-07 : [budget préalable AST + structure JSON](oauthconnect-jsonpath-budget.md), avant toute évaluation exhaustive, incluant tests absolus imbriqués, copies et chemins. Détail du bornage accepté, sans changement des règles/pin ; aucun correctif ou résultat de preuve revendiqué par cette note.
- Pendant **une** complétion : un GET par définition API utilisée, réponse réutilisée par ses mappings ; pas de cache de profil/token entre opérations. Pas de jointure dépendant d'une réponse précédente. Deux mappings email et email_verified doivent référencer la **même API et le même objet sélectionné** (chemins parents des résultats identiques), pour ne pas déclarer vérifiée une autre adresse. Utiliser les chemins localisés de la bibliothèque ; config structurellement incohérente refusée au boot, incohérence dépendant du JSON refusée à l'exécution.
- Cardinalité : `sub` exactement un résultat ; autre claim zéro ou un. Plusieurs résultats => erreur, même égaux ; jamais « premier élément » implicite. Mauvais type (objet/array/bool à la place d'une chaîne, etc.) => erreur, pas champ absent. Null vaut absence uniquement pour les champs optionnels.
- `sub` OAuth : chaîne non vide préservée exactement, ou entier JSON non négatif représentable en `u64`, converti en décimal sans perte. Flottants/nombres hors borne refusés. C'est une règle typée de normalisation d'identifiant, **pas** une transform configurable. Pas de conversion email/username en ID.
- `email` : chaîne non vide ou absence ; `email_verified` : booléen, absent/null => false. Si email absent, résultat false même si le booléen isolé vaut true. Aucune adresse synthétique ni booléen converti depuis une chaîne.
- `preferred_username` : chaîne optionnelle ; le DTO historique `username: String` reçoit `""` si absent, sans fallback vers email/sub. Ce champ descriptif ne choisit jamais le username du compte plateforme. `picture` : chaîne optionnelle vers avatar_url, jamais téléchargée par Connect.
- **Delta GitHub explicite, ratifié le 2026-11-06 (« Email primaire seul »)** : ancien `map_profile` prend email primaire, sinon public `/user.email` et recherche son statut dans emails ; ancien appel emails absorbe des erreurs en liste vide. Cette jointure/fallback n'est pas exprimable par un mapping unique API+path sans élargissement interdit. V1 cible : primaire seul ; zéro primaire => email absent, doublon primaire => erreur ; panne/403/JSON invalide => erreur normale RustyCog. Pas de prétention d'équivalence complète. Ne pas ajouter une exception GitHub codée pour masquer ce changement accepté.

### 2.4 Profil OIDC standard

`profile.kind = "oidc"`, `issuer` exact épinglé, `allowed_endpoint_origins` explicites contenant l'origine issuer et celles des endpoints admis. Les scopes communs contiennent `openid` ; cible GitLab = `openid`, `profile`, `email`, sans reprise aveugle de l'ancien scope `read_user`. Ni mappings vendor, ni paramètre claims riche, ni nom de claim configurable.

`setup` utilise Discovery depuis l'issuer configuré, valide l'égalité exacte du champ issuer et les URLs auth/token/JWKS/userinfo présentes contre ces origines et le mode transport ; pas de confiance à `iss`, `jku` ou `x5u` reçus pour choisir un endpoint. La méthode d'authentification token configurée doit être compatible avec les métadonnées de l'autorité ; défauts normatifs de Discovery traités par la bibliothèque standard, pas par essai successif de méthodes. Métadonnées et JWKS sont attachés à cette instance, jamais à un cache global clé `kid` seulement. Échec initial => boot refusé. Le client HTTP utilisé par Discovery/JWKS est aussi injecté et borné.

Flow unique : Authorization Code, `response_type=code`. Pas d'Implicit/Hybrid, JWE ou algorithme `none`/HS* ; signatures **RS256** pour cette première cible GitLab, autre algorithme => refus explicite, nouveau besoin à cadrer. OIDC exige ici S256 configuré des deux côtés ; pas de downgrade si challenge/verifier manque. L'activation de capacité IAM suit le support récepteur réel.

Après échange : ID token obligatoire, validé **dans Connect avant d'utiliser un claim ou retourner des tokens** :

1. Signature avec clés de l'issuer configuré, algorithme autorisé et sélection `kid` non ambiguë ; ne pas remplacer la vérification par le seul TLS.
2. `iss` exact ; `aud` contient client_id et aucune audience supplémentaire non approuvée (V1 n'en approuve aucune) ; `azp`, s'il est présent, vaut client_id.
3. `exp` futur (pas de tolérance d'expiration V1), `iat` obligatoire et pas futur de plus de 60 s ; cohérence temporelle et `nbf` si présent. Aucun décodeur JWT non vérifiant utilisé comme source d'identité.
4. Nonce envoyé par ce parcours = nonce attendu de la transaction consommée ; ID token sans nonce ou nonce différent => refus. Vérifier `at_hash` s'il est présent avec l'access token effectivement obtenu. Ne pas inventer un nonce depuis le code, l'access token, le state ou le cookie.
5. `sub` chaîne non vide obligatoire, conservée **octet pour octet**. Pas de parsing numérique OIDC, de préfixe issuer ou de trim qui casserait l'ancien lien GitLab. La frontière issuer est portée par l'instance épinglée ; changement d'issuer sous le même slug bloqué opérationnellement.

Projection fixe : `sub -> id`, `preferred_username -> username` (absence => chaîne vide), `email` optionnel, `email_verified` booléen absent => false, `picture -> avatar_url` optionnel. Aucun fallback à `confirmed_at` pour OIDC. Une réponse UserInfo peut compléter les champs absents de l'ID token, uniquement via l'endpoint découvert autorisé et avec contrôle obligatoire `userinfo.sub == id_token.sub`. Les valeurs d'identité non-null contradictoires entre les deux sources entraînent un refus, pas un écrasement silencieux. Employer `openidconnect` pour les types/contrôles standard ; ne pas livrer un second vérificateur JWT maison.

Pour borner ce choix en V1 : un seul GET UserInfo si l'email est absent de l'ID token validé et si Discovery fournit cet endpoint ; pas d'appel pour exposer d'autres claims. Sans endpoint, le profil reste incomplet. Erreur de cet appel => erreur normale, pas succès dégradé silencieux. Email et son booléen de vérification viennent de la même source ; booléen absent => false. Un champ présent mais mal typé est refusé, pas « complété » ailleurs.

Cache JWKS **technique**, non de corrélation : TTL 300 s maximum, refresh au plus une fois par 30 s et par instance, une seule requête de refresh en vol ; `kid` inconnu peut demander ce refresh borné puis échoue s'il reste absent. Après expiration sans refresh réussi, échec fermé ; aucun usage de clé périmée sans borne. Timeouts/erreurs n'autorisent jamais signature/issuer/nonce ignorés. Les paramètres sont des constantes V1 testables, pas un sous-système de cache générique.

## 3. API S2S minimale et routing

### 3.1 Deux opérations v2, aucune exposition de claims riches

Base IAM d'une entrée : `https://<oauth-connect>/oauth-connect/github` (ou gitlab/autre slug). Le client ajoute le suffixe v2 ; ce slug n'est pas fourni dans le JSON. Exposer seulement POST :

- `/oauth-connect/{instance}/v2/authorize` : `{ redirect_uri, state, code_challenge?, code_challenge_method?, nonce }` => `{ authorization_url, scope }` existant. Challenge/méthode vont par paire, S256 seul ; `nonce` = chaîne aléatoire base64url sans padding de 32 octets, distincte du challenge et jamais verifier. OIDC le transmet ; OAuth ne transmet pas un paramètre OIDC au vendor. Ce contexte v2 est envoyé pour les deux profils, IAM n'ayant pas à connaître le type vendor.
- `/oauth-connect/{instance}/v2/complete` : `{ code, redirect_uri, code_verifier?, expected_nonce }` => `ProviderAuthentication { tokens: ProviderTokens, profile: ProviderUserProfile }`. `expected_nonce` a le même format et vient exclusivement de la transaction writer consommée. OIDC le valide ; le profil OAuth n'interprète pas de JWT/nonce OIDC et termine par ses GET JSON. Codes, verifier, tokens et nonce non logués/Debug redacted.

Toute complétion exige le redirect exact allowlisté ; vérifier la paire PKCE et les champs requis **avant** I/O. Pas de réponse partielle contenant des tokens si profil/validation échoue. Le code n'est pas rejoué automatiquement après timeout : l'état vendor peut déjà avoir changé et la transaction IAM est consommée. Nouveau START nécessaire.

Ajouter à `FederatedOAuthClient` des méthodes v2 nommées `authorize_authentication` et `complete_authorization` prenant les DTO v2. Un défaut temporaire pour les implémentations legacy **refuse**, il ne compose pas silencieusement les méthodes v1. L'adapter HTTP IAM les implémente explicitement ; ni `user_profile(access_token)` ni un default legacy ne valide l'OIDC de v2. Conserver les méthodes/DTO v1 pendant l'intégration des anciens crates/fixtures si nécessaire, puis les retirer seulement après inventaire des consommateurs, pas par substitution globale.

La cible déployée n'a pas de `/v2/profile`, d'alias `/v1` multi-instances ni de négociation/fallback automatique. Les anciens services v1 peuvent rester déployés temporairement pour les anciennes versions IAM pendant migration. Le nouveau parcours IAM utilise v2 exclusivement ; conserver un endpoint ancien ne vaut pas support d'un LOGIN OIDC via access token seul.

### 3.2 HMAC, routage et erreurs

- `SERVICE_PREFIX = "/oauth-connect"`, router embarquable non préfixé et prefixed_router standalone ; monter le préfixe une fois. Construire au boot les sous-routes de chaque instance avec **son** client et **son** HMAC. Aucun secret global qui remplace toutes les clés d'instance ; aucune tentative avec les autres clés après échec.
- HMAC de 0409 inchangé : `METHOD\nPATH\nTIMESTAMP\nBODY`, quatre parties séparées par newlines littéraux ; PATH complet reçu, ex. `/oauth-connect/github/v2/complete`. Utiliser l'URI d'origine avant nest, pas le chemin amputé. Fenêtre 30 s, comparaison constant-time et corps exact ; couvrir les champs nonce/PKCE par le même HMAC. URI non canonique/instance inconnue => 404 sans appel vendor ; signature invalide => 401 existant. Ne pas faire de lookup provider dans le corps après signature.
- HTTPS vérifié par défaut IAM→Connect ; options CA/transport déjà acquises préservées. HMAC demeure obligatoire même en exception locale. Aucun bearer IAM ou appel FGA. GET health/readiness n'expose ni catalogue sensible, ni config/secrets et ne valide pas le parcours OAuth.
- Erreurs de config/mapping/type/cardinalité/claim obligatoire/HTTP/validation OIDC : catégories/conversions `rustycog::core::{DomainError, ServiceError}` et `FederatedOAuthError` existantes aux frontières. Pas de nouvelle enveloppe d'erreurs, registre de codes, retry manager ou framework. Conserver le comportement HTTP des handlers communs (`server.rs`, échec de collaborateur en 502 générique) et erreurs IAM existantes ; adapter localement les conversions nécessaires sans fuite de valeur/JSON vendor. Les tests figent ces comportements existants, pas une API d'erreurs parallèle.

## 4. Transport, secrets et composition

- Clients HTTP injectés par `setup`, redirect policy **none**, timeout total 10 s par requête, connexion 3 s, corps décompressé/plafond lu ≤ 1 MiB pour token/profil/discovery/JWKS. Ne pas appeler `.json()`/`.text()` sans lecture bornée préalable. Même politique dans le callback HTTP de `openidconnect` : son client pratique par défaut ne suffit pas à garantir le plafond.
- 2xx + JSON valides attendus ; pas d'acceptation d'une page HTML ou d'une redirection. Token endpoint = POST form Authorization Code, client auth configurée, Accept JSON ; access_token non vide et token_type Bearer compatible requis, refresh/expiry optionnels du DTO courant. Pas de requête GET token, de token dans query ou d'URL dérivée d'un claim.
- Résolution secrets au boot, avant création du registry : reprendre SDK `SecretReference`/`OpenBaoConfig`, KV v2, token-file explicite borné et sélection openbao/direct de GitHub. Valider l'ensemble avant I/O autant que possible ; aucune clé manquante ne provoque fallback direct/env. Direct reste explicite ; références et valeurs directes sont mutuellement exclusives pour chaque instance. HMAC résolu ≥ 16 octets. Pas de secret partagé implicitement entre entrées.
- Erreur sur une instance => boot entier refusé ; absence de secret ne désactive pas un slug. Types config/résolus, DTO secrets et Debug/serialization restent redacted, y compris dans les listes. Chemins/keys de références eux-mêmes peuvent être sensibles : respecter les wrappers SDK et ne pas dumper la config.
- Ne pas modifier le SDK ni son pin pour réaliser le service ; réemployer l'acquis 0505 sans l'auto-accepter. Scopes et URL côté Connect, jamais secrets vendor dans le registry IAM. Rotation/politiques/nouveaux workloads OpenBao sont des opérations autorisées séparément, pas des effets d'un rename.

## 5. Adaptation IAM et contexte OIDC

Modules touchés : `infra/src/auth/http_connector.rs`, composition du registry `setup/src/app.rs`, `domain/src/entity/oauth_transaction.rs`, ports/repos/modèles de transaction, `domain/src/service/oauth_service.rs`, `application/src/usecase/link_provider.rs`, démarrage navigateur et fixtures HTTP. Le domaine IAM dépend toujours du contrat vendor-neutral, pas de `oauth-connect-infra` ni de JSONPath/openidconnect.

1. Au BEGIN v2, générer avec OsRng un nonce indépendant 32 octets encodé base64url sans padding. Le persister comme **secret temporaire de la transaction IAM**, avant de renvoyer le contexte public au START. `BegunOAuthTransaction` expose nonce+challenge, jamais verifier. Ni dérivation depuis state/cookie/PKCE ni cache mémoire. IAM génère ce contexte de manière uniforme, sans enum GitHub/GitLab/OIDC.
2. Étendre le writer transactionnel existant par un champ nullable `authentication_nonce` (legacy en vol). `ConsumedOAuthTransaction` seul donne accès au nonce attendu et verifier pour appeler `complete_authorization`. Consommation atomique charge ces valeurs puis les efface du stockage dans le même commit, à l'image du verifier actuel. Toutes les conditions writer/provider/intention/cible/redirect/expiry demeurent. Jamais de SELECT replica comme capacité.
3. Nonce requis dans les transactions du parcours v2 ; ancien state sans contexte durable => refus et nouveau START. Pas de fabrication au callback pour réparer une transaction. Cookie/browser nonce/state nonce gardent leur rôle distinct et TTL ≤ 600 s.
4. LOGIN et LINK/RELINK utilisent le même résultat v2 validé à la place de `exchange_code` puis `user_profile`. Conserver l'ordre et les règles métier après réception du profil, ainsi que le stockage des tokens au succès ; ne rien persister dans provider_tokens avant identification/autorisation métier. Échec OIDC ne crée ni user, ni lien, ni session.
5. Migration SQL **additive** de la transaction (et mapping entity/repository/tests) pour préserver les états locaux actuellement utiles ; ne pas réaplatir/détruire le schéma initial au prétexte de l'ancienne note de 0412. Aucune migration des provider_tokens, emails ou UUID. Déploiement migration avant nouveau binaire ; rollback garde la colonne nullable et réutilise l'ancienne version de binaire/config. Pas de reset DB, réécriture d'issuer ou bulk relinking.

GitLab : `sub` doit être le **même** ID canonique que `GitLabUser.id.to_string()` actuel. Les slugs github/gitlab et leurs lignes provider_tokens restent inchangés. Un changement d'issuer n'est pas une nouvelle incarnation transparente : employer une nouvelle autorité/slug et cadrer le linking, pas recycler gitlab silencieusement. Si le fournisseur concret ne respecte pas l'équivalence sub ancien id, **stop bascule et escalade** ; pas de mapping OIDC spécial, email matching forcé ou migration de liens improvisée.

## 6. Migration topologie, Cargo, déploiement et docs

Séquence d'implémentation (chaque tranche avec RED comportemental puis GREEN quand exécutable ; pas de pipeline d'agents imposé) :

1. Arbitrages 0413 acceptés le 2026-11-06 ; appliquer ce contrat actif et le plan QA réconcilié. Après intégration documentaire par le parent, écrire les témoins RED de config/mapping/routing/normalisation et d'invariants IAM avant corrections, sans attendre les E2E externes pour les unités/IT autorisées. Une erreur de compilation due à une API inexistante prépare le test mais n'est **pas** preuve de régression comportementale.
2. Ajouter DTO/trait/récepteur v2 et tests de frontière HMAC/PKCE/nonce, sans casser les anciens services encore membres. Construire OAuthConnect et ses deux profils derrière ces ports. Tester des slugs arbitraires compatibles, pas seulement github/gitlab.
3. Étendre la transaction IAM/migration puis adapter **les deux** chemins login et link/relink, registry HTTP et fixtures. Vérifier refus legacy/sans nonce et consommation avant I/O. Ne pas changer les politiques de compte pour rendre les tests verts.
4. Préparer le récepteur et la config de déploiement unifiée ; la bascule IAM vers les bases `/oauth-connect/github` et `/oauth-connect/gitlab`, callbacks publics inchangés, reste conditionnée aux preuves E2E/cutover du §7. Drainer les anciens START pendant au moins leur TTL 600 s et coordonner le rollout ; sinon anciennes transactions sont refusées avec nouveau START. Pas de mélange silencieux v1/v2 ni de retry d'un callback consommé. Conserver une version/image/config legacy récupérable pour rollback, pas les vieux sources en double éternellement.
5. Après preuves locales et handoff E2E recevable, retirer `services/{GitHubConnect,GitLabConnect}` et leurs crates/members/path dependencies/tests déplacés ; un seul service cible. Racine Cargo et lockfile, Dockerfile/build sélectionnant les binaires, listes d'images/scripts/CI si elles nomment les anciens packages doivent être actualisés par inventaire exact. Pas de bump SDK connexe.
6. `docker-compose.yml` : une définition OAuthConnect remplace les deux, mettre à jour `depends_on`, volumes config/secrets et healthchecks ; proposer port hôte HTTP 8085 / HTTPS 8446 (anciens ports GitHub, **à libérer au cutover**, sans collision pendant coexistence). Le port GitLab 8086/8447 cesse d'être la cible après retrait. Pas d'alias wildcard qui contourne le PATH signé.
7. TLS/mesh : actualiser SAN/service DNS, certificats générés et destinations réseau. Contradiction actuelle HTTP interne Compose signalée dans 0413 : nouveau chemin vérifié HTTPS, ou exception locale explicite, pas de garantie fictive tirée des ports 8446/8447. Ne pas ouvrir les routes S2S dans les allowlists navigateur Envoy. Le monolithe conserve l'adapter HTTP IAM ; aucun `.nest` OAuthConnect ajouté.
8. Déploiement local-full affecté : `ops/deploy/apps/overlays/local-full/oauth-proof/{iam.toml,resources.yaml}`, `github-openbao/github.toml`, `helpers/seed-openbao.sh`, `ops/scripts/generate-platform-mesh-certs.sh` sont des consommateurs repérés, pas l'inventaire final. Préserver les chemins KV GitHub existants (`github-connect/oauth`, `github-connect/iam`) et références montées tant qu'aucune migration ops explicite n'est approuvée ; pas de rotation de fait par renommage du service. Aucun apply/activation runtime dans le lot documentaire ou par cette prescription.
9. Docs à migrer : fiches `docs/services/{github-connect,gitlab-connect}.md` vers fiche OAuthConnect + renvois historiques, guide `services/IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md`, README des services/workspace et exemples config ; pointer les contraintes secrets/déploiement existantes sans annoncer leur qualification live. Les annotations de supersession partielle 0408/0411 et interface 0407, ainsi que l'index ADR, portent désormais l'acceptation du 2026-11-06 ; pointeur wiki IAM et synchronisation des digests sont pris en charge par le parent. Pas de réécriture des photographies 0100–0502 ni des preuves 0412.

## 7. Critères RED/GREEN et handoff de preuve

Le QA planner choisit les placements/noms de nouveaux tests et réconcilie ses oracles indépendants ; cette liste donne le comportement contractuel, sans se substituer à son plan.

- **Config/boot** : deux profils et ≥ deux instances ; troisième slug compatible sans branche vendor ; unknown keys/env, vieux préfixes, registry vide/dupliqué, mauvais profil/path/API, secrets mixtes/manquants, HMAC court, issuer/URL/transport/scopes/PKCE incohérents refusés avant écoute. Aucun fallback de secret et Debug redacted, y compris échecs.
- **Mapping** : IDs numériques GitHub convertis sans perte et sujets OIDC inchangés ; email optionnel, absent/null vs mauvais type, bool absent/false, 0/1/2 résultats, sélection primaire, paire email/verified même objet, username absent, avatar actuel. Deux champs même API => un seul GET ; erreurs HTTP/JSON/limite ne deviennent pas faux succès. Témoin primaire absent documente le delta GitHub ratifié.
- **S2S** : serveur Connect réel + collaborateurs HTTP WireMock (ADR0200/0201), pas mock des usecases du service. Même code/body signé pour une autre instance ou PATH raccourci refusé ; aucune I/O vendor avant authentification/validation. Challenge S256 atteint authorize sans verifier, verifier seulement à complete consommé, nonce couvert par HMAC. Route inconnue pas de fallback ; pas de downgrade v2→v1.
- **OIDC** : issuer/client/audience/azp, signature fausse/alg none/HS/inconnu, exp/iat/nbf, ID token absent, nonce absent/faux, at_hash faux, UserInfo sub différent et claims contradictoires => refus sans effet IAM. Deux instances avec kid ou sub identiques mais issuers distincts ne partagent ni confiance ni lien. Test JWKS rotation/TTL/échec refresh et reprise sur un autre replica sans cache de corrélation.
- **IAM** : lien existant + email changé conserve UUID ; email manquant refuse LOGIN même déjà lié ; nouveau lien email vérifié rapproche, non vérifié refuse ce rapprochement ; nouveau compte reste incomplet selon règles actuelles. LINK sans email admis, identité liée ailleurs refusée. LOGIN/LINK/RELINK conservent les garanties writer/cookie/state, purge du nonce/verifier et échec fermé après consommation. Tester migration additive + anciennes transactions sans nonce ; pas de reset pour les faire passer.
- **Transports/secrets** : conserver tests loader strict/OpenBao (échec coffre, valeur absente, référence invalide, taille token-file, HMAC réellement consommé) et modes TLS/CA ; timeout, redirect et corps >1 MiB démontrés. GitHub endpoint/config non littéral testé. Aucune fuite de nonce/code/token/verifier/client_secret par format Debug/HTTP/log testable.

Commandes candidates **à exécuter par les auteurs autorisés, pas exécutées ici**, séquentielles sur Windows, un Cargo à la fois, jobs=24 du dépôt sans override :

- `cargo test -p idp-connect-contract --features server` ;
- `cargo test -p oauth-connect-configuration` puis `cargo test -p oauth-connect-domain` et `cargo test -p oauth-connect-infra --lib` une fois ces packages créés ;
- `cargo test -p iam-domain --lib`, puis tests unitaires ciblés des crates IAM modifiées ;
- IT régressions existantes : `cargo test -p iam-service --test oauth_browser_transactions`, `--test auth_transactions_postgres`, `--test oauth_cleanup_lifecycle`, `--test auth_oauth_callback`, `--test auth_oauth_start`, `--test verified_idp_https` (chaque commande séparée, fixtures actuelles adaptées) ; nouveaux tests OAuthConnect sous son package, pas « tout cargo » ;
- pendant transition conserver `github-connect-configuration` / `github-connect-service --test openbao_startup` et `--test hmac_contract` ou leurs tests déplacés **avant** suppression ; ces suites ne sont pas des pure unit gates garanties ;
- `cargo check -p oauth-connect-service -p iam-service -p oodhive-monolith` après confirmation des noms au manifeste courant ; contrôles fmt/clippy ciblés selon politique du dépôt.

Avant IT/runtime, acteur exécutant charge lifecycle/test-evidence, inspecte fixtures transitives et tient slot Cargo/registre Docker. Aucun Kind pour IT ; pas de build/test de développement Docker. Ne pas interpréter préflight bloqué, dépendance introuvable, erreur de compilation ou test filtré comme RED comportemental/PASS. Conserver les preuves RED puis GREEN sur même oracle et commandes/versions réelles.

**Handoff E2E externe (bloquant bascule, autre propriétaire)** : fournir config publique expurgée (slugs, issuer, scopes, callbacks, API/mappings, PKCE, mode secrets/CA sans valeurs), versions IAM/Connect, état de migration et fiche des différences de profil. Attendre de cet acteur les deux parcours GitHub/GitLab, le compte/UUID existant et son ancien provider_user_id GitLab comparés au nouveau sub exact, email/email_verified effectifs et absence d'action/claim riche. Refaire les oracles transactionnels/linking/redirect/HMAC pertinents sur les routes réelles ; préciser fournisseur réel ou synthétique et limites. Un mock/signature valide local ne prouve pas le `sub` du GitLab déployé. Ne pas écrire/exécuter ses scénarios ni créer un second harness ici.

## 8. Risques, sources et limites de ce contrat

- Risques de compilation : nouveaux DTO/méthodes consommés dans services **et fixtures**, champs ajoutés aux constructeurs/rows de transaction, option `server` Axum, Send/Sync des AST/clients et future async. Ne pas traiter un zéro de références LSP comme absence de tests/macros.
- `openidconnect` 4.0.0 est **déjà déclaré** au workspace, mais réexporte oauth2 **5.0.0**, tandis que le workspace déclare oauth2 **4.4.2**. Ne pas mélanger leurs `ClientId`, `Scope`, `PkceCodeVerifier` ou signatures HTTP ; utiliser les réexports d'openidconnect dans le profil OIDC, isoler les conversions ou l'échange HTTP derrière les ports. Aucun bump global oauth2/reqwest nécessairement requis. Aucun résultat Windows/MSRV/feature/lockfile acquis ici.
- APIs JSONPath vérifiées sur sources versionnées, pas compilées. Le pin et la surface d'AST doivent être confirmés par les premières validations ciblées ; ne pas remplacer ce contrôle par une nouvelle DSL. Aucun audit supply-chain ou benchmark prétendu.
- Sources locales : ADR0407–0412 et0505 ; `crates/idp-connect-contract/src/{client,dto,server}.rs`, `services/IAMRusty/{domain/src/entity/oauth_transaction.rs,domain/src/service/oauth_service.rs,application/src/usecase/link_provider.rs,infra/src/auth/http_connector.rs}`, `services/{GitHubConnect,GitLabConnect}/infra/src/{github,gitlab}.rs`, configuration/setup/http des Connect ; tests/fixtures cités ci-dessus et briefs du parent du2026-11-06.
- Sources primaires consultées le2026-11-06 : [JSONPath parser](https://docs.rs/jsonpath-rust/1.0.12/jsonpath_rust/parser/fn.parse_json_path.html), [query/chemins](https://docs.rs/jsonpath-rust/1.0.12/jsonpath_rust/trait.JsonPath.html), [registry](https://crates.io/api/v1/crates/jsonpath-rust), [dépôt](https://github.com/besok/jsonpath-rust), [OIDC Core validation](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation), [RFC9700](https://www.rfc-editor.org/rfc/rfc9700.html#section-4.5.3), [GitLab OIDC](https://docs.gitlab.com/integration/openid_connect_provider/), [GitHub emails](https://docs.github.com/en/rest/users/emails#list-email-addresses-for-the-authenticated-user), [openidconnect4](https://docs.rs/openidconnect/4.0.0/openidconnect/), [son manifeste](https://docs.rs/crate/openidconnect/4.0.0/source/Cargo.toml).
- Sources symboliques de l'analyse initiale consultées au checkout canonique inchangé ; actualisation de ratification documentaire depuis `0a51206b1f9f4a999de7fc4812e947aee0445531` dans le worktree isolé, hors index partagés. Pas d'activation/reconstruction d'index, pas d'édition applicative/tests, Cargo ou runtime par cet auteur. Acceptation ADR acquise ; aucune preuve nouvelle de GREEN/E2E. Intégration/publication, wiki et synchronisation des digests restent au parent.
