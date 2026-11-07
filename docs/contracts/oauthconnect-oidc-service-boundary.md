# OAuthConnect — coutures OIDC, registry mixte et service

Date : 2026-11-07. Complément du [contrat actif](oauthconnect-multi-instances.md) §§2.4/3/4/6/7 et de la [frontière HTTP/secrets](oauthconnect-http-secrets-boundary.md). GO utilisateur « go oidc et service op » ; base publiée `eabc6459bc5f3f5c564d8789de68594226cb7aed`, SDK `93b890ff7b821e9cb2e7d0aeb81957d098778a2f`. ADR0413 reste **Accepted / Partial**. Cette note fixe les seules coutures avant le socle et ses consommateurs, pas une livraison OIDC/service ni un résultat de test.

## 1. Portée et dépendance standard

Les choix ci-dessous sont des détails de la cible déjà acceptée : deux profils, même port `FederatedOAuthClient`, HTTP S2S v2, validation OIDC standard, cache technique et composition RustyCog. **Pas de nouvelle ADR ni ratification nécessaire.** Aucun nouveau service de cache/port inter-service, vérificateur JWT maison, framework auth, SDK/pin, hot-load, claims riches ou actions. IAM, nonce durable/migration, déploiement/Compose/Kind, retrait legacy et E2E externes restent hors lot ; l'ancien ID GitLab/sub est toujours une gate de bascule externe.

Conserver la dépendance workspace **`openidconnect = { version = "4.0.0", ... }`**, contrainte Cargo compatible4.0.1, et l'activer dans infra. Les API officielles **4.0.1**, patch publié le2026-10-15, ont été vérifiées pour cette couture ; cette version **n'est pas au lock de la base**. Parent : résoudre/verrouiller cette consommation avant les tests, sans modifier la contrainte pour provoquer un bump global ; si la version effectivement résolue diffère, recouper les API avant de publier le T0. Aucun résultat de compilation de cette dépendance n'est acquis ici.

`openidconnect`4 réexporte **oauth2 v5**, distinct d'`oauth2` workspace4.4.2. Dans le profil OIDC, employer ses réexports (`ClientId`, `ClientSecret`, `Scope`, `AuthorizationCode`, PKCE, `HttpRequest/HttpResponse`, traits de réponse) ; ne pas mélanger les types/clients HTTP v4. Pas d'upgrade connexe oauth2/reqwest/http : les ajouts/arêtes du lock et toute contrainte incompatible sont à contrôler par le parent, pas à corriger par un update général.

## 2. Surface minimale et callback HTTP borné

Nouveau client infra opaque, `Debug` redacted, `Send + Sync` :

```rust
// Méthode associée dans impl OidcClient ; corps T0 refusant à écrire.
pub async fn new(
    provider: &ProviderConfig,
    client_secret: SecretValue,
    http: reqwest::Client,
) -> Result<Self, DomainError>;
```

Signature indicative de méthode associée : le T0 doit la rendre compilable/refusante avant consommateurs. Le constructeur valide la déclaration OIDC, puis obtient **Discovery et le JWKS initial** avec le client de confiance injecté par setup ; aucun listener n'existe encore. Échec => erreur sanitized, aucune instance « sans clés en attendant ». Le client implémente les méthodes v2 existantes ; les surfaces legacy v1 restent refusantes, sans token→profile implicite ni ID token/claims bruts renvoyés à IAM. Chaque méthode v2 vérifie elle-même redirect exact, nonce et PKCE S256 obligatoire avant I/O, même appelée directement sans middleware S2S.

Réemployer `CoreProviderMetadata::discover_async` et les types standards JWKS. Cet appel fait **déjà** le GET Discovery puis le GET `jwks_uri` : ne pas ajouter une seconde acquisition initiale par défaut. La bibliothèque contrôle l'issuer de Discovery ; contrôler aussi les endpoints/token auth/signature policy selon §2.4. Le callback valide **chaque URL avant son I/O**, y compris le JWKS suivi à l'intérieur de Discovery, contre l'issuer/origines autorisées et les règles URL/transport ; vérifier les autres endpoints découverts avant de les utiliser. Ne pas attendre le retour de Discovery pour empêcher un GET JWKS vers une origine interdite. Ne suivre ni redirect, ni `jku/x5u` d'un JWT.

