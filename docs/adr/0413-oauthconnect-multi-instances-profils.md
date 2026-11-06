# ADR-0413 : Un service OAuthConnect héberge des instances configurées OAuth ou OIDC, sans déplacer les comptes IAM

- Statut : Accepted
- Réalité : Partial
- Date : 2026-11-06
- Décideurs : utilisateur, acceptation explicite du 2026-11-06 transmise par le parent
- Jalon concerné : IAM-IdP / OAuthConnect, refonte TDD (hors Apparatus P0–P6)
- SuperSède : **partiellement**, depuis le 2026-11-06, 0408 (topologie), 0411 (onboarding) et 0407 (interface d'authentification), dans le périmètre délimité ci-dessous
- SuperSédée par : —
- Contrat d'implémentation : [oauthconnect-multi-instances.md](../contracts/oauthconnect-multi-instances.md)

`Accepted` ratifie la cible, pas sa livraison. La ratification documentaire était initialement `Unimplemented` ; la première tranche de contrat ci-dessous justifie désormais `Partial`, sans annoncer le service complet. L'utilisateur a explicitement choisi le 2026-11-06 « Valider v2 et nonce (Recommandé) » et « Email primaire seul » : API authorize+complete, nonce IAM durable indépendant, tokens+profil minimal et delta GitHub sans fallback public sont ratifiés, sans claims riches ni actions déléguées.

### Réalité vérifiée — première tranche du contrat v2 (2026-11-06)

- `idp-connect-contract` expose les DTO/méthodes et le routeur `authorize`/`complete` v2, HMAC par instance sur PATH complet, contrôle nonce/PKCE et erreurs génériques. Les defaults legacy refusent sans composer v1. Le default du crate reste sans Axum ; `base64` existant est optionnel avec `server`.
- TDD Windows : 16 tests de frontière v2 exécutés en RED sur le socle (5 témoins verts, 11 assertions HTTP404 en échec), puis **16/16 verts** sans modifier leurs attentes ; **15/15 tests lib existants verts**, check sans feature serveur, fmt et Clippy strict all-targets verts. Revue statique de sécurité bornée sans finding ; les clients sortants sont des fakes de test, pas des fournisseurs réels.
- **Restent non implémentés dans cette tranche** : service OAuthConnect/configuration/profils OAuth et OIDC, validation effective d'un ID token externe, génération/persistance/consommation du nonce IAM, adaptation des clients/parcours IAM, déploiement/cutover et retrait des services historiques. Aucun E2E, Docker ou changement de runtime n'est exécuté par ces preuves. Les claims riches/actions restent différés.

## Contexte

L'utilisateur demande un service RustyCog unique, plusieurs entrées de configuration au boot, deux profils seulement : OAuth adaptable (GitHub) et OIDC standard (GitLab). Chaque claim OAuth normalisé est décrit par un objet API GET JSON + JSONPath, sans scripts, transformations configurables ni chaînage HTTP. Cette orientation contredit le choix de déploiements indépendants de [0408](0408-connecteurs-idp-services-http.md) et son onboarding repris par [0411](0411-idp-provider-slug-registry-fail-closed.md).

Les connecteurs sont **déjà** des services RustyCog minces : configuration typée, setup, AppState/RouteBuilder et handlers S2S communs. Il ne s'agit ni d'introduire RustyCog, ni d'ajouter DB/events/OpenFGA/nest pour suivre un gabarit métier.

Le contrat existant sépare `exchange_code_with_pkce -> ProviderTokens` puis `user_profile(access_token)`. Il ne transporte ni ID token ni contexte nonce OIDC. Les consommateurs réels sont notamment `ConsumedOAuthTransaction::exchange_code`, `OAuthService::process_callback` et `LinkProviderUseCaseImpl::fetch_provider_profile`. Faire seulement passer GitLab de `/api/v4/user` à un endpoint OIDC ne valide pas une identité OIDC.

La note publiée le 2026-11-06 dans [0407](0407-contrat-authn-federee-vendor-neutral.md#limitation-connue--claims-additionnels-et-usages-délégués-2026-11-06) diffère explicitement les claims riches et usages délégués. Elle n'interdit pas les données minimales de validation OIDC et n'avait ratifié aucun regroupement des opérations ; le regroupement minimal v2 est désormais ratifié par la présente ADR, sans lever ce report.

## Décision

1. **Un authenticator HTTP stateless `OAuthConnect`**, hors processus IAM, charge une liste non vide d'instances immuables au boot. `setup` est l'unique composition root. Une instance est une **autorité logique** : slug stable, endpoints/issuer, client OAuth et politique de confiance. Ce n'est ni un Pod, ni un cluster. Plusieurs replicas peuvent servir la même autorité. `github` et `gitlab` restent les clés IAM ; `oauth` et `oidc` sont des profils, jamais leurs remplacements.
2. **Deux profils fermés** : `oauth` exécute les mappings `{ api, json_path }` des seuls champs d'identité actuels ; `oidc` lit les noms standard fixes et vérifie l'ID token avec l'issuer épinglé. GitLab utilise OIDC ; GitHub reste OAuth + API user/emails. `jsonpath-rust` **1.0.12**, maintenu et publié le 2026-10-13, est la dépendance retenue pour JSONPath ; pas de moteur maison ni de framework auth supplémentaire. Les normalisations et bornes sont dans le contrat, pas dans un catalogue de branches GitHub/GitLab.
3. **API S2S v2 à deux opérations**, sous `/oauth-connect/{instance}/v2/authorize` et `/oauth-connect/{instance}/v2/complete`. `complete` échange le code, valide l'identité puis renvoie **uniquement** `{ tokens: ProviderTokens, profile: ProviderUserProfile }`. Aucun ID token, JSON brut, claim additionnel ou handle de contexte n'est exposé à IAM. Cette composition est une évolution minimale d'authentification, **pas** l'API future des claims/actions. La crate `idp-connect-contract` garde son default sans Axum ; son option `server` porte le récepteur commun. Pas de tentative v1 automatique après échec v2.
4. **IAM possède le contexte transactionnel**, Connect la validation OIDC. Pour v2, IAM génère un nonce indépendant aléatoire, lié à sa transaction persistante, distinct du state, du nonce de state et du cookie navigateur. OIDC l'envoie à l'autorité et vérifie sa présence/égalité dans l'ID token. Le verifier PKCE ne quitte IAM qu'après consommation writer commitée, avec le nonce attendu ; Connect ne maintient aucun cache de corrélation entre authorize et complete. OAuth ne transmet pas de paramètre nonce OIDC au vendor. Le choix d'utiliser un nonce OIDC est ici retenu comme défense explicite, **pas présenté comme une obligation de tout Authorization Code flow**.
5. **Confiance et métier conservés** : HMAC par instance couvrant METHOD/PATH complet/TIMESTAMP/BODY, fenêtre 30 s, HTTPS vérifié par défaut, exceptions locales explicites, allowlists exactes, PKCE sans downgrade, loader strict et secrets de démarrage OpenBao/direct explicites. IAM garde comptes UUID, transactions, login/link/relink et stockage des tokens. En LOGIN, email requis **avant tout lookup**, puis priorité au lien provider+ID, rapprochement email seulement si le provider le déclare vérifié, sinon règles actuelles de compte incomplet/refus. LINK authentifié accepte email absent et refuse l'identité liée ailleurs. Erreurs via les mécanismes RustyCog et conversions du contrat existants, pas de nouveau protocole d'erreurs.

### Portée exacte de la supersession partielle — effective le 2026-11-06

- **0408** : remplacer Décision 1–2 (deux services / N+1 = nouveau service), la partie « config + service » de 3, les ports/préfixes distincts de 6 et l'alternative rejetant le service unique. Conserver frontière HTTP hors IAM, registry typé, exceptions stateless de 5, absence de nest de 4 et report de 7. Un nouveau fournisseur compatible devient une entrée de configuration ; un besoin hors des deux profils nécessite un nouveau cadrage.
- **0411** : remplacer Décision 5 et les conséquences/slices documentaires qui prescrivent copie du gabarit + nouveau service, ainsi que l'exclusion du service multi-instances. Conserver slug typé, charset/case-fold, catalogue boot, fail-closed, 400/422, absence de hot-load et règles de redémarrage des configurations partagées.
- **0407** : préciser/faire évoluer seulement l'interface séquentielle d'authentification (§§2–5) pour les nouveaux consommateurs v2 ; conserver frontière vendor-neutral, subject mapping et report des claims/actions. Le protocole v1 historique reste documenté, pas réécrit comme livré en v2.
- **0409/0410/0412** : ni callback déplacé, ni baisse de confiance, ni changement des URLs navigateur. Le contexte nonce complète la transaction existante ; il ne remplace aucune condition de consommation. Les en-têtes et le corps historiques de ces ADR ne sont pas modifiés par cette ratification.

Les pointeurs de supersession **partielle** et le périmètre daté sont portés dans les ADR vivantes concernées. Leurs statuts Accepted et Réalités Implemented historiques restent inchangés ; la cible nouvelle est Accepted / Unimplemented. Pas de passage global de 0408/0411 à « obsolète », ni d'annonce de livraison de la refonte.

## Conséquences

- Moins de duplication configuration/transport/setup ; ajout d'instances compatibles sans nouveau binaire ni recompilation du domaine IAM. En contrepartie, panne/compromission du processus unifié touche toutes les instances et leurs secrets : l'isolation par routes/HMAC n'est pas une isolation de processus.
- Évolution explicite de la crate contrat et des deux chemins IAM LOGIN/LINK/RELINK ; petit stockage transactionnel du nonce, non des claims. Déploiement récepteur avant consommateur, drainage des transactions v1 et rollback documenté. Pas de migration des clés `provider_tokens` ni des UUID.
- La configuration GitHub minimale sélectionne l'email primaire de `/user/emails`. L'ancien fallback vers `/user.email` quand aucune entrée primaire n'existe nécessite une jointure entre réponses, exclue par la borne utilisateur. **Différence explicitement approuvée le 2026-11-06** : zéro email primaire devient email absent ; erreur HTTP n'est plus transformée silencieusement en liste vide. IAM ne change pas sa règle de LOGIN. Pas de script caché pour reproduire ce fallback.
- La bascule GitLab est bloquée jusqu'à preuve, par l'intervenant E2E, que le `sub` OIDC exact correspond à l'ancien `id.to_string()`, avec scopes/claims effectifs et UUID/liens conservés. La documentation GitLab décrit `sub` comme ID utilisateur ; elle ne prouve pas notre déploiement.
- Contradiction observée : `docker-compose.yml:167-168,340,362` décrit encore IAM→Connect en HTTP interne, alors que 0409 exige HTTPS hors exception locale. La migration doit rendre le mode explicite et privilégier HTTPS vérifié ; cette ratification ne qualifie pas l'existant comme HTTPS de bout en bout.
- [0505](0505-startup-secrets-sdk-openbao.md) reste **Proposed / Implemented**. Réemployer l'acquis GitHub ne l'accepte pas et n'autorise ni rotation, ni modification des politiques OpenBao par défaut.

## Alternatives écartées

- Garder N services : contredit l'orientation utilisateur ; meilleure isolation de processus, coût de duplication conservé.
- Vendors in-process IAM / adoption directe de lucid-auth : annule la frontière, ou ajoute un framework et des adaptations déjà identifiées sans nécessité.
- Conserver token puis profil en transportant ID token + nonce par IAM : possible, mais élargit les DTO/états secrets et oblige un second hop pour la même identification. Le regroupement v2 minimal a été explicitement ratifié le 2026-11-06.
- Cache `access_token -> id_token/nonce` dans Connect : restart/répliques problématiques, réintroduit une corrélation process-local.
- OIDC Code+PKCE sans nonce : permis par le protocole ; non retenu ici au profit d'une liaison explicite au contexte writer IAM. Coût assumé : champ transactionnel temporaire et migration additive.
- Scripts, chaînes d'API, claims riches/actions : hors besoin ratifié de ce lot.

## Validation et non décidé ici

Le [contrat actif](../contracts/oauthconnect-multi-instances.md) fixe les entrées RED/GREEN, les modules et les gates de migration ; le plan QA produit indépendamment a été réconcilié avec lui. Après intégration documentaire par le parent, les unités/IT autorisées peuvent commencer en TDD sans attendre les E2E externes. Les preuves E2E/cutover restent requises pour la bascule et le retrait des anciens services. Aucune compilation, IT ou E2E exécutée dans ce cadrage. Pas de garantie HA/production ni d'adoption de 0505. E2E GitHub/GitLab : autre intervenant, obligations de handoff seulement.

L'accord humain sur cette ADR, le regroupement v2 avec nonce IAM durable et la différence de fallback GitHub est acquis le 2026-11-06. Claims riches, actions déléguées, nouveaux profils, hot-load, nest, gestion de comptes sans email et nouveaux mécanismes d'erreur restent hors scope.

## Références vérifiées

- Base : `b39c6c4dcc6de9b7f123c00c9bd168aad08eca74` ; brief utilisateur du 2026-11-06 ; aucune preuve d'implémentation de la refonte.
- Ratification : choix utilisateur explicites « Valider v2 et nonce (Recommandé) » et « Email primaire seul », le 2026-11-06, transmis par le parent ; trace de continuité `.cursor/handoffs/20261106-oauthconnect-tdd.md`. Proposition documentaire intégrée au commit `0a51206b1f9f4a999de7fc4812e947aee0445531` avant cette actualisation de statut.
- ADR : 0100–0103, 0200–0202, 0400, 0407–0412, 0505. Analogue : résolution typée des secrets au setup de 0505, distincte du loader déclaratif.
- Code : `crates/idp-connect-contract/src/{client,dto,server}.rs` ; `services/IAMRusty/domain/src/entity/oauth_transaction.rs`, `domain/src/service/oauth_service.rs`, `application/src/usecase/link_provider.rs`, `infra/src/auth/http_connector.rs` ; `services/{GitHubConnect,GitLabConnect}/infra/src/{github,gitlab}.rs` ; configuration/setup/http des deux Connect.
- Handbook : `docs/services/{github-connect,gitlab-connect}.md` ; conception wiki antérieure : `obsidian/AI FOR ALL/projects/iamrusty/concepts/oauth-provider-linking.md` (QMD `#294f8d`, notamment ancien onboarding §Open Questions). Wiki = conception, pas ratification.
- Sources primaires consultées le 2026-11-06 : [OIDC Core, ID token validation](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation), [OAuth Security BCP](https://www.rfc-editor.org/rfc/rfc9700.html#section-4.5.3), [GitLab OIDC](https://docs.gitlab.com/integration/openid_connect_provider/), [jsonpath-rust 1.0.12](https://docs.rs/jsonpath-rust/1.0.12/jsonpath_rust/), [publication registry](https://crates.io/api/v1/crates/jsonpath-rust), [openidconnect 4.0.0](https://docs.rs/openidconnect/4.0.0/openidconnect/).
