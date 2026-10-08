# ADR-0412 : OAuth utilise une transaction persistante liée au navigateur et consommée atomiquement, pour login, link et relink

- Statut : Accepted
- Réalité : Partial
- Date : 2026-10-03
- Décideurs : utilisateur, ratification explicite en conversation le 2026-10-03, transmise par le parent ; écriture après accord, pas Accept autonome de l'agent
- Jalon concerné : IAM-IdP / correctifs AuthN (hors Apparatus P0–P6)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0409](0409-confiance-callback-oauth-idp-connect.md), [0411](0411-idp-provider-slug-registry-fail-closed.md), [0305](0305-account-identity-trust-domain.md)

`Accepted` ratifie la cible ci-dessous. `Réalité : Partial` au 2026-10-04 : transaction writer PostgreSQL, cookie/browser binding, consommation atomique, PKCE, cleanup et contexte partagé sont écrits dans les sources A/B/root. À cet état historique, aucun résultat de compilation root complète, d’IT ou d’E2E de cet ensemble n’était fourni. Actualisation du 2026-10-08 ci-dessous : units + IT IAM vertes attestées par l’utilisateur ; E2E exact-route/Envoy finale toujours restante. Elle complète 0409 §3 : IAM conserve callback/linking/state ; l’amendement relink ci-dessous lève uniquement l’ancienne obligation Bearer au callback transactionnel ratifié.

## Contexte

À la baseline historique `f060d47`, `services/IAMRusty/http/src/oauth_state.rs` fournit `OAuthState`, `OAuthOperation`, `inspect`, `decode`, un TTL de 600 s et `USED_NONCES` process-local. Un state signé ne prouve pas que son présentateur est le navigateur qui a initié l'opération ; un registre mémoire ne constitue pas un anti-rejeu multi-replica. Les constats S6 incluent aussi relink sans state obligatoire. Le risque d'identité de S1 reste distinct : 0305 interdit de transformer le `sub` d'un issuer org en compte humain plateforme.

La baseline historique d'audit est `f060d47` / rustycog `ba69c9e`. Au début de rédaction, HEAD observé : `5348a63`. Les sources courantes réalisent une partie de cette cible ; cette note n’atteste aucune nouvelle validation intégrée. Le dossier wiki antérieur `projects/iamrusty/concepts/oauth-state-and-csrf-protection.md`, signalé périmé dans 0409, n'est pas l'autorité de ce choix.

## Décision

1. **IAM conserve le protocole navigateur.** Le connecteur demeure un authenticator HTTP vendor-neutral (`FederatedOAuthClient`, `AuthorizeRequest`, `TokenRequest`) ; ni compte humain, ni auth-cookie, ni décision de linking dans le connecteur.
2. **Transaction persistante.** Chaque démarrage login/link/relink crée une transaction dans le Postgres writer IAM. Elle lie les empreintes du state et du nonce navigateur, le provider canonique, l'intention, le compte cible éventuel, le redirect exact, l'expiration et le statut de consommation. L'autorisation de link/relink vient du principal plateforme vérifié au démarrage ; le callback ne choisit jamais un autre compte depuis une query ou un `sub` nu.
3. **Cookie transactionnel explicite.** IAM crée un nonce aléatoire de navigateur et ne stocke que son empreinte. Cookie HttpOnly, host-only, SameSite adapté au callback navigateur ; Secure obligatoire hors mode local explicitement autorisé. Ce cookie n'est pas une session d'authentification et ne confère aucun droit de compte. State signé et liaison navigateur sont tous deux obligatoires au callback, y compris relink. Aucun cookie de remplacement n'est créé au callback pour rendre un state acceptable.
4. **Consommation writer atomique.** Une condition unique vérifie transaction pending, empreintes, provider, intention/redirect et expiration, puis la consomme. Un seul replica gagne. La consommation est commitée avant échange du code, linking ou émission de credentials. Un échec ultérieur exige un nouveau démarrage, pas un retry qui rejoue la transaction. Les callbacks d'erreur vendor ne contournent pas la validation/consommation. `USED_NONCES` mémoire n'est plus l'autorité du parcours réseau.
5. **PKCE quand supporté.** IAM génère et conserve le verifier privé dans la transaction, calcule un challenge S256 et le transmet au connecteur ; le verifier est utilisé une fois à l'échange du code. Pas de fallback silencieux vers un échange sans PKCE quand le provider est configuré capable. Aucun secret vendor ne migre dans IAM.
6. **Compatibilité bornée et exception callback relink ratifiée le 2026-10-04.** Les responsabilités 0409/0410 restent. Le callback réel **GET/HEAD `/iam/api/auth/{provider_name}/relink-callback`** ne requiert pas d’access JWT : il est autorisé uniquement par state + cookie liés à la transaction writer. Cette route seule lève l’ancienne consigne callback JWT ; START Link/Relink restent JWT + issuer plateforme. Les allowlists courantes ouvrent uniquement github/gitlab configurés, méthodes et chemins exacts ; aucun alias `/relink/callback`, wildcard IAM ou fallback non-browser. La forme des states login/link reste ; relink a son intention explicite. Ancien callback sans transaction/liaison navigateur refusé.
7. **Durée et confidentialité.** Réutiliser le TTL de state actuel de 600 s ; la transaction n'expire pas plus tard que le state. Pas de state, cookie, code, verifier ou token dans les logs/Debug. Après consommation, les secrets temporaires sont retirés du stockage ; la purge des transactions expirées est bornée. Les détails d'enveloppe de messages et de cookie sont fixés dans le contrat A/B ci-dessous, pas dans un nouveau modèle de comptes humains.

## Conséquences

