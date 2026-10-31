# Parité des scénarios authentification — k6 Windows

Inventaire intégré le 2026-10-31, base05941d1/SDK02e5. Qualification historique : application/image/navigateur7b8c1f95, validation complémentaire8606173. Aucun scénario k6 OAuth exécuté ici. Cette matrice trace les garanties et les limites, pas un PASS de portage.

## Portée et source des preuves

État qualifié `docs/contracts/20261029-auth-security-qualified-state.md` : Accepted/Implemented pour0412 localement, pas production/HA/edgeTLS/GitLab réel. Comparaison lead8606173..HEAD : seuls quatre fichiers Manifesto hors parcours changés ; IAM/connecteurs/crates/lock, matchers Envoy/profil oauth-proof et SDK02e5 inchangés.17IT0412 réutilisables sans réexécution par principe, à réévaluer si le delta futur invalide leur portée.

Le dernier run utilisait un compte existant complet : ledger `.cursor/handoffs/20261028-auth-security-closure.md:3742–3745` recoupé par le lead. La branche inscription du pilote n'est donc pas prouvée exécutée dans ce run. Le portage doit exiger202, terminer l'inscription puis effectuer un nouveau login. Aucun resetDB ni compte déjà complet ne remplace cette exigence.

## Matrice ancien oracle vers destination

1. **P1 départ login** → Chromium k6 ; parent pour causalité. GET `/iam/api/auth/gitlab/login` sans Authorization, redirection réelle vers IdP, nonce créé ; regex publique ext-authz off, pas IAM direct. Sources `ops/tests/oauth-browser-local-full.js:18–49,54–60` et contrat0412 exact-route:98–142,189–206. Historique exécuté, pas k6. Compter chaque départ nécessaire ; observedAuthorize>=2 ancien n'est pas une cardinalité exacte future.

2. **Inscription puis nouvelle connexion** → Chromium k6. Premier callback202/registration_required/token présent ; POST same-origin complete-registration sans redirection,200/201, puis nouvelle navigation login.202 n'est pas P2. Source pilote:62–77. Compte neuf isolé/run ; ne pas rendre l'inscription facultative en acceptant directement200.

3. **P2 login200 et autorité** → Chromium k6 ; cryptographie aux UT/IT conservées. Callback exact `/iam/api/auth/gitlab/callback`,200/operation=login, access_token et email exact. Observer iss=http://127.0.0.1:8080/iam et aud=aiforall avec API k6, pas Buffer ni validation locale de trust. Le même token doit être accepté réellement par Envoy au START P3. Sources pilote:76–81, contrat0412 autorité§§2/4, état qualifié:28–33.

4. **Nonce et cookie jar réel** → Chromium k6. Même contexte ; cookie iam-oauth-browser host-only127.0.0.1, Path=/iam, HttpOnly, SameSite=Lax, Max-Age600, sans Secure/Domain pour ce seul profil HTTP. Issu de login303, jamais forgé ou remplacé par client API. Sources pilote:43–49,54,83–86. Une unit de policy ne prouve pas le moteur.

5. **P3 réassociation** → Chromium k6, parent pour ext-authz. GET relink-start avec token P2 et credentials same-origin, redirection implicite interdite,200/auth_url sur `http://localhost:18081/oauth/authorize`. Navigation URL inchangée sans Bearer ; callback exact `/iam/api/auth/gitlab/relink-callback`,200, mêmes user.id/email, emails tableau. START ET callback achevés ; URL seule insuffisante. Sources pilote:87–101 et contrat exact-route:189–206.

6. **N1 link sans JWT** → Chromium k6 dans le même contexte, parent pour discrimination. GET `/iam/api/auth/gitlab/link`,403 exact, pas redirection/Set-Cookie/transaction/échange fournisseur/dispatch IAM supplémentaire. Sources pilote:102–114 et contrat exact-route:189–206.401/403 toléré par ancien pilote n'élargit pas le403 qualifié.

7. **N2 relink-start sans JWT** → même affectation et sources. GET `/iam/api/auth/gitlab/relink-start`,403 exact, aucune création de transaction/Set-Cookie/échange/dispatch supplémentaire.

8. **N3 alias sans JWT** → même affectation ; GET `/iam/api/auth/gitlab/relink/callback`,403 exact, hors regex publique et exemption exacte, pas consommation/Set-Cookie/échange fournisseur. Source supplémentaire contrat exact-route:119–122. Pour N1–N3, absence d'effetsDB historiquement inférée du refus pré-IAM, pas mesureSQL : conserver cette limite, pas instrumentation nouvelle.

9. **Credentials et confidentialité** → Chromium k6 et parent pour sorties. Zéro Authorization sur login/callbacks, zéro Authorization/cookie vers autorisation IdP, zéro Set-Cookie callbacks/négatifs. Aucun JWT/cookie/code/state/clé/URL sensible, trace/HAR ou exception brute publié. Sources pilote:18–49,112–123. Booléens d'absence ne remplacent pas compteurs des cas achevés.

10. **Fournisseur effectif/PKCE/même run** → parent corrélé à Chromium. Vrai GitLabConnect, fournisseur synthétique existant réellement appelé, authorize/token/profile et PKCE S256 ; code lié client/redirect/challenge et usage unique/TTL ; run opaque, sans secrets publiés. Sources pilote:115–119, contrat local-full§3, ledger:3441,3757–3764. Résultat pilote PENDING_GATEWAY_IAM_IDP_CORRELATION, pas PASS autonome. Ancien compte existant : quatre compteurs authorize/token/profile/pkce_validated chacun+2, non transposables automatiquement au parcours inscription+nouveau login+relink.

