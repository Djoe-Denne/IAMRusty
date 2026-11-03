# GitHub : connexion fédérée aujourd'hui, actions déléguées plus tard

## Périmètre actuel — décision utilisateur du 3 novembre 2026

L'utilisateur se connecte à AIForAll via GitHub. Il ne donne pas, par ce login,
mandat à nos services de modifier son profil, ses dépôts ou ses organisations.
Cette note précise le moindre privilège du connector existant ; elle ne crée
ni nouvelle architecture de délégation ni acceptation d'une ADR Proposed.

Canon applicable : [ADR0407](../adr/0407-contrat-authn-federee-vendor-neutral.md),
[ADR0409](../adr/0409-confiance-callback-oauth-idp-connect.md) et
[ADR0412](../adr/0412-oauth-transaction-persistante-liee-navigateur.md), Accepted.
IAM garde le compte plateforme, callback, liaison navigateur et linking ;
GitHubConnect effectue les échanges vendor et traduit le profil.

### Données et scope minimal

`GitHubConnectClient::user_profile` lit `GET /user` puis `GET /user/emails` :

- `id` GitHub stable, converti en `ProviderUserProfile.id` ; la clé fédérée reste
  `(provider_id, provider_user_id)`, jamais le login ou l'email mutable ;
- login et avatar publics ;
- email principal et indicateur `verified`, avec le fallback public existant.

Le scope demandé par l'**OAuth App GitHub** est **`user:email` seulement**.
GitHub permet la lecture du profil public sans scope additionnel ;
`user:email` permet de lire les emails, même privés. `read:user` n'est donc pas
nécessaire pour les champs publics utilisés ici. Le précédent scope `user`
est trop large : il inclut des écritures du profil et `user:follow`.
Aucun scope de dépôt, d'organisation ou d'action n'est demandé par anticipation.

Cette règle porte sur la **requête de consentement** du connector, pas sur une
révocation automatique des anciens droits. Un token déjà émis ou un consentement
historique peut conserver des permissions plus larges. La validation vendor doit
contrôler le consentement effectif avec un grant neuf ou une réautorisation
explicitement préparée ; ne pas révoquer les accès existants silencieusement.
L'absence d'email utilisable n'autorise ni scope plus large automatique, ni email
inventé, ni contournement des contrôles de vérification IAM.

### Terminologie à conserver

- **Authentification** : établir qui est l'utilisateur pour ouvrir sa session
  AIForAll. Le parcours courant est une **connexion fédérée via OAuth 2.0 et
  l'API de profil GitHub** ; ce n'est pas une intégration OIDC.
- **OAuth 2.0** : cadre d'autorisation d'accès à des ressources. Ici il permet
  l'accès au profil nécessaire au login ; ce n'est pas, à lui seul, un protocole
  standardisé d'authentification comme OpenID Connect.
- **OpenID Connect (OIDC)** : couche d'identité/authentification sur OAuth 2.0,
  notamment avec un ID Token. Ne pas appeler le token d'accès GitHub un ID Token.
- **SAML 2.0** : standard de fédération à assertions XML concernant
  authentification, attributs et décisions d'autorisation, utilisé notamment en
  SSO. Ce n'est ni un scope OAuth, ni le flux GitHub implémenté ici.
- **Autorisation déléguée d'actions GitHub** : consentement donnant à un service
  des droits API définis pour agir au nom de l'utilisateur. Elle est distincte
  du login, du linking d'identité et des autorisations internes AIForAll/OpenFGA.

Une **GitHub App** n'est pas synonyme d'**OAuth App** : elle peut aussi utiliser
un flux OAuth utilisateur, mais ses permissions sont fines et configurées sur
l'App, pas obtenues en ajoutant les scopes classiques ci-dessus. Un token
d'installation agit comme installation, pas comme l'utilisateur connecté.
Le choix futur ne doit pas être déduit du seul mot « OAuth ».

## À faire impérativement avant les actions GitHub par nos services

Ce travail reste **non implémenté / à concevoir**. Il ne suffit pas de remplacer
la constante de scope ou de réutiliser le token acquis au login.

1. **Définir les actions et ressources exactes** : lecture/écriture, dépôts et
   organisations concernés ; choisir avec l'architecte OAuth App utilisateur,
   GitHub App utilisateur ou installation, et les permissions minimales adaptées.
2. **Revoir les scopes ou permissions** pour ces actions ; garder le login
   minimal. Prévoir un consentement supplémentaire explicite et un refus sans
   perte de la simple capacité de connexion. Ne jamais ajouter `repo` par défaut.
3. **Distinguer identité liée et délégation consentie** dans les contrats,
   contrôles d'accès et preuves. Un JWT IAM, un lien GitHub ou un email vérifié
   n'atteste pas que l'utilisateur a autorisé l'action vendor demandée.
4. **Gérer le cycle de vie des credentials** : droits réellement accordés,
   expiration/refresh quand supportés, révocation/déconnexion, stockage sécurisé,
   absence de tokens dans logs/receipts et absence de substitution par un PAT,
   le credential `gh` ou un token d'installation d'une autre autorité.
5. **Tester les refus et les effets** : consentement absent/partiel/révoqué,
   scope insuffisant, dépôt ou compte étranger, expiration et retries sûrs ;
   qualifier l'inverse des mutations autorisées avant le parcours réel.
6. **Faire évoluer les ADR/contrats puis les implémentations concernées** avant
   activation. ADR0407 laisse déjà org-sync/git API hors de son contrat de login ;
   cette liste ne ratifie ni architecture future ni nouvelles permissions.

## Credentials locaux, CI et preuve restante

Le Client ID et Client Secret identifient l'application OAuth ; ce ne sont pas
une session utilisateur prête à l'emploi. Leur stockage dans un `.env` privé ou
un secret GitHub Actions ne réalise pas le consentement navigateur et ne configure
pas automatiquement IAM/Kind/OpenBao. Le HMAC IAM↔Connect est un autre secret.
Aucun enregistrement d'App, accès à un `.env`, création de secret CI ou login
vendor n'est effectué par cette modification.

Le test unitaire
`sign_in_requests_only_email_read_scope_with_or_without_pkce` vérifie le scope
exact dans l'URL et dans le DTO, avec et sans PKCE. Il ne prouve pas le login
GitHub réel. Les E2E local-full restent soumises aux gardes du
[contrat existant](../contracts/local-full-existing-e2e-wiring.md), notamment au
nettoyage Telegraph avant signup et aux 39 oracles mesh / 6 obligations Apparatus.

## Sources

- [GitHub — scopes des OAuth Apps](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps)
- [GitHub — tokens utilisateur des GitHub Apps](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app)
- [OpenID Connect Core 1.0](https://openid.net/specs/openid-connect-core-1_0.html)
- [OASIS — SAML 2.0](https://www.oasis-open.org/standard/saml/)
- QMD `aiforall-wiki` : `projects/iamrusty/decisions/0407-contrat-federe.md`
  et `projects/iamrusty/concepts/oauth-provider-linking.md` retrouvées/lues ;
  la discussion précise OAuth/OIDC/SAML évoquée n'a pas été retrouvée dans
  l'index interrogé. Les notes wiki ne remplacent pas les ADR canoniques.