Adapter HTTP privé implémentant `openidconnect::AsyncHttpClient`, ou closure compatible : entrée `HttpRequest = http::Request<Vec<u8>>`, sortie `HttpResponse = http::Response<Vec<u8>>`. Le `reqwest::Client` nu et le callback pratique de la bibliothèque ne sont **pas** une preuve du plafond : toutes ses opérations Discovery/JWKS/token/UserInfo passent par le raccord borné.

**Petit refactor partagé, pas seconde lecture** : extraire de `infra::transport::read_json` la lecture de corps bornée en octets, puis faire déléguer `read_json` à ce helper en conservant son comportement. Le callback capture statut/headers utiles et remet les **octets originaux bornés** à la bibliothèque ; ne pas parser en `Value` puis resérialiser (perte des doublons/formes, contraintes des parseurs standards, UserInfo signé). Le helper conserve les contrôles livrés : succès HTTP, encodages résiduels,1MiB **décompressé** avant extension, EOF avant parsing, erreurs génériques sans corps/source sensible. Le callback peut refuser non2xx par erreur transport sanitized ; pas de nouvelle API d'erreur OAuth détaillée exigée. Gzip/timeouts/retry/CA restent ceux du vrai builder livré, pas un nouveau builder OIDC.

## 3. Validation OIDC et données minimales

- Construire l'URL via `authorize_url` avec **state et nonce reçus**, sans les régénérer ; aucune demande de verifier à authorize. L'API documentée `PkceCodeChallenge` fournit des constructeurs depuis verifier, pas un `new(challenge)` à supposer : si besoin, utiliser les deux paramètres fixes `code_challenge`/`code_challenge_method=S256` via `add_extra_param` après validation v2, sans simultanément demander un autre PKCE au builder. Pas de paramètres supplémentaires configurables. `openid` déjà ajouté par le client OIDC standard doit apparaître une fois ; ajouter les autres scopes déclarés sans duplication.
- Token : Authorization Code/redirect exact/S256 et auth token configurée via les types standard et le callback ; aucun retry du POST, y compris lorsqu'un refresh JWKS devient nécessaire **après** réception des tokens. ID token obligatoire ; `claims`/verifier de la bibliothèque pour signature RS256, issuer, audience, expiration et nonce attendu. Interdire algorithme libre, désactivation signature/audience/issuer ou remplacement par TLS.
- Defaults à ne pas surinterpréter : configurer les audiences supplémentaires comme non approuvées et le hook `set_issue_time_verifier_fn` pour la borne iat du contrat ; vérifier la cohérence `iat/exp` et `exp > maintenant` sans tolérance. La bibliothèque4.0.1 laisse **azp** à l'appelant : vérifier le champ signé `authorized_party()` quand présent. **nbf**, non porté par les claims Core standards, doit être un petit champ additionnel privé typé de vérification, lu **après validation cryptographique** et contrôlé s'il existe ; pas décodage JWT/base64 ou vérification JWS maison. Aucun de ces champs privés n'étend le DTO de sortie.
- Vérifier `at_hash` présent avec **`AccessTokenHash::from_token`**, algorithme et clé réellement retenus par le verifier ; pas un hash bricolé ni comparaison à un autre access token. Les erreurs typées de la bibliothèque servent à choisir la conduite interne, jamais leurs textes/corps/token en sortie/logs.
- Projection et UserInfo inchangés : `sub` exact non vide, claims standards fixes, email/verified de la même source ; un seul UserInfo si email absent et endpoint disponible. `user_info(access_token, Some(sub_validé))` contrôle la substitution ; conflits non-null refusés. Pas d'appel pour collecter des extras ni faux email/verified ; absent preferred_username reste chaîne vide. Pas de profil/tokens partiels avant fin de toute validation.

## 4. JWKS : cache par client d'instance, à la demande