- Migration d'une table de transactions OAuth et injection unique de son writer dans la composition root IAM.
- A possède HTTP/state/cookie et les DTO connecteurs ; B possède domaine/usecase/storage. Le parent brokerise setup, migrations enregistrées, Cargo et fixtures partagées.
- Une panne du writer interdit de démarrer ou consommer ; pas de fallback mémoire/stateless. Plusieurs replicas et plusieurs onglets partagent la même règle de consommation.
- PKCE ajoute des champs optionnels aux DTO connecteurs ; déployer le support récepteur avant d'activer la capacité côté IAM.
- Les clients qui fabriquaient des callbacks sans démarrage navigateur doivent migrer ; ce durcissement est intentionnel.
- IT testcontainers après intégration complète ; E2E navigateur/Envoy et replay inter-replicas ensuite, sur le contexte explicitement autorisé `kind-aiforall-local-full` (3 nœuds), sans modifier le cluster legacy étranger. Aucun test ni démarrage runtime par cette réconciliation.

## Alternatives rejetées

- State signé seul : contexte intègre, mais pas preuve de navigateur initiateur.
- Set de nonces process-local : rejeu possible sur un autre replica et perte au restart.
- Cookie d'authentification présumé existant : le runtime utilise notamment Bearer ; ne pas inventer une session navigateur.
- Callback permissif sans state pour relink ou clients historiques : maintient la faille.
- OAuth déplacé vers le connecteur : contredit 0409 et ajoute une deuxième surface user-facing.

## Non décidé ici

- Protocole pour clients non-browser, nouvelle UX de login ou de comptes multidomaines.
- Chiffrement field-level des provider tokens (0409), SSO global, session cookie d'authentification.
- Cloud/GKE/Flux, production HA ou modalités Apparatus 0009–0011.

## État source et critères de fermeture — actualisation 2026-10-08

- `services/IAMRusty/http/src/{oauth_browser,security_context,public_routes}.rs` : cookie noncehash, transaction/state obligatoires et projection du contexte Axum. `IamHttpSecurityContext` agrège issuer plateforme, limiter instance-local et contexte OAuth ; aucune injection dans `AppState.extensions` ni DI SDK.
- `services/IAMRusty/infra/src/repository/oauth_transaction_write.rs` et migration IAM : create/consume writer, purge bornée des expirées (consommées ou non), `SKIP LOCKED`. Cleanup application : défaut 60 s / 100, cap 1000, stop watch instance-local ; pas de garantie TTL+60 sous panne/backlog.
- `services/IAMRusty/setup/src/app.rs` : validation navigateur/issuer/limiter pré-DB, contexte OAuth post-DB avec `usecases.oauth.clone()`, writer partagé avec cleanup ; `runtime/monolith/src/` compose les mêmes outputs, préfixe `/iam` une fois et supervise les handles. Source écrite à l’état du 2026-10-04 ; units + IT couvertes par l’attestation utilisateur du 2026-10-08 ci-dessous, E2E finale non prouvée.
- Risques couverts par les suites IAM existantes `oauth_browser_transactions`, `oauth_cleanup_lifecycle`, `auth_transactions_postgres` : replay inter-replicas, cookie absent/faux, state/provider/intention/cible/redirect/expiry, consommation avant échange, PKCE/failure, purge avec backlog/concurrence et arrêt supervisé. **E2E exact-route/Envoy finale explicitement restante** ; OAuth vendor désactivé dans le profil par défaut ne constitue pas cette preuve.
- État historique du 2026-10-04 : root HEAD `5348a63` non committé ; tests/fixtures écrits et revue CORE PASS statique n’étaient pas des tests exécutés. SDK22 PASS ne valide pas IAM.
- **2026-10-08 : suites IAM unitaires + IT vertes sur master**, exécutées et confirmées par Djoé Denne. Il s’agit d’une **attestation utilisateur**, pas d’un artefact CI archivé ni d’un run réalisé par cet agent. Cette confirmation couvre units + IT seulement, **pas l’E2E exact-route/Envoy**. Statut **Accepted** et Réalité **Partial** inchangés jusqu’à cette preuve E2E finale.

## Références

- Canon : 0409 §§1–5 ; 0411 registry/slug ; 0305 principal `(iss, sub)` et linking contrôlé.
- Code existant : `services/IAMRusty/http/src/oauth_state.rs`, `services/IAMRusty/http/src/handlers/auth.rs`, `services/IAMRusty/application/src/usecase/oauth.rs`, `services/IAMRusty/domain/src/service/oauth_service.rs`.
- Contrats existants : `crates/idp-connect-contract/src/{client,dto}.rs`.
- Contrat d'implémentation : `.cursor/review-briefings/20261003-auth-fixes-interface-contract.md`.
- Ratification : message utilisateur du 2026-10-03 validant transaction persistante, cookie noncehash, liaison obligatoire, consommation inter-replicas et PKCE supporté.
- Preuve SOURCE : fichiers et suites ci-dessus, contrat interface §§8–13 et handoffs A/B/root du 2026-10-04. Preuve d’exécution actualisée le 2026-10-08 : units + IT IAM vertes confirmées par l’utilisateur, sans artefact CI archivé. **E2E exact-route/Envoy finale non fournie** ; ne pas promouvoir Implemented avant cette preuve restante.

## Mise à jour 2026-10-04 — migrations aplaties

Il n'existe pas de données en production à préserver. Le schéma IAM, dont `oauth_transactions`, ses contraintes et son index d'expiration, est livré en un seul fichier de migration initiale, `services/IAMRusty/migration/src/m20220101_000001_initial_schema.rs`. Les migrations incrémentales seront réintroduites seulement quand un état persisté devra être préservé. Statut et réalité inchangés ; cette simplification ne valide pas les transactions OAuth en IT/E2E.