11. **Configuration et causalité Envoy** → parent. Premier match gagne ; login/callback regex publique, relink-callback exact, N1–N3 préfixe authentifié. Slug configuré, même Envoy/incarnation/epoch et identité de configuration chargée. Historique : IAM12→16(+4), ext-authz3→7(+4), IAM5xx1→1, erreurs SAN/connexion/timeout mesurées+0, autres services rq+0. Négatifs sans dispatch inférés des quatre positifs/+4 IAM et trois403, pas capture isolée/SQL. Liaison Podargs/mount/CM UID/RV/hash/no_dynamic et effets réels, pas config_dump byte-exact. Quatre compteurs5xx MISSING et statuts Admin INCONCLUSIVE conservés. Sources état qualifié:36–38,59–66 et ledger:3757–3764 recoupé.

12. **Bon/mauvais SAN même CA/bon restauré** → mutation/ACK/restauration parent, témoins HTTP k6 API seulement. Contrôle existant référencé `.tmp/auth-envoy-same-ca-san-20261029.py` ; wrapper exécuté SHA2567da5a219f23ceead048b21a2df060dc8c94906d578c25f0e529649a9eadf0b27, helper9b0da96b7e0db6ace93628d4d55f059d997231384c44737ff2a39ea4f1a64605. Sources ledger:1747–1757,3343–3348,3400–3412 rapportées par l'enquête. Contenu actuel et bail non relus/admis : aucune réutilisation automatique.

   Gardes historiques : owner-chain Envoy/incarnation, Deployment IAM/Secret UID exacts ; patch JSON privé via `--patch-file` testant UID/données attendues (pas stdin, vérifié dans l'enveloppe actuelle:169–184), seule paire IAM remplacée par leaf Hive connu même CA ; restauration même UID/bytes sous verrou exclusif et rollout, ACK patch/restauration/rollout et Pod UID remplacés tracés. Pas GET Secret, reset, installation ou restart de nœud.

   Bon303/handshake2/SAN0/verifyerror0 ; mauvais503/handshake2/SAN1/verifyerror0 ; restauré303/handshake3/SAN1/verifyerror0. Discriminant `cluster.iam.ssl.fail_verify_san`+1, erreur CA inchangée, même Envoy/epoch0.503 seul insuffisant ; mauvais leaf accepté=FAIL, discriminant absent=INCONCLUSIVE, restauration non vérifiée=blocage. Bail historique6b7 et artefacts privés restent historiques, sans autorité présente.

13. **Transactions persistantes/cleanup0412** → IT Windows conservées. oauth_browser_transactions9, auth_transactions_postgres6, oauth_cleanup_lifecycle2 :17PASS/0failed/0ignored/0filtered historiques. Rejeu inter-réplicas, cookie absent/faux, bindings state/provider/intention/cible/redirect/expiration, consommation avant échange, PKCE/échec, purge backlog/arrêt supervisé. Sources contrat exact-route:176–187, état qualifié:39–45. Pas E2E artificiels, gates conditionnelles seulement aux changements pertinents.

14. **Crypto/principal/remote** → preuves historiques au niveau d'origine. Domaine19/vraiRS2563 ; Hive67 ba21/SDK74e8 ; IAM290PASS/1ignored00c6cf2/SDK02e5 ; SDK7unitaires ledger/discovery (enfant Windows réel non exécuté) ; remote config29/protocole19/TLS privé2. Sources état qualifié:43–45,76–88. Pas nouveaux tests du portage ni qualification HSM/cloud/production. ADR0314/remote conservent leurs niveaux. Manifesto complète échouée LNK1102/E0463, jamais PASS ; HA/deux répliques/projections complètes hors portée.

## Décompte statique proposé du nouveau parcours

À confirmer avant run contre scénario final et configuration effectivement chargée, non preuve d'exécution :

- Deux départs login, deux callbacks login (202 puis200), un complete-registration, un START relink et un callback relink : **sept appels applicatifs IAM positifs** ; trois négatifs sans dispatch supplémentaire.
- Deux autorisations login et une relink : **trois échanges IdP**, soit chacun des quatre compteurs authorize/token/profile/pkce_validated attendu+3.
- Ext-authz : START authentifié et trois refus, attendu **+4**, si exemptions login/callback/complete-registration et relink-callback confirmées. Complete-registration publique : `ops/deploy/mesh/envoy.yaml:88–94`.

Pas de trafic concurrent, retries ou requêtes implicites non comptés. Les témoins certificat ont une phase/baseline distincte pour ne pas brouiller les comptes OAuth. Ces nombres planifiés ne remplacent ni l'ancien+2IdP/+4IAM ni la mesure du futur run.

## Préconditions restantes

Cible non admise : zéro listener18080/18081 au contrôle lead, pas réveil/déploiement/tunnel implicite. Contrat mission2 intégré/ratifié avant code ; commandes exactes parent/inspect/run avant exécution. Ancien local-private-parent.ps1 WSL/Linux exclu. Enveloppe certificat à relire et raccorder minimalement via mécanismes existants : absent=opération non qualifiable, pas backend/fixture/framework nouveau. Cas attendus achevés, checks/thresholds fermés, compteurs non nuls, résumé courant et restauration/cleanup même sur setup échoué restent à démontrer.

Enquête : candidat consolidé result_mvwjcrh0_000b_hq8ghb, session ses_e6d83f632ffegV3ASIvmAYhMr3, lecture seule sans fichier/test/HTTP/runtime. Lead a recoupé l'usage du compte existant et les deltas au ledger:3742–3764 ; autres références attribuées à l'enquête et au canon, sans prétendre relire les artefacts privés.