État privé partagé par les clones d'un **même client d'instance** : snapshot JWKS immuable, instant de succès, instant de dernière tentative, génération/résultat de refresh et garde single-flight. Pas de cache global par `kid`/issuer, même si deux instances utilisent le même `kid`, `sub` ou endpoint. Chaque replica possède son cache technique local ; les bornes ci-dessous ne constituent pas un rate limiter distribué entre replicas/clusters. Cela ne change pas l'autorité logique de l'instance ni l'identité provider+sub.

- Après acquisition initiale, timestamp de succès et dernière tentative initialisés de façon conservatrice à la fin de l'initialisation. Cette acquisition consomme aussi la première fenêtre de30s. Endpoints/issuer restent épinglés ; refresh = GET du **JWKS fixé**, pas redécouverte/hot-load de métadonnées.
- Snapshot frais si âge **strictement inférieur à300s** ; à300s il est expiré. Clé connue et cache frais : vérification sans GET. Expiré : obtenir un snapshot neuf avant validation, sinon refus ; aucune clé expirée de secours.
- Sur **`SignatureVerificationError::NoMatchingKey`** du chemin standard, un cache frais peut demander un refresh ; au plus une tentative supplémentaire de vérification du **même** ID token. Ne pas déclencher de GET sur tout CryptoError/claim invalide, ambiguïté de clé ou algorithme refusé. Une rotation réutilisant le même kid n'est donc pas promise avant expiration du TTL ; pas d'assouplissement de signature pour l'accommoder.
- Enregistrer la tentative **avant** I/O ; nouvelle tentative possible seulement si ≥30s depuis la précédente, succès **ou échec**. À la sortie de la garde single-flight, recontrôler génération, fraîcheur et résultat déjà obtenu par l'autre tâche : les waiters ne font pas chacun un GET. Pas de reset de fenêtre sur chaque requête/erreur, ni d'attente cachée de30s dans un handler pour contourner le refus.
- Un échec ne renouvelle pas le timestamp de succès. Une ancienne clé encore fraîche reste utilisable pour son identité vérifiée ; une clé absente ou expirée reste refusée. Ne pas conserver un « stale autorisé » sans borne après panne.
- **Pas de tâche périodique/détachée** pour ce cache : refresh dans les futures possédées des requêtes, single-flight local. Libérer la garde en annulation/erreur ; pas de flag in-flight bloqué ni waiter abandonné. Ne pas tenir un verrou de snapshot pendant la vérification JWT ou une I/O réseau ; une garde de sérialisation de refresh distincte peut posséder l'attente réseau.

**Testabilité sans sommeil300s** : temps de cache monotone (`tokio::time::Instant`), indépendant de l'heure UTC des claims. Petit hook privé de construction/cache `Arc<dyn Fn() -> Instant + Send + Sync>`, production `Instant::now`, tests colocalisés avançant une horloge contrôlée (30/300/±borne) sans toucher les deadlines HTTP. Pas de nouveau framework Clock ni option de config publique. Publier ce point privé dans le T0 pour les tests du module avant GREEN ; les tests HTTP de surface continuent d'utiliser le constructeur normal. Les hooks UTC existants du verifier standard peuvent servir aux cas temporels JWT ; ne pas faire dépendre leur expiration d'une avance du seul cache.

## 5. Composition mixte, sans casser l'acquis OAuth-only

Garder **inchangés** `ConfiguredOAuthClient` et `build_oauth_clients(&AppConfig)` : cette API reste OAuth-only et refuse une déclaration OIDC **avant toute acquisition**, comme ses tests publiés. Ne pas changer ces attentes pour faire passer le nouveau lot.

Ajouter en setup, sans dépendance HTTP→setup ni nouveau port :

```rust
pub struct ConfiguredClient {
    pub id: String,
    pub client: Arc<dyn FederatedOAuthClient>,
    pub hmac_key: HmacKey,
}
pub async fn build_clients(config: &AppConfig)
    -> Result<Vec<ConfiguredClient>, DomainError>;
```

Bundle `Debug` redacted/non sérialisable. Valider tout le catalogue, résoudre les secrets via le chemin existant, vérifier slugs/completude et construire chaque client avec **le vrai builder** : OAuth constructeur synchrone acquis, OIDC constructeur async avec hydratation initiale. Réemployer les helpers internes communs, sans deuxième résolution induite par un appel imbriqué à l'ancien builder. HMAC conserve les octets déjà normalisés, pas de second trim. Erreur sur une instance => aucun résultat partiel/publié ; pas de rollback promis des GET coffre/Discovery déjà réalisés. Le nouveau builder ne saute aucun profil et ne publie pas de readiness provisoire avant fin de tous les clients.

## 6. Router et startup RustyCog minimal

- Nouvelle crate HTTP OAuthConnect mince : dépend de RustyCog HTTP, readiness et `idp-connect-contract` avec `server`, **pas de setup/infra OIDC**. Ses entrées sont une collection `Vec<(String, ConnectState)>` construite par setup ; registry immuable validée (non vide, slugs canoniques, doublons refusés avant perte d'information dans une map). Aucun lookup de client/clé depuis le body, hot-load ou cache global.
- Surfaces `create_router(AppState, entries) -> Result<Router, DomainError>` et `create_prefixed_router(AppState, entries, Arc<ReadinessProbe>) -> Result<Router, DomainError>` ; `SERVICE_PREFIX="/oauth-connect"`. Le premier monte `/{slug}` → **`idp_connect_contract::server::router_v2` existant**, le second ajoute le préfixe une fois et `/ready`, comme le pattern GitHubConnect. Réutiliser RouteBuilder/health et AppState ; pas de nouveau middleware HMAC, bearer utilisateur, permission FGA ou framework de handlers.
- Le router v2 commun vérifie `OriginalUri.path()` avant JSON/client et porte déjà nonce/PKCE,64KiB entrant, erreurs401/413/502. Tester le chemin reçu complet `/oauth-connect/{slug}/v2/...`, mauvaise clé/autre instance/path raccourci/body altéré ; aucun fallback v1. Instance inconnue/non canonique404 sans I/O ; mauvaises méthodes suivent le router existant, pas une nouvelle table d'erreurs.
- Setup `Application::new(config).await`/AppBuilder : validation, secrets/clients mixtes complets, AppState/registry/probe, **puis seulement** lancement listener. Logging SDK une fois dans le binaire `oauth-connect-service`, chargement strict existant, erreurs sanitized. Pas de DB/events/queues/FGA/nest monolithe à ajouter par gabarit ; ne pas copier des exigences Bearer sur les routes HMAC.
- Pour le listener réel/testable, utiliser l'API SDK async **`start_router_owned(router, ServerConfig)`** déjà livrée : propriétaire `OwnedHttpServer`, `local_addrs` pour port0, demande d'arrêt et `join`. Une méthode additive `pub async fn start(&self) -> Result<OwnedHttpServer, _>` sur `Application` est la couture testable ; `run(self)` supervise signal/sortie serveur et ferme/joint les ressources, sans runtime parallèle. Réemployer `HttpShutdownBudget`/pattern existant (grâce5s/deadline6s), pas réimplémenter HTTP/TLS/ownership. L'ancien `serve_router` demeure compatible mais son abandon seul n'offre pas cette preuve d'arrêt.
- Absence de tâche cache détachée : les rafraîchissements à la demande appartiennent aux requêtes HTTP possédées. Un échec de construction/bind ne laisse ni listener frère, ni catalogue partiel ; readiness ne qualifie pas IAM, le fournisseur live ou la bascule. Aucun changement/adoption implicite des statuts0503/0505.

## 7. Ownership et entrées TDD

- **Socle avant consommateurs** : parent intègre signatures, petit refactor de reader en octets sans changer ses règles, dépendances/lock nécessaires et points de test privés. Contrôler les références/read_json et les régressions affectées ; compilation absente/erreur API n'est pas RED. Pas de nouveau comportement OIDC caché dans le T0 refusant.
- **A OIDC** : infra `oidc`/callback/JWKS/verification et tests privés-cache + tests HTTP sortants ; les quelques lignes du reader partagé appartiennent à un seul auteur désigné, pas en concurrence. Aucun changement de projection/budget, SDK ou code IAM.
- **B HTTP/registry** : nouvelle crate HTTP, montage v2/health/readiness et ses tests avec fakes **au port sortant** ; tests indépendants d'A, mais pas de mock des usecases du service. Vrai listener local pour les oracles réseau ; faux client ne prouve pas OIDC.
- **C composition/service**, après intégration A+B : nouveau `build_clients`, Application/binaire et tests de boot mixte/échec tardif/listener réel/HMAC→OIDC signé. Les API OAuth-only et leurs preuves restent présentes. Parent possède root Cargo/lock, exports partagés et publication ; pas de consommateur sur deux branches non intégrées.

QA minimale à rapprocher du plan indépendant : vrais JWT RS256 signés pour succès/attaques de claims, clé/issuer/audience/nonce/at_hash/azp/nbf ; UserInfo sub/conflits ; origine JWKS interdite **avant requête**, corps/redirects bornés sur tous endpoints ; initialisation1JWKS, hits frais0GET, égalitéTTL/30s, rotation nouveaukid, échec sans stale, rafale concurrente single-flight, annulation de refresh et isolation deux instances mêmeskid/sub. Clock contrôlée pour TTL, pas sommeil300s. Router/service : PATH complet, zéro appel avant HMAC, catalogue mixte entier avant écoute, CA/bind failure, port0 et shutdown/join réels. WireMock/TLS locaux réemployés, pas nouveau harness global, Docker ou E2E. Les gaps réseau/headers/RSS du transport précédent restent honnêtes, pas effacés par ces tests.

## 8. Sources, statut des preuves et handoff

Sources canoniques eabc lues via Serena (racine conservée, aucune activation/réindexation) : `services/OAuthConnect/infra/src/transport.rs:read_json`, `setup/src/composition.rs:build_oauth_clients` et ses références `minimal_composition` ; `crates/idp-connect-contract/src/server.rs:{router_v2,hmac_middleware}` ; pattern `services/GitHubConnect/http/src/lib.rs` et setup/app ; SDK `rustycog-http/src/{builder,owned_server}.rs` et [contrat propriétaire HTTP](http-runtime-owned-shutdown.md). Worktree documentaire non indexé, sous-module consulté au canon ; aucun SDK modifié.

Recherche externe proactive et documentation officielle **4.0.1** lues le2026-11-07 : [AsyncHttpClient](https://docs.rs/openidconnect/4.0.1/openidconnect/trait.AsyncHttpClient.html), [Discovery](https://docs.rs/openidconnect/4.0.1/openidconnect/struct.ProviderMetadata.html#method.discover_async) et [source issuer/JWKS](https://docs.rs/crate/openidconnect/4.0.1/source/src/discovery/mod.rs), [verifier et hooks](https://docs.rs/openidconnect/4.0.1/openidconnect/struct.IdTokenVerifier.html), [source azp](https://docs.rs/crate/openidconnect/4.0.1/source/src/verification/mod.rs), [PKCE](https://docs.rs/openidconnect/4.0.1/openidconnect/struct.PkceCodeChallenge.html), [at_hash](https://docs.rs/openidconnect/4.0.1/openidconnect/struct.AccessTokenHash.html), [UserInfo](https://docs.rs/openidconnect/4.0.1/openidconnect/struct.Client.html#method.user_info). Lecture d'API/source, **pas compilation ni validation de la dépendance résolue**.

Parent : intégrer cette annexe et réconcilier QA/API avant tests consommateurs, conserver les189preuves antérieures tant que non affectées, demander RED→GREEN/qualité/revues bornées sur le diff nouveau. Aucun code/test/Cargo/commit/listener/runtime exécuté par cet auteur ; aucune nouvelle preuve OIDC/service ni promotion de Réalité. Si une API réelle empêche ces invariants, remonter le point avant de réduire une garantie ou de changer une frontière ; aucun arbitrage structurel bloquant identifié dans ce cadrage.
